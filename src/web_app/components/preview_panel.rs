use yew::prelude::*;
use crate::web_app::app::{AppState, AppAction};
use crate::domain::SelectionMode;

#[derive(Properties, PartialEq)]
pub struct PreviewPanelProps {
    pub state: UseReducerHandle<AppState>,
}

#[function_component(PreviewPanel)]
pub fn preview_panel(props: &PreviewPanelProps) -> Html {
    
    // We will need a way to detect scroll and detach autoscroll, but for now let's just render the lyrics.
    let is_autoscroll_active = use_state(|| true);

    let on_scroll = {
        let is_autoscroll_active = is_autoscroll_active.clone();
        Callback::from(move |_: Event| {
            // If user scrolled manually, we detach autoscroll.
            // A simple implementation: any scroll event detaches.
            // A real implementation would distinguish programmatic vs user scroll.
            is_autoscroll_active.set(false);
        })
    };

    let resume_autoscroll = {
        let is_autoscroll_active = is_autoscroll_active.clone();
        Callback::from(move |_| {
            is_autoscroll_active.set(true);
        })
    };

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
            <div class="lyrics-preview" onscroll={on_scroll}>
                {
                    if let Some(doc) = &props.state.document {
                        let current_entry = doc.current_entry(props.state.current_time_ms);
                        let current_id = current_entry.map(|e| e.id());
                        
                        doc.entries().iter().map(|entry| {
                            let is_active = current_id == Some(entry.id());
                            let is_empty = entry.is_empty();
                            let time_ms = entry.time_ms();
                            
                            let onclick = {
                                let state = props.state.clone();
                                let id = entry.id();
                                Callback::from(move |_| {
                                    state.dispatch(AppAction::Seek(time_ms));
                                    state.dispatch(AppAction::SelectEntry(id, SelectionMode::Replace));
                                })
                            };

                            let mut classes = classes!("lyric-row");
                            if is_active { classes.push("active"); }
                            if is_empty { classes.push("empty"); }

                            html! {
                                <button class={classes} {onclick}>
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
