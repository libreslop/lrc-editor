use yew::prelude::*;
use std::rc::Rc;
use crate::domain::{LrcDocument, LrcParser, SelectionState, SelectionMode};
use super::components::source_panel::SourcePanel;
use super::components::preview_panel::PreviewPanel;
use super::components::timeline_panel::TimelinePanel;
use wasm_bindgen::JsCast;

pub enum AppAction {
    UpdateSource(String),
    SelectEntry(usize, SelectionMode),
    ClearSelection,
    SelectAll,
    SetTime(u32),
    SetDuration(u32),
    TogglePlay,
    Seek(u32),
    Undo,
    Redo,
    SetZoom(f64),
    SaveHistory(String),
    DeleteSelected,
    ShiftSelected(i32),
}

#[derive(PartialEq)]
pub struct AppState {
    pub source_text: String,
    pub document: Option<LrcDocument>,
    pub selection: SelectionState,
    pub parse_error: Option<String>,
    pub current_time_ms: u32,
    pub duration_ms: u32,
    pub playing: bool,
    pub last_seek_request: Option<u32>,
    pub history: Vec<String>,
    pub history_index: usize,
    pub zoom_level: f64,
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
                if let Some(doc) = &new_state.document {
                    let entry = doc.current_entry(time);
                    new_state.selection.sync_to_active(entry, new_state.selection.suppresses_source_selection());
                }
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
                if let Some(doc) = &new_state.document {
                    let entry = doc.current_entry(time);
                    new_state.selection.sync_to_active(entry, new_state.selection.suppresses_source_selection());
                }
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
                        
                        // Clean up any empty lines with just spaces left behind, optionally.
                        new_state.source_text = text.clone();
                        if let Ok(new_doc) = crate::domain::LrcParser::new(&text).parse() {
                            new_state.selection.prune(&new_doc);
                            new_state.document = Some(new_doc);
                            new_state.parse_error = None;
                        }
                        // Save history
                        new_state.history.truncate(new_state.history_index + 1);
                        new_state.history.push(text);
                        new_state.history_index = new_state.history.len() - 1;
                    }
                }
            }
            AppAction::ShiftSelected(delta_ms) => {
                if !new_state.selection.selected_ids().is_empty() && delta_ms != 0 {
                    if let Some(doc) = &new_state.document {
                        let mut text = new_state.source_text.clone();
                        for id in new_state.selection.selected_ids().iter() {
                            if let Some(entry) = doc.entries().iter().find(|e| e.id() == *id) {
                                let old_tag = format!("[{}]", entry.timestamp());
                                let new_time = (entry.time_ms() as i32 + delta_ms).max(0) as u32;
                                let mins = new_time / 60000;
                                let secs = (new_time % 60000) / 1000;
                                let hund = (new_time % 1000) / 10;
                                let new_tag = format!("[{:02}:{:02}.{:02}]", mins, secs, hund);
                                text = text.replacen(&old_tag, &new_tag, 1);
                            }
                        }
                        
                        new_state.source_text = text.clone();
                        if let Ok(new_doc) = crate::domain::LrcParser::new(&text).parse() {
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

#[function_component(App)]
pub fn app() -> Html {
    let state = use_reducer(|| AppState {
        source_text: String::new(),
        document: None,
        selection: SelectionState::default(),
        parse_error: None,
        current_time_ms: 0,
        duration_ms: 0,
        playing: false,
        last_seek_request: None,
        history: vec![String::new()],
        history_index: 0,
        zoom_level: 1.0,
    });

    {
        let state = state.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if let Some(target) = e.target() {
                    if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                        let tag = el.tag_name();
                        if tag == "TEXTAREA" || tag == "INPUT" {
                            return;
                        }
                    }
                }
                if e.key() == " " {
                    e.prevent_default();
                    state.dispatch(AppAction::TogglePlay);
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
            
            let _ = window.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            
            move || {
                let _ = window.remove_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            }
        });
    }

    html! {
        <div class="editor-shell">
            <div class="top-split">
                <SourcePanel state={state.clone()} />
                <PreviewPanel state={state.clone()} />
            </div>
            <TimelinePanel state={state.clone()} />
        </div>
    }
}
