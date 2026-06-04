use yew::prelude::*;
use crate::web_app::actions::{AppState, AppAction};
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

    let onchange = {
        let state = state.clone();
        Callback::from(move |e: Event| {
            if let Some(textarea) = e.target_dyn_into::<HtmlTextAreaElement>() {
                state.dispatch(AppAction::SaveHistory(textarea.value()));
            }
        })
    };

    let undo = {
        let state = state.clone();
        Callback::from(move |_| state.dispatch(AppAction::Undo))
    };

    let redo = {
        let state = state.clone();
        Callback::from(move |_| state.dispatch(AppAction::Redo))
    };

    let invalid_class = if state.parse_error.is_some() { "invalid" } else { "" };

    let textarea_ref = use_node_ref();

    let onmouseenter = {
        let textarea_ref = textarea_ref.clone();
        Callback::from(move |_| {
            if let Some(ta) = textarea_ref.cast::<HtmlTextAreaElement>() {
                let _ = ta.focus();
            }
        })
    };

    let onmouseleave = {
        let textarea_ref = textarea_ref.clone();
        Callback::from(move |_| {
            if let Some(ta) = textarea_ref.cast::<HtmlTextAreaElement>() {
                let _ = ta.blur();
            }
        })
    };

    html! {
        <div class="panel source-panel">
            <div class="panel-toolbar">
                <div class="button-group">
                    <button class="icon-button" title="Undo" onclick={undo} disabled={state.history_index == 0}>
                        <svg viewBox="0 0 24 24"><path d="M3 10h10a5 5 0 0 1 5 5v0a5 5 0 0 1-5 5H9"/><polyline points="7 6 3 10 7 14"/></svg>
                    </button>
                    <button class="icon-button" title="Redo" onclick={redo} disabled={state.history_index + 1 >= state.history.len()}>
                        <svg viewBox="0 0 24 24"><path d="M21 10H11a5 5 0 0 0-5 5v0a5 5 0 0 0 5 5h4"/><polyline points="17 6 21 10 17 14"/></svg>
                    </button>
                </div>
            </div>
            <textarea 
                ref={textarea_ref}
                class={classes!("source-editor", invalid_class)}
                value={state.source_text.clone()}
                {oninput}
                {onchange}
                {onmouseenter}
                {onmouseleave}
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
