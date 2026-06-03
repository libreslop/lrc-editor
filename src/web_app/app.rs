use yew::prelude::*;
use std::rc::Rc;
use crate::domain::{LrcDocument, LrcParser, SelectionState, SelectionMode};
use super::components::source_panel::SourcePanel;
use super::components::preview_panel::PreviewPanel;
use super::components::timeline_panel::TimelinePanel;

pub enum AppAction {
    UpdateSource(String),
    SelectEntry(usize, SelectionMode),
    ClearSelection,
    SelectAll,
    SetTime(u32),
    SetDuration(u32),
    TogglePlay,
    Seek(u32),
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
    });

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
