use yew::prelude::*;
use crate::web_app::actions::{AppState, AppAction};
use crate::domain::ZoomLevel;

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
    last_user_input_time: std::rc::Rc<std::cell::RefCell<f64>>,
) -> ZoomHandlers {
    let zoom_in = {
        let state = state.clone();
        let viewport_ref = viewport_ref.clone();
        let scroll_left_state = scroll_left.clone();
        let viewport_width = viewport_width.clone();
        let last_user_input_time = last_user_input_time.clone();
        Callback::from(move |_| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            let zoom = state.view.zoom_level.as_f64();
            let vp_width = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let w = vp.client_width() as f64;
                if w > 0.0 { w } else { *viewport_width }
            } else {
                *viewport_width
            };
            let max_dur = state.max_timeline_duration();
            let dur_secs = max_dur.to_secs();
            let min_zoom = if vp_width > 0.0 && dur_secs > 0.0 {
                vp_width / (dur_secs * 92.0)
            } else {
                0.001
            };
            let old_px_per_second = 92.0 * zoom;
            let new_zoom = (zoom * 1.25).clamp(min_zoom, 10.0);
            let new_px_per_second = 92.0 * new_zoom;

            let scroll_left_old = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                vp.scroll_left() as f64
            } else {
                *scroll_left_state
            };

            let center_x_old = scroll_left_old + vp_width / 2.0;
            let center_x_new = center_x_old * (new_px_per_second / old_px_per_second);
            let new_scroll = center_x_new - vp_width / 2.0;

            let total_width_px = (max_dur.as_u32() as f64 / 1000.0) * new_px_per_second;
            let max_scroll_px = (total_width_px - vp_width).max(0.0);
            let new_scroll_clamped = new_scroll.clamp(0.0, max_scroll_px);

            state.dispatch(AppAction::SetZoom(ZoomLevel(new_zoom)));
            scroll_left_state.set(new_scroll_clamped);
        })
    };

    let zoom_out = {
        let state = state.clone();
        let viewport_ref = viewport_ref.clone();
        let scroll_left_state = scroll_left.clone();
        let viewport_width = viewport_width.clone();
        let last_user_input_time = last_user_input_time.clone();
        Callback::from(move |_| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            let zoom = state.view.zoom_level.as_f64();
            let vp_width = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let w = vp.client_width() as f64;
                if w > 0.0 { w } else { *viewport_width }
            } else {
                *viewport_width
            };
            let max_dur = state.max_timeline_duration();
            let dur_secs = max_dur.to_secs();
            let min_zoom = if vp_width > 0.0 && dur_secs > 0.0 {
                vp_width / (dur_secs * 92.0)
            } else {
                0.001
            };
            let old_px_per_second = 92.0 * zoom;
            let new_zoom = (zoom / 1.25).clamp(min_zoom, 10.0);
            let new_px_per_second = 92.0 * new_zoom;

            let scroll_left_old = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                vp.scroll_left() as f64
            } else {
                *scroll_left_state
            };

            let center_x_old = scroll_left_old + vp_width / 2.0;
            let center_x_new = center_x_old * (new_px_per_second / old_px_per_second);
            let new_scroll = center_x_new - vp_width / 2.0;

            let total_width_px = (max_dur.as_u32() as f64 / 1000.0) * new_px_per_second;
            let max_scroll_px = (total_width_px - vp_width).max(0.0);
            let new_scroll_clamped = new_scroll.clamp(0.0, max_scroll_px);

            state.dispatch(AppAction::SetZoom(ZoomLevel(new_zoom)));
            scroll_left_state.set(new_scroll_clamped);
        })
    };

    ZoomHandlers {
        zoom_in,
        zoom_out,
    }
}
