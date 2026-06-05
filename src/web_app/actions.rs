use std::rc::Rc;
use yew::prelude::*;
use wasm_bindgen::JsCast;
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
    AddChunk(TimeMs, TimeMs),
}

#[derive(PartialEq, Clone)]
pub struct PlaybackState {
    pub current_time_ms: TimeMs,
    pub duration_ms: TimeMs,
    pub playing: bool,
    pub last_seek_request: Option<TimeMs>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            current_time_ms: TimeMs(0),
            duration_ms: TimeMs(0),
            playing: false,
            last_seek_request: None,
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct DocumentState {
    pub source_text: String,
    pub document: Option<LrcDocument>,
    pub parse_error: Option<String>,
    pub next_uid: usize,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self {
            source_text: String::new(),
            document: None,
            parse_error: None,
            next_uid: 1,
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct HistoryState {
    pub history: Vec<String>,
    pub history_index: usize,
}

impl Default for HistoryState {
    fn default() -> Self {
        Self {
            history: vec![String::new()],
            history_index: 0,
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct ViewState {
    pub zoom_level: f64,
    pub selection: SelectionState,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom_level: 0.25,
            selection: SelectionState::default(),
        }
    }
}

#[derive(PartialEq, Clone, Default)]
pub struct ProjectState {
    pub audio_filename: Option<String>,
    pub lrc_filename: Option<String>,
}

#[derive(PartialEq, Clone, Default)]
pub struct AppState {
    pub playback: PlaybackState,
    pub document: DocumentState,
    pub history: HistoryState,
    pub view: ViewState,
    pub project: ProjectState,
}

impl AppState {
    pub fn max_timeline_duration(&self) -> TimeMs {
        let last_lyric_ms = self.document.document.as_ref()
            .and_then(|doc| doc.last_entry_time_ms())
            .unwrap_or(TimeMs(0));
        TimeMs(self.playback.duration_ms.as_u32().max(last_lyric_ms.as_u32()) + 15000)
    }

    pub fn trigger_lrc_export(&self) {
        let text = self.document.source_text.clone();
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let filename = if let Some(audio_name) = &self.project.audio_filename {
                    let base = audio_name.rfind('.').map(|i| &audio_name[..i]).unwrap_or(audio_name);
                    format!("{}.lrc", base)
                } else if let Some(lrc_name) = &self.project.lrc_filename {
                    lrc_name.clone()
                } else {
                    "lyrics.lrc".to_string()
                };

                if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&text.into())) {
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                        if let Ok(a) = document.create_element("a") {
                            if let Ok(a) = a.dyn_into::<web_sys::HtmlAnchorElement>() {
                                a.set_href(&url);
                                a.set_download(&filename);
                                a.click();
                                let _ = web_sys::Url::revoke_object_url(&url);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_document(&mut self, source: String) {
        self.document.source_text = source.clone();
        let parser = LrcParser::new(&source);
        match parser.parse() {
            Ok(mut doc) => {
                crate::domain::document::reconcile_identity(
                    self.document.document.as_ref(),
                    &mut doc,
                    &mut self.document.next_uid,
                );
                self.view.selection.prune(&doc);
                self.document.document = Some(doc);
                self.document.parse_error = None;
            }
            Err(e) => {
                self.document.parse_error = Some(e.prefixed_message());
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
                new_state.project.lrc_filename = Some(name);
            }
            AppAction::SetAudioFilename(name) => {
                new_state.project.audio_filename = Some(name);
            }
            AppAction::SelectEntry(id, mode) => {
                if let Some(doc) = &new_state.document.document {
                    new_state.view.selection.select_entry(doc, id, mode);
                }
            }
            AppAction::ClearSelection => {
                new_state.view.selection = SelectionState::default();
            }
            AppAction::SelectAll => {
                if let Some(doc) = &new_state.document.document {
                    new_state.view.selection.select_all(doc);
                }
            }
            AppAction::SetTime(time) => {
                let max_dur = new_state.max_timeline_duration();
                new_state.playback.current_time_ms = if time.as_u32() > max_dur.as_u32() {
                    max_dur
                } else {
                    time
                };
            }
            AppAction::SetDuration(time) => {
                new_state.playback.duration_ms = time;
                let max_dur = new_state.max_timeline_duration();
                if new_state.playback.current_time_ms.as_u32() > max_dur.as_u32() {
                    new_state.playback.current_time_ms = max_dur;
                }
            }
            AppAction::TogglePlay => {
                new_state.playback.playing = !new_state.playback.playing;
            }
            AppAction::Seek(time) => {
                let max_dur = new_state.max_timeline_duration();
                let clamped_time = if time.as_u32() > max_dur.as_u32() {
                    max_dur
                } else {
                    time
                };
                new_state.playback.last_seek_request = Some(clamped_time);
                new_state.playback.current_time_ms = clamped_time;
            }
            AppAction::Undo => {
                if new_state.history.history_index > 0 {
                    new_state.history.history_index -= 1;
                    let source = new_state.history.history[new_state.history.history_index].clone();
                    new_state.update_document(source);
                }
            }
            AppAction::Redo => {
                if new_state.history.history_index + 1 < new_state.history.history.len() {
                    new_state.history.history_index += 1;
                    let source = new_state.history.history[new_state.history.history_index].clone();
                    new_state.update_document(source);
                }
            }
            AppAction::SetZoom(zoom) => {
                new_state.view.zoom_level = zoom.clamp(0.001, 10.0);
            }
            AppAction::SaveHistory(source) => {
                new_state.history.history.truncate(new_state.history.history_index + 1);
                new_state.history.history.push(source);
                new_state.history.history_index = new_state.history.history.len() - 1;
            }
            AppAction::DeleteSelected => {
                if !new_state.view.selection.selected_ids().is_empty() {
                    if let Some(doc) = &new_state.document.document {
                        let selected_uids = new_state.view.selection.selected_ids().to_vec();
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
                        
                        let new_doc = LrcDocument::new(entries, doc.metadata().to_vec(), doc.line_count());
                        let text = new_doc.to_source_text();
                        
                        new_state.update_document(text.clone());
                        
                        new_state.history.history.truncate(new_state.history.history_index + 1);
                        new_state.history.history.push(text);
                        new_state.history.history_index = new_state.history.history.len() - 1;
                    }
                }
            }
            AppAction::ShiftSelected(delta_ms) => {
                if !new_state.view.selection.selected_ids().is_empty() && delta_ms != 0 {
                    if let Some(doc) = &new_state.document.document {
                        let timeline_duration_ms = new_state.max_timeline_duration();
                        let editor = crate::web_app::editor::timeline::TimelineEditor::new(doc);
                        let text = editor.shift_selected(
                            new_state.view.selection.selected_ids(),
                            delta_ms,
                            timeline_duration_ms
                        );
                        
                        new_state.update_document(text.clone());
                        new_state.history.history.truncate(new_state.history.history_index + 1);
                        new_state.history.history.push(text);
                        new_state.history.history_index = new_state.history.history.len() - 1;
                    }
                }
            }
            AppAction::ShiftBoundary(chunk_id, left_edge, both, delta_ms) => {
                if delta_ms != 0 {
                    if let Some(doc) = &new_state.document.document {
                        let timeline_duration_ms = new_state.max_timeline_duration();
                        let editor = crate::web_app::editor::timeline::TimelineEditor::new(doc);
                        let text = editor.shift_boundary(
                            chunk_id,
                            left_edge,
                            both,
                            delta_ms,
                            timeline_duration_ms
                        );
                        
                        new_state.update_document(text.clone());
                        new_state.history.history.truncate(new_state.history.history_index + 1);
                        new_state.history.history.push(text);
                        new_state.history.history_index = new_state.history.history.len() - 1;
                    }
                }
            }
            AppAction::AddChunk(start, end) => {
                if start < end {
                    let doc_to_use = new_state.document.document.clone().unwrap_or_else(|| {
                        LrcDocument::new(vec![], vec![], 0)
                    });
                    let timeline_duration_ms = new_state.max_timeline_duration();
                    let editor = crate::web_app::editor::timeline::TimelineEditor::new(&doc_to_use);
                    let text = editor.add_chunk(start, end, timeline_duration_ms);
                    
                    let new_uid = new_state.document.next_uid;
                    
                    new_state.update_document(text.clone());
                    new_state.history.history.truncate(new_state.history.history_index + 1);
                    new_state.history.history.push(text);
                    new_state.history.history_index = new_state.history.history.len() - 1;
                    
                    if let Some(new_doc) = &new_state.document.document {
                        let to_select = new_doc.entries().iter()
                            .filter(|e| e.display_text() == "*CHANGE ME*")
                            .min_by_key(|e| {
                                (e.time_ms().as_u32() as i32 - start.as_u32() as i32).abs()
                            })
                            .map(|e| e.uid())
                            .unwrap_or_else(|| {
                                new_doc.entries().iter()
                                    .find(|e| e.uid() >= new_uid)
                                    .map(|e| e.uid())
                                    .unwrap_or(new_uid)
                            });
                        new_state.view.selection.select_entry(new_doc, to_select, SelectionMode::Replace);
                    }
                }
            }
        }
        
        let was_empty = self.document.document.as_ref()
            .map_or(true, |doc| doc.entries().iter().all(|entry| entry.is_empty()));
        let is_empty = new_state.document.document.as_ref()
            .map_or(true, |doc| doc.entries().iter().all(|entry| entry.is_empty()));
        if is_empty && !was_empty {
            new_state.playback.current_time_ms = TimeMs(0);
        }

        Rc::new(new_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_state() -> Rc<AppState> {
        Rc::new(AppState::default())
    }

    #[test]
    fn test_reduce_update_source() {
        let state = mock_state();
        let new_state = state.reduce(AppAction::UpdateSource("[00:01.00]Test".to_string()));
        assert_eq!(new_state.document.source_text, "[00:01.00]Test");
        assert!(new_state.document.document.is_some());
        assert!(new_state.document.parse_error.is_none());
    }

    #[test]
    fn test_reduce_seek() {
        let state = mock_state();
        let new_state = state.reduce(AppAction::Seek(TimeMs(1000)));
        assert_eq!(new_state.playback.current_time_ms, TimeMs(1000));
        assert_eq!(new_state.playback.last_seek_request, Some(TimeMs(1000)));
    }

    #[test]
    fn test_reduce_zoom() {
        let state = mock_state();
        let new_state = state.clone().reduce(AppAction::SetZoom(2.0));
        assert_eq!(new_state.view.zoom_level, 2.0);
        
        let clamped = state.reduce(AppAction::SetZoom(100.0));
        assert_eq!(clamped.view.zoom_level, 10.0);
    }

    #[test]
    fn test_duration_clamping() {
        let state = mock_state();
        // Default duration is 0, so max_timeline_duration is 15000ms (overscroll)
        
        let seek_far = state.clone().reduce(AppAction::Seek(TimeMs(25000)));
        assert_eq!(seek_far.playback.current_time_ms, TimeMs(15000));
        
        let set_dur = state.clone().reduce(AppAction::SetDuration(TimeMs(5000)));
        // max_timeline_duration becomes 20000ms
        let seek_edge = set_dur.reduce(AppAction::Seek(TimeMs(20000)));
        assert_eq!(seek_edge.playback.current_time_ms, TimeMs(20000));
    }
}
