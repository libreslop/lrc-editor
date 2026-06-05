use yew::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::web_app::actions::AppState;
use crate::domain::{TimeMs, Pixels};
use crate::web_app::components::timeline::DragTarget;

#[hook]
pub fn use_playback_sync(
    state: UseReducerHandle<AppState>,
    drag_mode: UseStateHandle<Option<DragTarget>>,
    playhead_ref: NodeRef,
    viewport_ref: NodeRef,
    timecode_ref: NodeRef,
    is_scrollbar_dragged: Rc<RefCell<bool>>,
    last_mouse_pos: Rc<RefCell<crate::domain::Vec2>>,
    suppress_panning: Rc<RefCell<bool>>,
    scroll_left_state: UseStateHandle<f64>,
    ignore_next_scroll: Rc<RefCell<bool>>,
    current_time_ms_ref: Rc<RefCell<TimeMs>>,
    px_per_second_ref: Rc<RefCell<Pixels>>,
    last_user_input_time: Rc<RefCell<f64>>,
) {
    let playing = state.playback.playing;
    let dragging_playhead = *drag_mode == Some(DragTarget::Playhead);
    
    use_effect_with((playing, dragging_playhead), move |(playing, dragging_playhead)| {
        let cb = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
        let cb_clone = cb.clone();
        
        let frame_id = Rc::new(RefCell::new(None::<i32>));
        let frame_id_clone = frame_id.clone();
        
        let playhead = playhead_ref.clone();
        let viewport = viewport_ref.clone();
        let timecode = timecode_ref.clone();
        let last_mouse_pos = last_mouse_pos.clone();
        let is_dragging = *dragging_playhead;
        let suppress_panning = suppress_panning.clone();
        let scroll_left_state = scroll_left_state.clone();
        let state = state.clone();
        let ignore_next_scroll = ignore_next_scroll.clone();
        let last_user_input_time = last_user_input_time.clone();

        let mut prev_world_x_opt = None::<f64>;

        if *playing || is_dragging {
            *cb_clone.borrow_mut() = Some(Closure::wrap(Box::new({
                let ignore_next_scroll = ignore_next_scroll.clone();
                let suppress_panning = suppress_panning.clone();
                let last_user_input_time = last_user_input_time.clone();
                move || {
                if let (Some(p), Some(v)) = (
                    playhead.cast::<web_sys::HtmlElement>(),
                    viewport.cast::<web_sys::HtmlElement>(),
                ) {
                    let rect = v.get_bounding_client_rect();
                    let px = px_per_second_ref.borrow().as_f64();
                    let current_time_ms = *current_time_ms_ref.borrow();
                    
                    let playhead_x = if is_dragging {
                        let mouse_x = last_mouse_pos.borrow().x;
                        
                        // Calculate panning velocity based on mouse position relative to viewport rect
                        let mut vel = 0.0;
                        let safe_zone = 90.0;
                        if mouse_x < rect.left() + safe_zone {
                            let ratio = (rect.left() + safe_zone - mouse_x) / safe_zone;
                            vel = -10.0 * ratio.min(1.0);
                        } else if mouse_x > rect.right() - safe_zone {
                            let ratio = (mouse_x - (rect.right() - safe_zone)) / safe_zone;
                            vel = 10.0 * ratio.min(1.0);
                        }
                        
                        // Determine if we can pan (meaning we are not at the scroll limits)
                        let mut can_pan = false;
                        if vel < 0.0 {
                            can_pan = v.scroll_left() > 0;
                        } else if vel > 0.0 {
                            let max_scroll = (v.scroll_width() - v.client_width()).max(0);
                            can_pan = v.scroll_left() < max_scroll;
                        }

                        // Apply programmatic scrolling if vel is non-zero and we can pan
                        if vel != 0.0 && can_pan {
                            let old_scroll = v.scroll_left();
                            v.set_scroll_left(old_scroll + vel as i32);
                            scroll_left_state.set(v.scroll_left() as f64);
                        }

                        let absolute_x = mouse_x - rect.left() + v.scroll_left() as f64;
                        let duration_ms = state.max_timeline_duration();
                        let width_px_val = duration_ms.to_secs() * px;
                        
                        let min_x = (v.scroll_left() as f64).max(0.0);
                        let max_x = (v.scroll_left() as f64 + v.client_width() as f64).min(width_px_val);
                        let clamped_x = absolute_x.clamp(min_x, max_x);

                        let target_time_ms = (clamped_x / px * 1000.0) as i32;
                        let new_time = TimeMs(target_time_ms.max(0) as u32);
                        
                        // Disable snapping when actively panning to prevent visual playhead jitter,
                        // but keep snapping enabled when at the scrolling boundaries.
                        let snapped_time = if vel != 0.0 && can_pan {
                            new_time
                        } else {
                            crate::web_app::editor::timeline::TimelineSnapper::snap_playhead(
                                &state,
                                new_time,
                                duration_ms,
                                *px_per_second_ref.borrow(),
                            )
                        };
                        
                        *current_time_ms_ref.borrow_mut() = snapped_time;
                        state.dispatch(crate::web_app::actions::AppAction::SetTime(snapped_time));

                        let calculated_playhead_x = snapped_time.to_secs() * px;

                        let is_in_safe_zone = calculated_playhead_x < v.scroll_left() as f64 + safe_zone 
                            || calculated_playhead_x > v.scroll_left() as f64 + v.client_width() as f64 - safe_zone;
                        let hit_right_border = calculated_playhead_x >= v.scroll_left() as f64 + v.client_width() as f64 - 1.0;

                        if hit_right_border || !is_in_safe_zone {
                            *suppress_panning.borrow_mut() = false;
                        } else if is_in_safe_zone {
                            *suppress_panning.borrow_mut() = true;
                        }

                        calculated_playhead_x
                    } else {
                        current_time_ms.to_secs() * px
                    };
                    
                    let _ = p.set_attribute("style", &format!("transform: translateX({}px);", playhead_x));

                    if !*is_scrollbar_dragged.borrow() && !is_dragging {
                        let world_x = current_time_ms.to_secs() * px;
                        let scroll_left = v.scroll_left() as f64;
                        let client_width = v.client_width() as f64;
                        let safe_zone = 90.0;
                        
                        let is_in_safe_zone = world_x < scroll_left + safe_zone || world_x > scroll_left + client_width - safe_zone;
                        let is_off_screen = world_x < scroll_left || world_x > scroll_left + client_width;

                        let mut entered_safe_zone = false;
                        if let Some(prev_world_x) = prev_world_x_opt {
                            let prev_is_in_safe_zone = prev_world_x < scroll_left + safe_zone || prev_world_x > scroll_left + client_width - safe_zone;
                            let prev_is_off_screen = prev_world_x < scroll_left || prev_world_x > scroll_left + client_width;
                            if !prev_is_in_safe_zone && !prev_is_off_screen && is_in_safe_zone {
                                entered_safe_zone = true;
                            }
                        }
                        prev_world_x_opt = Some(world_x);

                        let mut should_pan = is_off_screen;
                        if is_in_safe_zone && entered_safe_zone && !*suppress_panning.borrow() {
                            should_pan = true;
                        }

                        if should_pan {
                            let elapsed_since_input = js_sys::Date::now() - *last_user_input_time.borrow();
                            if elapsed_since_input >= 3000.0 {
                                *ignore_next_scroll.borrow_mut() = true;
                                let is_to_the_right = world_x > scroll_left + client_width - safe_zone;
                                let target_scroll = if is_to_the_right {
                                    (world_x - safe_zone) as i32
                                } else {
                                    (world_x - client_width / 2.0) as i32
                                };
                                v.set_scroll_left(target_scroll);
                                scroll_left_state.set(v.scroll_left() as f64);
                            }
                        }

                        let hit_right_border = world_x >= scroll_left + client_width - 1.0;
                        if hit_right_border || !is_in_safe_zone {
                            *suppress_panning.borrow_mut() = false;
                        }
                    }

                    if let Some(tc) = timecode.cast::<web_sys::HtmlElement>() {
                        let total_secs = current_time_ms.as_u32() / 1000;
                        let mins = total_secs / 60;
                        let secs = total_secs % 60;
                        let ms = current_time_ms.as_u32() % 1000;
                        tc.set_inner_text(&format!("{:02}:{:02}.{:03}", mins, secs, ms));
                    }
                }
                if let Some(window) = web_sys::window()
                    && let Some(closure) = cb.borrow().as_ref()
                        && let Ok(id) = window.request_animation_frame(closure.as_ref().unchecked_ref()) {
                            *frame_id_clone.borrow_mut() = Some(id);
                        }
            }}) as Box<dyn FnMut()>));
            
            if let Some(window) = web_sys::window()
                && let Ok(id) = window.request_animation_frame(cb_clone.borrow().as_ref().unwrap().as_ref().unchecked_ref()) {
                    *frame_id.borrow_mut() = Some(id);
                }
        } else {
            *cb_clone.borrow_mut() = None;
        }
        
        move || {
            if let Some(window) = web_sys::window()
                && let Some(id) = *frame_id.borrow() {
                    let _ = window.cancel_animation_frame(id);
                }
            *cb_clone.borrow_mut() = None;
        }
    });
}
