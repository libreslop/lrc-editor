use yew::prelude::*;
use wasm_bindgen::JsCast;
use crate::web_app::actions::{AppState, AppAction};

#[derive(Properties, PartialEq)]
pub struct PreviewPanelProps {
    pub state: UseReducerHandle<AppState>,
}

#[function_component(PreviewPanel)]
pub fn preview_panel(props: &PreviewPanelProps) -> Html {
    let preview_ref = use_node_ref();
    
    // We will need a way to detect scroll and detach autoscroll, but for now let's just render the lyrics.
    let is_autoscroll_active = use_state(|| true);

    let disable_autoscroll_wheel = {
        let is_autoscroll_active = is_autoscroll_active.clone();
        let preview_ref = preview_ref.clone();
        Callback::from(move |_: WheelEvent| {
            if let Some(preview) = preview_ref.cast::<web_sys::HtmlElement>() {
                if preview.scroll_height() > preview.client_height() {
                    is_autoscroll_active.set(false);
                }
            }
        })
    };
    let disable_autoscroll_mouse = {
        let is_autoscroll_active = is_autoscroll_active.clone();
        let preview_ref = preview_ref.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(preview) = preview_ref.cast::<web_sys::HtmlElement>() {
                if preview.scroll_height() > preview.client_height() {
                    is_autoscroll_active.set(false);
                }
            }
        })
    };
    let disable_autoscroll_touch = {
        let is_autoscroll_active = is_autoscroll_active.clone();
        let preview_ref = preview_ref.clone();
        Callback::from(move |_: TouchEvent| {
            if let Some(preview) = preview_ref.cast::<web_sys::HtmlElement>() {
                if preview.scroll_height() > preview.client_height() {
                    is_autoscroll_active.set(false);
                }
            }
        })
    };
    let resume_autoscroll = {
        let is_autoscroll_active = is_autoscroll_active.clone();
        Callback::from(move |_| {
            is_autoscroll_active.set(true);
        })
    };
    
    {
        let current_time_ms = props.state.playback.current_time_ms;
        let preview_ref = preview_ref.clone();
        let is_autoscroll_active_val = *is_autoscroll_active;
        use_effect_with((current_time_ms, is_autoscroll_active_val), move |(_, is_active)| {
            if *is_active {
                if let Some(preview) = preview_ref.cast::<web_sys::HtmlElement>() {
                    if let Ok(Some(active)) = preview.query_selector(".active") {
                        if let Ok(active) = active.dyn_into::<web_sys::HtmlElement>() {
                            let target_y = active.offset_top() - preview.client_height() / 2 + active.offset_height() / 2;
                            preview.set_scroll_top(target_y);
                        }
                    }
                }
            }
            || ()
        });
    }

    html! {
        <div class="panel preview-panel">
            <div class="panel-toolbar preview-toolbar">
                <span class="panel-title">{ "Preview" }</span>
                <button 
                    class="resume-button" 
                    hidden={*is_autoscroll_active}
                    onclick={resume_autoscroll}
                >
                    { "Resume autoscroll" }
                </button>
            </div>
            <div class="lyrics-preview" 
                ref={preview_ref}
                onwheel={disable_autoscroll_wheel} 
                onmousedown={disable_autoscroll_mouse}
                ontouchmove={disable_autoscroll_touch} 
            >
                {
                    if let Some(doc) = &props.state.document.document {
                        let current_entry = doc.current_entry(props.state.playback.current_time_ms);
                        let current_id = current_entry.map(|e| e.id());
                        
                        doc.entries().iter().map(|entry| {
                            let is_active = current_id == Some(entry.id());
                            let is_empty = entry.is_empty();
                            let time_ms = entry.time_ms();
                            
                            let onclick = {
                                let state = props.state.clone();
                                Callback::from(move |_| {
                                    state.dispatch(AppAction::Seek(time_ms));
                                })
                            };

                            let onmousedown = Callback::from(|e: MouseEvent| {
                                e.stop_propagation();
                            });

                            let mut classes = classes!("lyric-row");
                            if is_active { classes.push("active"); }
                            if is_empty { classes.push("empty"); }

                            html! {
                                <button class={classes} {onclick} {onmousedown} tabindex="-1">
                                    <span class="lyric-time">{ entry.timestamp() }</span>
                                    <span class="lyric-text">{ 
                                        if is_empty { "[Empty]" } else { entry.text() } 
                                    }</span>
                                </button>
                            }
                        }).collect::<Html>()
                    } else {
                        html! {}
                    }
                }
            </div>
        </div>
    }
}
