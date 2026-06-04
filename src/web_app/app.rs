use yew::prelude::*;
use crate::domain::{SelectionState, TimeMs};
use super::components::source_panel::SourcePanel;
use super::components::preview_panel::PreviewPanel;
use super::components::timeline_panel::TimelinePanel;
use super::actions::{AppState, AppAction};
use wasm_bindgen::JsCast;

#[function_component(App)]
pub fn app() -> Html {
    let state = use_reducer(|| AppState {
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
