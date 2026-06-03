use yew::prelude::*;
use crate::web_app::app::{AppState, AppAction};
use web_sys::HtmlTextAreaElement;

#[derive(Properties, PartialEq)]
pub struct SourcePanelProps {
    pub state: UseReducerHandle<AppState>,
}

#[function_component(SourcePanel)]
pub fn source_panel(props: &SourcePanelProps) -> Html {
    let state = props.state.clone();
    
    let oninput = {
        let state = state.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(textarea) = e.target_dyn_into::<HtmlTextAreaElement>() {
                state.dispatch(AppAction::UpdateSource(textarea.value()));
            }
        })
    };

    let invalid_class = if state.parse_error.is_some() { "invalid" } else { "" };

    html! {
        <div class="panel source-panel">
            <div class="panel-toolbar">
                <span class="panel-title">{ "Source" }</span>
                <div class="button-group">
                    <button class="icon-button" title="Undo">
                        <svg viewBox="0 0 24 24"><path d="M3 10h10a5 5 0 0 1 5 5v0a5 5 0 0 1-5 5H9"/><polyline points="7 6 3 10 7 14"/></svg>
                    </button>
                    <button class="icon-button" title="Redo">
                        <svg viewBox="0 0 24 24"><path d="M21 10H11a5 5 0 0 0-5 5v0a5 5 0 0 0 5 5h4"/><polyline points="17 6 21 10 17 14"/></svg>
                    </button>
                    <button class="icon-button" title="Copy">
                        <svg viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
                    </button>
                    <button class="icon-button" title="Import LRC">
                        <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                    </button>
                    <button class="icon-button" title="Export LRC">
                        <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                    </button>
                </div>
            </div>
            <textarea 
                class={classes!("source-editor", invalid_class)}
                value={state.source_text.clone()}
                {oninput}
                spellcheck="false"
            />
            if let Some(err) = &state.parse_error {
                <div class="toast">
                    { err }
                </div>
            }
        </div>
    }
}
