use yew::prelude::*;
use crate::domain::TimeMs;
use super::components::source_panel::SourcePanel;
use super::components::preview_panel::PreviewPanel;
use super::components::timeline_panel::TimelinePanel;
use super::actions::{AppState, AppAction};
use wasm_bindgen::JsCast;

#[function_component(App)]
pub fn app() -> Html {
    let state = use_reducer(|| {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        let source_text = storage.as_ref().and_then(|s| s.get_item("lrc_source_text").ok().flatten()).unwrap_or_default();
        let current_time_ms = storage.as_ref().and_then(|s| s.get_item("lrc_current_time").ok().flatten())
            .and_then(|t| t.parse::<u32>().ok())
            .map(TimeMs)
            .unwrap_or(TimeMs(0));
        let audio_filename = storage.as_ref().and_then(|s| s.get_item("lrc_audio_filename").ok().flatten());
        let lrc_filename = storage.as_ref().and_then(|s| s.get_item("lrc_lrc_filename").ok().flatten());

        let mut state = AppState {
            document: crate::web_app::actions::DocumentState {
                source_text: source_text.clone(),
                document: None,
                parse_error: None,
                next_uid: 1,
            },
            playback: crate::web_app::actions::PlaybackState {
                current_time_ms,
                duration_ms: TimeMs(0),
                playing: false,
                last_seek_request: None,
            },
            history: crate::web_app::actions::HistoryState {
                history: vec![source_text.clone()],
                history_index: 0,
            },
            view: crate::web_app::actions::ViewState {
                zoom_level: 0.25,
                selection: crate::domain::SelectionState::default(),
            },
            project: crate::web_app::actions::ProjectState {
                audio_filename,
                lrc_filename,
            },
        };
        
        if !source_text.is_empty() {
            state.update_document(source_text);
        }
        
        state
    });

    let show_help = use_state(|| false);

    // Persistence Effect
    {
        let state = state.clone();
        use_effect_with(
            (state.document.source_text.clone(), state.playback.current_time_ms, state.project.audio_filename.clone(), state.project.lrc_filename.clone()),
            move |(source, time, audio, lrc)| {
                let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
                if let Some(storage) = storage {
                    let _ = storage.set_item("lrc_source_text", source);
                    let _ = storage.set_item("lrc_current_time", &time.as_u32().to_string());
                    if let Some(audio) = audio {
                        let _ = storage.set_item("lrc_audio_filename", audio);
                    } else {
                        let _ = storage.remove_item("lrc_audio_filename");
                    }
                    if let Some(lrc) = lrc {
                        let _ = storage.set_item("lrc_lrc_filename", lrc);
                    } else {
                        let _ = storage.remove_item("lrc_lrc_filename");
                    }
                }
            }
        );
    }

    // Dynamic Title Effect
    {
        let state = state.clone();
        use_effect_with(
            (state.project.audio_filename.clone(), state.project.lrc_filename.clone()),
            move |(audio_name, lrc_name)| {
                let filename = if let Some(audio_name) = audio_name {
                    let base = audio_name.rfind('.').map(|i| &audio_name[..i]).unwrap_or(audio_name);
                    format!("{}.lrc", base)
                } else if let Some(lrc_name) = lrc_name {
                    lrc_name.clone()
                } else {
                    "lyrics.lrc".to_string()
                };

                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        document.set_title(&format!("Editing {} | LRC Editor", filename));
                    }
                }
            }
        );
    }

    let state_ref = yew::use_mut_ref(|| state.clone());
    *state_ref.borrow_mut() = state.clone();

    {
        let state_ref = state_ref.clone();
        let show_help = show_help.clone();
        use_effect_with(show_help.clone(), move |show_help_val| {
            let is_help_open = **show_help_val;
            let window = web_sys::window().unwrap();
            let state_ref = state_ref.clone();
            let show_help = show_help.clone();
            
            let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if is_help_open && e.key() == "Escape" {
                    show_help.set(false);
                    return;
                }

                let current_state = state_ref.borrow().clone();
                let ctrl = e.ctrl_key() || e.meta_key();
                let shift = e.shift_key();
                let key = e.key();

                // 1. Ctrl + S for exporting lrc file (works globally, including in textareas)
                if ctrl && (key == "s" || key == "S") {
                    e.prevent_default();
                    current_state.trigger_lrc_export();
                    return;
                }

                // 2. Ignore other editing/navigation shortcuts when typing in text fields
                if let Some(target) = e.target() {
                    if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                        let tag = el.tag_name();
                        if tag == "TEXTAREA" || tag == "INPUT" {
                            return;
                        }
                    }
                }

                match key.as_str() {
                    " " => {
                        e.prevent_default();
                        current_state.dispatch(AppAction::TogglePlay);
                    }
                    "z" | "Z" if ctrl && !shift => {
                        e.prevent_default();
                        current_state.dispatch(AppAction::Undo);
                    }
                    "y" | "Y" if ctrl => {
                        e.prevent_default();
                        current_state.dispatch(AppAction::Redo);
                    }
                    "z" | "Z" if ctrl && shift => {
                        e.prevent_default();
                        current_state.dispatch(AppAction::Redo);
                    }
                    "a" | "A" if ctrl => {
                        e.prevent_default();
                        current_state.dispatch(AppAction::SelectAll);
                    }
                    "ArrowLeft" | "Left" => {
                        e.prevent_default();
                        let current = current_state.playback.current_time_ms.as_u32();
                        let duration = current_state.playback.duration_ms;
                        let mut edges = vec![0, duration.as_u32()];
                        if let Some(doc) = &current_state.document.document {
                            for chunk in doc.timeline_chunks(duration) {
                                if !chunk.is_empty() {
                                    edges.push(chunk.start_ms().as_u32());
                                    edges.push(chunk.end_ms().as_u32());
                                }
                            }
                        }
                        edges.sort_unstable();
                        edges.dedup();
                        
                        let target_u32 = edges.iter()
                            .copied()
                            .rev()
                            .find(|&e| e < current)
                            .unwrap_or(0);
                        current_state.dispatch(AppAction::Seek(TimeMs(target_u32)));
                    }
                    "ArrowRight" | "Right" => {
                        e.prevent_default();
                        let current = current_state.playback.current_time_ms.as_u32();
                        let duration = current_state.playback.duration_ms;
                        let mut edges = vec![0, duration.as_u32()];
                        if let Some(doc) = &current_state.document.document {
                            for chunk in doc.timeline_chunks(duration) {
                                if !chunk.is_empty() {
                                    edges.push(chunk.start_ms().as_u32());
                                    edges.push(chunk.end_ms().as_u32());
                                }
                            }
                        }
                        edges.sort_unstable();
                        edges.dedup();
                        
                        let target_u32 = edges.iter()
                            .copied()
                            .find(|&e| e > current)
                            .unwrap_or_else(|| duration.as_u32());
                        current_state.dispatch(AppAction::Seek(TimeMs(target_u32)));
                    }
                    _ => {}
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
            
            let _ = window.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            
            let window_inner = window.clone();
            move || {
                let _ = window_inner.remove_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            }
        });
    }

    let on_home_click = Callback::from(|e: MouseEvent| {
        e.prevent_default();
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("https://tools.siri.ws");
        }
    });

    let on_home_aux_click = Callback::from(|e: MouseEvent| {
        if e.button() == 1 { // Middle click
            e.prevent_default();
            if let Some(window) = web_sys::window() {
                let _ = window.open_with_url_and_target("https://tools.siri.ws", "_blank");
            }
        }
    });

    let toggle_help = {
        let show_help = show_help.clone();
        Callback::from(move |_| show_help.set(!*show_help))
    };

    html! {
        <div class="editor-shell">
            <div class="top-split">
                <SourcePanel 
                    state={state.clone()} 
                    on_home_click={on_home_click}
                    on_home_aux_click={on_home_aux_click}
                    on_help_click={toggle_help.clone()}
                />
                <PreviewPanel state={state.clone()} />
            </div>
            <TimelinePanel state={state.clone()} />

            if *show_help {
                <div class="help-overlay" onclick={toggle_help.clone()}>
                    <div class="help-popup" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="help-header">
                            <h2>{"Keyboard Shortcuts"}</h2>
                            <button class="icon-button close-button" onclick={toggle_help.clone()}>
                                <svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                            </button>
                        </div>
                        <div class="keybind-list">
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Play / Pause"}</span>
                                <span class="keybind-key">{"Space"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Undo"}</span>
                                <span class="keybind-key">{"Ctrl + Z"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Redo"}</span>
                                <span class="keybind-key">{"Ctrl + Y / Ctrl + Shift + Z"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Delete Selected"}</span>
                                <span class="keybind-key">{"Delete / Backspace"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Select All"}</span>
                                <span class="keybind-key">{"Ctrl + A"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Deselect All"}</span>
                                <span class="keybind-key">{"Esc"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Add to Selection"}</span>
                                <span class="keybind-key">{"Shift / Ctrl + Click"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Zoom In / Out"}</span>
                                <span class="keybind-key">{"Ctrl + Wheel"}</span>
                            </div>
                            <div class="keybind-item">
                                <span class="keybind-desc">{"Pan Timeline"}</span>
                                <span class="keybind-key">{"Wheel"}</span>
                            </div>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}
