use std::rc::Rc;
use yew::prelude::*;
use crate::domain::{LrcDocument, LrcParser, SelectionState, SelectionMode, TimeMs};

pub enum AppAction {
    UpdateSource(String),
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
    ShiftBoundary(usize, bool, i32),
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
        }
    }
}

impl Reducible for AppState {
    type Action = AppAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut new_state = (*self).clone();
        match action {
            AppAction::UpdateSource(source) => {
                new_state.source_text = source.clone();
                let parser = LrcParser::new(&source);
                match parser.parse() {
                    Ok(doc) => {
                        new_state.selection.prune(&doc);
                        new_state.document = Some(doc);
                        new_state.parse_error = None;
                    }
                    Err(e) => {
                        new_state.parse_error = Some(e.prefixed_message());
                    }
                }
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
                new_state.current_time_ms = time;
            }
            AppAction::SetDuration(time) => {
                new_state.duration_ms = time;
            }
            AppAction::TogglePlay => {
                new_state.playing = !new_state.playing;
            }
            AppAction::Seek(time) => {
                new_state.last_seek_request = Some(time);
                new_state.current_time_ms = time;
            }
            AppAction::Undo => {
                if new_state.history_index > 0 {
                    new_state.history_index -= 1;
                    let source = new_state.history[new_state.history_index].clone();
                    new_state.source_text = source.clone();
                    if let Ok(doc) = LrcParser::new(&source).parse() {
                        new_state.selection.prune(&doc);
                        new_state.document = Some(doc);
                        new_state.parse_error = None;
                    }
                }
            }
            AppAction::Redo => {
                if new_state.history_index + 1 < new_state.history.len() {
                    new_state.history_index += 1;
                    let source = new_state.history[new_state.history_index].clone();
                    new_state.source_text = source.clone();
                    if let Ok(doc) = LrcParser::new(&source).parse() {
                        new_state.selection.prune(&doc);
                        new_state.document = Some(doc);
                        new_state.parse_error = None;
                    }
                }
            }
            AppAction::SetZoom(zoom) => {
                new_state.zoom_level = zoom.clamp(0.1, 10.0);
            }
            AppAction::SaveHistory(source) => {
                new_state.history.truncate(new_state.history_index + 1);
                new_state.history.push(source);
                new_state.history_index = new_state.history.len() - 1;
            }
            AppAction::DeleteSelected => {
                if !new_state.selection.selected_ids().is_empty() {
                    if let Some(doc) = &new_state.document {
                        let mut text = new_state.source_text.clone();
                        for id in new_state.selection.selected_ids().iter() {
                            if let Some(entry) = doc.entries().iter().find(|e| e.id() == *id) {
                                let tag = format!("[{}]", entry.timestamp());
                                text = text.replacen(&tag, "", 1);
                            }
                        }
                        
                        new_state.source_text = text.clone();
                        if let Ok(new_doc) = LrcParser::new(&text).parse() {
                            new_state.selection.prune(&new_doc);
                            new_state.document = Some(new_doc);
                            new_state.parse_error = None;
                        }
                        new_state.history.truncate(new_state.history_index + 1);
                        new_state.history.push(text);
                        new_state.history_index = new_state.history.len() - 1;
                    }
                }
            }
            AppAction::ShiftSelected(delta_ms) => {
                if !new_state.selection.selected_ids().is_empty() && delta_ms != 0 {
                    if let Some(doc) = &new_state.document {
                        let last_lyric_ms = doc.last_entry_time_ms().unwrap_or(TimeMs(0));
                        let timeline_duration_ms = TimeMs(new_state.duration_ms.as_u32().max(last_lyric_ms.as_u32()) + 10000);
                        let text = crate::web_app::editor::timeline::shift_selected(
                            doc,
                            new_state.selection.selected_ids(),
                            delta_ms,
                            timeline_duration_ms
                        );
                        
                        new_state.source_text = text.clone();
                        if let Ok(new_doc) = LrcParser::new(&text).parse() {
                            new_state.selection.prune(&new_doc);
                            new_state.document = Some(new_doc);
                            new_state.parse_error = None;
                        }
                        new_state.history.truncate(new_state.history_index + 1);
                        new_state.history.push(text);
                        new_state.history_index = new_state.history.len() - 1;
                    }
                }
            }
            AppAction::ShiftBoundary(chunk_id, left_edge, delta_ms) => {
                if delta_ms != 0 {
                    if let Some(doc) = &new_state.document {
                        let last_lyric_ms = doc.last_entry_time_ms().unwrap_or(TimeMs(0));
                        let timeline_duration_ms = TimeMs(new_state.duration_ms.as_u32().max(last_lyric_ms.as_u32()) + 10000);
                        let text = crate::web_app::editor::timeline::shift_boundary(
                            doc,
                            chunk_id,
                            left_edge,
                            delta_ms,
                            timeline_duration_ms
                        );
                        
                        new_state.source_text = text.clone();
                        if let Ok(new_doc) = LrcParser::new(&text).parse() {
                            new_state.selection.prune(&new_doc);
                            new_state.document = Some(new_doc);
                            new_state.parse_error = None;
                        }
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
