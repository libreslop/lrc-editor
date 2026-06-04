use yew::prelude::*;
use crate::web_app::actions::{AppState, AppAction};
use web_sys::HtmlTextAreaElement;

#[derive(Properties, PartialEq)]
pub struct SourcePanelProps {
    pub state: UseReducerHandle<AppState>,
    pub on_home_click: Callback<MouseEvent>,
    pub on_home_aux_click: Callback<MouseEvent>,
    pub on_help_click: Callback<MouseEvent>,
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

    let invalid_class = if state.document.parse_error.is_some() { "invalid" } else { "" };

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
            <div class="panel-toolbar" style="justify-content: space-between;">
                <div style="display: flex; gap: 8px; align-items: center;">
                    <button class="icon-button" title="Home (tools.siri.ws)" onclick={props.on_home_click.clone()} onauxclick={props.on_home_aux_click.clone()}>
                        <svg viewBox="0 0 24 24"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
                    </button>
                    <div class="button-group">
                        <button class="icon-button" title="Undo" onclick={undo} disabled={state.history.history_index == 0}>
                            <svg viewBox="0 0 24 24"><path d="M3 10h10a5 5 0 0 1 5 5v0a5 5 0 0 1-5 5H9"/><polyline points="7 6 3 10 7 14"/></svg>
                        </button>
                        <button class="icon-button" title="Redo" onclick={redo} disabled={state.history.history_index + 1 >= state.history.history.len()}>
                            <svg viewBox="0 0 24 24"><path d="M21 10H11a5 5 0 0 0-5 5v0a5 5 0 0 0 5 5h4"/><polyline points="17 6 21 10 17 14"/></svg>
                        </button>
                    </div>
                </div>
                <button class="icon-button" title="Keybinds Help" onclick={props.on_help_click.clone()}>
                    <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
                </button>
            </div>
            <textarea 
                ref={textarea_ref}
                class={classes!("source-editor", invalid_class)}
                value={state.document.source_text.clone()}
                {oninput}
                {onchange}
                {onmouseenter}
                {onmouseleave}
                spellcheck="false"
            />
            if let Some(err) = &state.document.parse_error {
                <div class="toast">
                    { err }
                </div>
            }
        </div>
    }
}
