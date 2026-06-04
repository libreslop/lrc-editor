use yew::prelude::*;
use crate::web_app::actions::AppState;

#[derive(Properties, PartialEq)]
pub struct PlaybackControlsProps {
    pub state: UseReducerHandle<AppState>,
    pub timecode_ref: NodeRef,
    pub on_toggle_play: Callback<MouseEvent>,
    pub on_zoom_in: Callback<MouseEvent>,
    pub on_zoom_out: Callback<MouseEvent>,
    pub scroll_left: f64,
    pub viewport_width: f64,
    pub total_width: f64,
    pub on_scrollbar_mousedown: Callback<MouseEvent>,
}

#[function_component(PlaybackControls)]
pub fn playback_controls(props: &PlaybackControlsProps) -> Html {
    let time_str = {
        let total_secs = props.state.current_time_ms.as_u32() / 1000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        let ms = props.state.current_time_ms.as_u32() % 1000;
        format!("{:02}:{:02}.{:03}", mins, secs, ms)
    };

    let scroll_handle_style = if props.total_width > 0.0 {
        let ratio = (props.viewport_width / props.total_width).min(1.0);
        let left_ratio = (props.scroll_left / props.total_width).min(1.0 - ratio);
        format!("width: {}%; left: {}%;", ratio * 100.0, left_ratio * 100.0)
    } else {
        "width: 100%; left: 0%;".to_string()
    };

    html! {
        <div class="transport-strip">
            <span class="timecode" ref={props.timecode_ref.clone()}>{ time_str }</span>
            <button class="transport-button" title={if props.state.playing { "Pause" } else { "Play" }} onclick={props.on_toggle_play.clone()}>
                if props.state.playing {
                    <svg viewBox="0 0 24 24"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
                } else {
                    <svg viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                }
            </button>
            
            <div class="custom-scrollbar-track" onmousedown={props.on_scrollbar_mousedown.clone()}>
                <div class="custom-scrollbar-handle" style={scroll_handle_style}></div>
            </div>

            <div class="zoom-controls">
                <button class="icon-button" title="Zoom Out" onclick={props.on_zoom_out.clone()}>
                    <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
                </button>
                <button class="icon-button" title="Zoom In" onclick={props.on_zoom_in.clone()}>
                    <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
                </button>
            </div>
        </div>
    }
}
