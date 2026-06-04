use yew::prelude::*;
use crate::domain::{Pixels, SelectionMode};
use crate::web_app::actions::{AppState, AppAction};
use super::DragTarget;

#[derive(Properties, PartialEq)]
pub struct LyricChunkProps {
    pub state: UseReducerHandle<AppState>,
    pub entry_id: usize,
    pub text: String,
    pub is_empty: bool,
    pub start_px: Pixels,
    pub width: Pixels,
    pub is_selected: bool,
    pub on_drag_start: Callback<(MouseEvent, DragTarget)>,
}

#[function_component(LyricChunk)]
pub fn lyric_chunk(props: &LyricChunkProps) -> Html {
    let mut classes = classes!("lyric-chunk");
    if props.is_selected { classes.push("selected"); }
    if props.is_empty { classes.push("empty-gap"); }

    let onmousedown_body = {
        let entry_id = props.entry_id;
        let state = props.state.clone();
        let is_selected = props.is_selected;
        let on_drag_start = props.on_drag_start.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            
            let mut mode = SelectionMode::Replace;
            let mut should_select = !is_selected;
            
            if e.shift_key() {
                mode = SelectionMode::Range;
                should_select = true;
            } else if e.ctrl_key() || e.meta_key() {
                mode = SelectionMode::Toggle;
                should_select = true;
            }
            
            if should_select {
                state.dispatch(AppAction::SelectEntry(entry_id, mode));
            }
            
            on_drag_start.emit((e, DragTarget::Body));
        })
    };

    let onmousedown_left = {
        let state = props.state.clone();
        let on_drag_start = props.on_drag_start.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            state.dispatch(AppAction::ClearSelection);
            on_drag_start.emit((e, DragTarget::LeftEdge));
        })
    };

    let onmousedown_right = {
        let state = props.state.clone();
        let on_drag_start = props.on_drag_start.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            state.dispatch(AppAction::ClearSelection);
            on_drag_start.emit((e, DragTarget::RightEdge));
        })
    };

    html! {
        <div 
            class={classes} 
            style={format!("left: {}px; width: {}px;", props.start_px.as_f64(), props.width.as_f64())}
            onmousedown={onmousedown_body}
        >
            { &props.text }
            <div class="edge-handle left" onmousedown={onmousedown_left}></div>
            <div class="edge-handle right" onmousedown={onmousedown_right}></div>
        </div>
    }
}
