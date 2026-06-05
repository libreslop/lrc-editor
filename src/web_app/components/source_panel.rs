use yew::prelude::*;
use crate::web_app::actions::{AppState, AppAction};
use web_sys::{HtmlTextAreaElement, HtmlElement};
use wasm_bindgen::JsCast;

fn is_timestamp(tag: &str) -> bool {
    if let Some(c) = tag.chars().next() {
        c.is_ascii_digit()
    } else {
        false
    }
}

fn highlight_line(line: &str) -> Vec<Html> {
    let mut nodes = Vec::new();
    if line.is_empty() {
        return nodes;
    }

    let mut rest = line;
    
    // 1. Scan bracketed tags at the start of the line (can be multiple)
    while rest.starts_with('[') {
        if let Some(close_idx) = rest.find(']') {
            let tag_content = &rest[1..close_idx];
            
            nodes.push(html! { <span class="hl-bracket">{"["}</span> });
            if is_timestamp(tag_content) {
                nodes.push(html! { <span class="hl-timestamp">{tag_content}</span> });
            } else if tag_content.contains(':') {
                if let Some(colon_idx) = tag_content.find(':') {
                    let key = &tag_content[0..colon_idx];
                    let val = &tag_content[colon_idx+1..];
                    nodes.push(html! { <span class="hl-metadata-key">{key}</span> });
                    nodes.push(html! { <span class="hl-metadata-colon">{":"}</span> });
                    nodes.push(html! { <span class="hl-metadata-value">{val}</span> });
                } else {
                    nodes.push(html! { <span class="hl-metadata-key">{tag_content}</span> });
                }
            } else {
                nodes.push(html! { <span class="hl-invalid">{tag_content}</span> });
            }
            nodes.push(html! { <span class="hl-bracket">{"]"}</span> });
            
            rest = &rest[close_idx + 1..];
        } else {
            nodes.push(html! { <span class="hl-invalid">{rest}</span> });
            rest = "";
            break;
        }
    }

    // 2. Scan remaining text for word timestamps like `<00:12.50>`
    let mut text_rest = rest;
    while !text_rest.is_empty() {
        if let Some(open_idx) = text_rest.find('<') {
            if open_idx > 0 {
                let text_segment = &text_rest[0..open_idx];
                nodes.push(html! { <span class="hl-lyric">{text_segment}</span> });
            }
            
            let bracket_rest = &text_rest[open_idx..];
            if let Some(close_idx) = bracket_rest.find('>') {
                let tag_content = &bracket_rest[1..close_idx];
                nodes.push(html! { <span class="hl-bracket">{"<"}</span> });
                if is_timestamp(tag_content) {
                    nodes.push(html! { <span class="hl-timestamp-word">{tag_content}</span> });
                } else {
                    nodes.push(html! { <span class="hl-invalid">{tag_content}</span> });
                }
                nodes.push(html! { <span class="hl-bracket">{">"}</span> });
                text_rest = &bracket_rest[close_idx + 1..];
            } else {
                nodes.push(html! { <span class="hl-invalid">{bracket_rest}</span> });
                break;
            }
        } else {
            nodes.push(html! { <span class="hl-lyric">{text_rest}</span> });
            break;
        }
    }

    nodes
}

fn highlight_lrc(text: &str, selected_line_idx: Option<usize>) -> Html {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut divs = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let is_selected = Some(idx) == selected_line_idx;
        let mut line_nodes = highlight_line(line);
        if line_nodes.is_empty() {
            line_nodes.push(html! { "\u{200b}" });
        }
        
        let class = classes!(
            "hl-line",
            is_selected.then_some("selected")
        );

        divs.push(html! {
            <div {class} data-line-idx={idx.to_string()}>
                { for line_nodes }
            </div>
        });
    }

    html! {
        <>
            { for divs }
        </>
    }
}

fn get_end_of_line_utf16(source_text: &str, target_line_idx: usize) -> Option<usize> {
    let mut current_offset_utf16 = 0;
    let lines: Vec<&str> = source_text.split('\n').collect();
    if target_line_idx < lines.len() {
        for idx in 0..=target_line_idx {
            let line_len = lines[idx].encode_utf16().count();
            if idx == target_line_idx {
                return Some(current_offset_utf16 + line_len);
            }
            current_offset_utf16 += line_len + 1; // +1 for the '\n' character
        }
    }
    None
}

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
    let last_positioned_selection = use_state(|| Vec::<usize>::new());
    
    let selected_line_idx = state.view.selection.last_selected_id()
        .and_then(|sel_id| state.document.document.as_ref()
            .and_then(|doc| doc.entry_by_uid(sel_id)
                .map(|entry| entry.source_line.as_zero_based())));
    
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
    let highlight_ref = use_node_ref();

    let onscroll = {
        let textarea_ref = textarea_ref.clone();
        let highlight_ref = highlight_ref.clone();
        Callback::from(move |_e: Event| {
            if let Some(textarea) = textarea_ref.cast::<HtmlTextAreaElement>() {
                if let Some(highlight) = highlight_ref.cast::<HtmlElement>() {
                    highlight.set_scroll_top(textarea.scroll_top());
                    highlight.set_scroll_left(textarea.scroll_left());
                }
            }
        })
    };

    {
        let textarea_ref = textarea_ref.clone();
        let highlight_ref = highlight_ref.clone();
        let source_text = state.document.source_text.clone();
        use_effect_with(source_text, move |_| {
            if let Some(textarea) = textarea_ref.cast::<HtmlTextAreaElement>() {
                if let Some(highlight) = highlight_ref.cast::<HtmlElement>() {
                    highlight.set_scroll_top(textarea.scroll_top());
                    highlight.set_scroll_left(textarea.scroll_left());
                }
            }
        });
    }

    {
        let textarea_ref = textarea_ref.clone();
        let highlight_ref = highlight_ref.clone();
        use_effect_with(selected_line_idx, move |&sel_idx| {
            if sel_idx.is_some() {
                if let Some(textarea) = textarea_ref.cast::<HtmlTextAreaElement>() {
                    if let Some(highlight) = highlight_ref.cast::<HtmlElement>() {
                        if let Some(selected_el) = highlight.query_selector(".hl-line.selected").ok().flatten() {
                            if let Ok(html_el) = selected_el.dyn_into::<HtmlElement>() {
                                let offset_top = html_el.offset_top();
                                let offset_height = html_el.offset_height();
                                let client_height = textarea.client_height();
                                
                                let target_scroll_top = offset_top - (client_height / 2) + (offset_height / 2);
                                let target_scroll_top = target_scroll_top.max(0);
                                
                                if let Ok(scroll_to_val) = js_sys::Reflect::get(&textarea, &"scrollTo".into()) {
                                    if let Some(scroll_to_func) = scroll_to_val.dyn_ref::<js_sys::Function>() {
                                        let options = js_sys::Object::new();
                                        let _ = js_sys::Reflect::set(&options, &"top".into(), &(target_scroll_top as f64).into());
                                        let _ = js_sys::Reflect::set(&options, &"behavior".into(), &"smooth".into());
                                        let _ = scroll_to_func.call1(&textarea, &options);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }


    let onmouseenter = {
        let textarea_ref = textarea_ref.clone();
        let state = state.clone();
        let last_positioned_selection = last_positioned_selection.clone();
        Callback::from(move |_| {
            if let Some(ta) = textarea_ref.cast::<HtmlTextAreaElement>() {
                let _ = ta.focus();
                
                let current_selection = state.view.selection.selection_order().to_vec();
                if current_selection != *last_positioned_selection {
                    last_positioned_selection.set(current_selection);
                    
                    if let Some(last_sel_id) = state.view.selection.last_selected_id() {
                        if let Some(doc) = &state.document.document {
                            if let Some(entry) = doc.entry_by_uid(last_sel_id) {
                                let target_line_idx = entry.source_line.as_zero_based();
                                if let Some(pos) = get_end_of_line_utf16(&state.document.source_text, target_line_idx) {
                                    let _ = ta.set_selection_range(pos as u32, pos as u32);
                                }
                            }
                        }
                    }
                }
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
            <div class={classes!("editor-container", invalid_class)}>
                <pre ref={highlight_ref} class="source-highlight">
                    { highlight_lrc(&state.document.source_text, selected_line_idx) }
                    { "\u{200b}" }
                </pre>
                <textarea 
                    ref={textarea_ref}
                    class="source-editor"
                    value={state.document.source_text.clone()}
                    {oninput}
                    {onchange}
                    {onscroll}
                    {onmouseenter}
                    {onmouseleave}
                    spellcheck="false"
                />
            </div>
            if let Some(err) = &state.document.parse_error {
                <div class="toast">
                    { err }
                </div>
            }
        </div>
    }
}

