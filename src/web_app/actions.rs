use std::rc::Rc;
use yew::prelude::*;
use crate::domain::{LrcDocument, LrcParser, SelectionState, SelectionMode, TimeMs};

pub enum AppAction {
    UpdateSource(String),
    SetLrcFilename(String),
    SetAudioFilename(String),
    SelectEntry(usize, SelectionMode),
    ClearSelection,
    SelectAll,
    SetTime(TimeMs),
    SetDuration(TimeMs),
    TogglePlay,
    Seek(TimeMs),
    Undo,
    Redo,
    SetZoom(f64),
    SaveHistory(String),
    DeleteSelected,
    ShiftSelected(i32),
    ShiftBoundary(usize, bool, bool, i32), // id, is_left, both, delta
}

#[derive(PartialEq)]
pub struct AppState {
    pub source_text: String,
    pub document: Option<LrcDocument>,
    pub selection: SelectionState,
    pub parse_error: Option<String>,
    pub current_time_ms: TimeMs,
    pub duration_ms: TimeMs,
    pub playing: bool,
    pub last_seek_request: Option<TimeMs>,
    pub history: Vec<String>,
    pub history_index: usize,
    pub zoom_level: f64,
    pub next_uid: usize,
    pub audio_filename: Option<String>,
    pub lrc_filename: Option<String>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            source_text: self.source_text.clone(),
            document: self.document.clone(),
            selection: self.selection.clone(),
            parse_error: self.parse_error.clone(),
            current_time_ms: self.current_time_ms,
            duration_ms: self.duration_ms,
            playing: self.playing,
            last_seek_request: self.last_seek_request,
            history: self.history.clone(),
            history_index: self.history_index,
            zoom_level: self.zoom_level,
            next_uid: self.next_uid,
            audio_filename: self.audio_filename.clone(),
            lrc_filename: self.lrc_filename.clone(),
        }
    }
}

impl AppState {
    pub fn max_timeline_duration(&self) -> TimeMs {
        let last_lyric_ms = self.document.as_ref()
            .and_then(|doc| doc.last_entry_time_ms())
            .unwrap_or(TimeMs(0));
        TimeMs(self.duration_ms.as_u32().max(last_lyric_ms.as_u32()) + 15000)
    }

    pub fn update_document(&mut self, source: String) {
        self.source_text = source.clone();
        let parser = LrcParser::new(&source);
        match parser.parse() {
            Ok(mut doc) => {
                crate::domain::document::reconcile_identity(
                    self.document.as_ref(),
                    &mut doc,
                    &mut self.next_uid,
                );
                self.selection.prune(&doc);
                self.document = Some(doc);
                self.parse_error = None;
            }
            Err(e) => {
                self.parse_error = Some(e.prefixed_message());
            }
        }
    }
}

impl Reducible for AppState {
    type Action = AppAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut new_state = (*self).clone();
        match action {
            AppAction::UpdateSource(source) => {
                new_state.update_document(source);
            }
            AppAction::SetLrcFilename(name) => {
                new_state.lrc_filename = Some(name);
            }
            AppAction::SetAudioFilename(name) => {
                new_state.audio_filename = Some(name);
            }
            AppAction::SelectEntry(id, mode) => {
                if let Some(doc) = &new_state.document {
                    new_state.selection.select_entry(doc, id, mode);
                }
            }
            AppAction::ClearSelection => {
                new_state.selection = SelectionState::default();
            }
            AppAction::SelectAll => {
                if let Some(doc) = &new_state.document {
                    new_state.selection.select_all(doc);
                }
            }
            AppAction::SetTime(time) => {
                let max_dur = new_state.max_timeline_duration();
                new_state.current_time_ms = if time.as_u32() > max_dur.as_u32() {
                    max_dur
                } else {
                    time
                };
            }
            AppAction::SetDuration(time) => {
                new_state.duration_ms = time;
                let max_dur = new_state.max_timeline_duration();
                if new_state.current_time_ms.as_u32() > max_dur.as_u32() {
                    new_state.current_time_ms = max_dur;
                }
            }
            AppAction::TogglePlay => {
                new_state.playing = !new_state.playing;
            }
            AppAction::Seek(time) => {
                let max_dur = new_state.max_timeline_duration();
                let clamped_time = if time.as_u32() > max_dur.as_u32() {
                    max_dur
                } else {
                    time
                };
                new_state.last_seek_request = Some(clamped_time);
                new_state.current_time_ms = clamped_time;
            }
            AppAction::Undo => {
                if new_state.history_index > 0 {
                    new_state.history_index -= 1;
                    let source = new_state.history[new_state.history_index].clone();
                    new_state.update_document(source);
                }
            }
            AppAction::Redo => {
                if new_state.history_index + 1 < new_state.history.len() {
                    new_state.history_index += 1;
                    let source = new_state.history[new_state.history_index].clone();
                    new_state.update_document(source);
                }
            }
            AppAction::SetZoom(zoom) => {
                new_state.zoom_level = zoom.clamp(0.001, 10.0);
            }
            AppAction::SaveHistory(source) => {
                new_state.history.truncate(new_state.history_index + 1);
                new_state.history.push(source);
                new_state.history_index = new_state.history.len() - 1;
            }
            AppAction::DeleteSelected => {
                if !new_state.selection.selected_ids().is_empty() {
                    if let Some(doc) = &new_state.document {
                        let selected_uids = new_state.selection.selected_ids().to_vec();
                        let mut entries = doc.entries().to_vec();
                        
                        for entry in entries.iter_mut() {
                            if selected_uids.contains(&entry.uid()) {
                                entry.text = String::new();
                                entry.display_text = String::new();
                            }
                        }
                        
                        let mut merged_entries: Vec<crate::domain::LyricEntry> = Vec::new();
                        for entry in entries {
                            if let Some(last) = merged_entries.last() {
                                if last.is_empty() && entry.is_empty() {
                                    continue;
                                }
                            }
                            merged_entries.push(entry);
                        }
                        let entries = merged_entries;
                        
                        let mut next_uid = new_state.next_uid;
                        let mut new_doc = LrcDocument::new(entries, doc.metadata().to_vec(), doc.line_count());
                        crate::domain::document::reconcile_identity(
                            Some(doc),
                            &mut new_doc,
                            &mut next_uid,
                        );
                        
                        let text = new_doc.to_source_text();
                        new_state.next_uid = next_uid;
                        new_state.source_text = text.clone();
                        new_state.selection.prune(&new_doc);
                        new_state.document = Some(new_doc);
                        new_state.parse_error = None;
                        
                        new_state.history.truncate(new_state.history_index + 1);
                        new_state.history.push(text);
                        new_state.history_index = new_state.history.len() - 1;
                    }
                }
            }
            AppAction::ShiftSelected(delta_ms) => {
                if !new_state.selection.selected_ids().is_empty() && delta_ms != 0 {
                    if let Some(doc) = &new_state.document {
                        let timeline_duration_ms = new_state.max_timeline_duration();
                        let text = crate::web_app::editor::timeline::shift_selected(
                            doc,
                            new_state.selection.selected_ids(),
                            delta_ms,
                            timeline_duration_ms
                        );
                        
                        new_state.update_document(text.clone());
                        new_state.history.truncate(new_state.history_index + 1);
                        new_state.history.push(text);
                        new_state.history_index = new_state.history.len() - 1;
                    }
                }
            }
            AppAction::ShiftBoundary(chunk_id, left_edge, both, delta_ms) => {
                if delta_ms != 0 {
                    if let Some(doc) = &new_state.document {
                        let timeline_duration_ms = new_state.max_timeline_duration();
                        let text = crate::web_app::editor::timeline::shift_boundary(
                            doc,
                            chunk_id,
                            left_edge,
                            both,
                            delta_ms,
                            timeline_duration_ms
                        );
                        
                        new_state.update_document(text.clone());
                        new_state.history.truncate(new_state.history_index + 1);
                        new_state.history.push(text);
                        new_state.history_index = new_state.history.len() - 1;
                    }
                }
            }
        }
        Rc::new(new_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_state() -> Rc<AppState> {
        Rc::new(AppState {
            source_text: String::new(),
            document: None,
            selection: SelectionState::default(),
            parse_error: None,
            current_time_ms: TimeMs(0),
            duration_ms: TimeMs(0),
            playing: false,
            last_seek_request: None,
            history: vec![String::new()],
            history_index: 0,
            zoom_level: 0.25,
            next_uid: 1,
            audio_filename: None,
            lrc_filename: None,
        })
    }

    #[test]
    fn test_reduce_update_source() {
        let state = mock_state();
        let new_state = state.reduce(AppAction::UpdateSource("[00:01.00]Test".to_string()));
        assert_eq!(new_state.source_text, "[00:01.00]Test");
        assert!(new_state.document.is_some());
        assert!(new_state.parse_error.is_none());
    }

    #[test]
    fn test_reduce_seek() {
        let state = mock_state();
        let new_state = state.reduce(AppAction::Seek(TimeMs(1000)));
        assert_eq!(new_state.current_time_ms, TimeMs(1000));
        assert_eq!(new_state.last_seek_request, Some(TimeMs(1000)));
    }

    #[test]
    fn test_reduce_zoom() {
        let state = mock_state();
        let new_state = state.clone().reduce(AppAction::SetZoom(2.0));
        assert_eq!(new_state.zoom_level, 2.0);
        
        let clamped = state.reduce(AppAction::SetZoom(100.0));
        assert_eq!(clamped.zoom_level, 10.0);
    }

    #[test]
    fn test_duration_clamping() {
        let state = mock_state();
        // Default duration is 0, so max_timeline_duration is 15000ms (overscroll)
        
        let seek_far = state.clone().reduce(AppAction::Seek(TimeMs(25000)));
        assert_eq!(seek_far.current_time_ms, TimeMs(15000));
        
        let set_dur = state.clone().reduce(AppAction::SetDuration(TimeMs(5000)));
        // max_timeline_duration becomes 20000ms
        let seek_edge = set_dur.reduce(AppAction::Seek(TimeMs(20000)));
        assert_eq!(seek_edge.current_time_ms, TimeMs(20000));
    }
}
