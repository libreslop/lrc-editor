use yew::prelude::*;
use crate::web_app::actions::{AppState, AppAction};
use crate::domain::ZoomLevel;
use wasm_bindgen::JsCast;

pub struct ZoomHandlers {
    pub zoom_in: Callback<MouseEvent>,
    pub zoom_out: Callback<MouseEvent>,
}

#[hook]
pub fn use_zoom_handlers(
    state: UseReducerHandle<AppState>,
    viewport_ref: NodeRef,
    scroll_left: UseStateHandle<f64>,
    viewport_width: UseStateHandle<f64>,
) -> ZoomHandlers {
    let zoom_in = {
        let state = state.clone();
        let viewport_ref = viewport_ref.clone();
        let scroll_left_state = scroll_left.clone();
        let viewport_width = viewport_width.clone();
        Callback::from(move |_| {
            let zoom = state.view.zoom_level.as_f64();
            let current_time_ms = state.playback.current_time_ms;
            let vp_width = *viewport_width;
            let dur_secs = state.max_timeline_duration().to_secs();
            let min_zoom = if vp_width > 0.0 && dur_secs > 0.0 {
                vp_width / (dur_secs * 92.0)
            } else {
                0.001
            };
            let old_px_per_second = 92.0 * zoom;
            let new_zoom = (zoom * 1.25).clamp(min_zoom, 10.0);
            let new_px_per_second = 92.0 * new_zoom;
            let playhead_x_old = current_time_ms.to_secs() * old_px_per_second;
            let playhead_x_new = current_time_ms.to_secs() * new_px_per_second;
            
            let screen_x = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                playhead_x_old - vp.scroll_left() as f64
            } else {
                0.0
            };
            let new_scroll = playhead_x_new - screen_x;

            state.dispatch(AppAction::SetZoom(ZoomLevel(new_zoom)));
            
            if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let vp_clone = vp.clone();
                let scroll_left_state = scroll_left_state.clone();
                let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                    vp_clone.set_scroll_left(new_scroll as i32);
                    scroll_left_state.set(vp_clone.scroll_left() as f64);
                });
                let _ = web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
            }
        })
    };

    let zoom_out = {
        let state = state.clone();
        let viewport_ref = viewport_ref.clone();
        let scroll_left_state = scroll_left.clone();
        let viewport_width = viewport_width.clone();
        Callback::from(move |_| {
            let zoom = state.view.zoom_level.as_f64();
            let current_time_ms = state.playback.current_time_ms;
            let vp_width = *viewport_width;
            let dur_secs = state.max_timeline_duration().to_secs();
            let min_zoom = if vp_width > 0.0 && dur_secs > 0.0 {
                vp_width / (dur_secs * 92.0)
            } else {
                0.001
            };
            let old_px_per_second = 92.0 * zoom;
            let new_zoom = (zoom / 1.25).clamp(min_zoom, 10.0);
            let new_px_per_second = 92.0 * new_zoom;
            let playhead_x_old = current_time_ms.to_secs() * old_px_per_second;
            let playhead_x_new = current_time_ms.to_secs() * new_px_per_second;
            
            let screen_x = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                playhead_x_old - vp.scroll_left() as f64
            } else {
                0.0
            };
            let new_scroll = playhead_x_new - screen_x;

            state.dispatch(AppAction::SetZoom(ZoomLevel(new_zoom)));
            
            if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let vp_clone = vp.clone();
                let scroll_left_state = scroll_left_state.clone();
                let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                    vp_clone.set_scroll_left(new_scroll as i32);
                    scroll_left_state.set(vp_clone.scroll_left() as f64);
                });
                let _ = web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
            }
        })
    };

    ZoomHandlers {
        zoom_in,
        zoom_out,
    }
}
