use yew::prelude::*;
use web_sys::HtmlAudioElement;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use crate::web_app::actions::{AppState, AppAction};
use crate::domain::{TimeMs, Pixels, ZoomLevel};
use super::timeline::{PlaybackControls, TrackPads, TimelineLanes, DragTarget};
use super::timeline::waveform_canvas::WaveformSummary;

#[derive(Properties, PartialEq)]
pub struct TimelinePanelProps {
    pub state: UseReducerHandle<AppState>,
}

#[function_component(TimelinePanel)]
pub fn timeline_panel(props: &TimelinePanelProps) -> Html {
    let px_per_second = Pixels(92.0 * props.state.view.zoom_level.as_f64());
    
    let audio_ref = use_node_ref();
    let file_input_ref = use_node_ref();
    let lrc_input_ref = use_node_ref();
    let canvas_ref = use_node_ref();
    let viewport_ref = use_node_ref();
    let playhead_ref = use_node_ref();
    let timecode_ref = use_node_ref();
    let scrollbar_track_ref = use_node_ref();
    let scrollbar_handle_ref = use_node_ref();
    
    let audio_url = use_state(|| None::<String>);
    let waveform_summary = use_state(|| None::<Rc<WaveformSummary>>);
    let scroll_left = use_state(|| 0.0);
    let viewport_width = use_state(|| 0.0);
    let viewport_scroll_width = use_state(|| 0.0);

    let drag_mode = use_state(|| None::<DragTarget>);
    let suppress_panning = use_mut_ref(|| false);
    let drag_start_x = use_mut_ref(|| 0.0);
    let drag_start_y = use_state(|| Pixels(0.0));
    let selection_rect = use_state(|| None::<crate::domain::Rect>);
    let drag_offset_ms = use_state(|| 0i32);
    let drag_target_uid = use_state(|| None::<usize>);
    let drag_scrollbar_track_left = use_mut_ref(|| 0.0);
    let drag_scrollbar_track_width = use_mut_ref(|| 1.0);
    let drag_scrollbar_handle_offset = use_mut_ref(|| 0.0);
    let is_scrollbar_dragged = use_mut_ref(|| false);
    let last_mouse_pos = use_mut_ref(|| crate::domain::Vec2 { x: 0.0, y: 0.0 });
    let ignore_next_scroll = use_mut_ref(|| false);
    let last_user_input_time = use_mut_ref(|| 0.0);

    let hover_lyrics_time = use_state(|| None::<TimeMs>);
    let drag_create_start = use_state(|| None::<TimeMs>);
    let drag_create_current = use_state(|| None::<TimeMs>);

    let file_handlers = crate::web_app::components::timeline::hooks::use_file_handlers(
        props.state.clone(),
        audio_url.clone(),
        waveform_summary.clone(),
        file_input_ref.clone(),
        lrc_input_ref.clone(),
    );

    // Load cached audio from IndexedDB on mount
    {
        let state = props.state.clone();
        let audio_url = audio_url.clone();
        let waveform_summary = waveform_summary.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(saved) = crate::web_app::indexed_db::load_audio_file().await {
                    state.dispatch(AppAction::SetAudioFilename(saved.name));
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&saved.blob) {
                        audio_url.set(Some(url.clone()));
                        crate::web_app::components::timeline::hooks::file_handlers::load_waveform_from_url(url, waveform_summary);
                    }
                }
            });
        });
    }

    let on_file_change = file_handlers.on_file_change;
    let import_click = file_handlers.import_click;
    let import_lrc_click = file_handlers.import_lrc_click;
    let on_lrc_change = file_handlers.on_lrc_change;
    let export_lrc = file_handlers.export_lrc;
    let on_loaded_metadata = file_handlers.on_loaded_metadata;


    
    let current_time_ms_ref = use_mut_ref(|| props.state.playback.current_time_ms);
    *current_time_ms_ref.borrow_mut() = props.state.playback.current_time_ms;

    let px_per_second_ref = use_mut_ref(|| px_per_second);
    *px_per_second_ref.borrow_mut() = px_per_second;

    let duration_ms = props.state.max_timeline_duration();
    let width_px = Pixels(duration_ms.to_secs() * px_per_second.as_f64());
    let audio_width_px = Pixels(props.state.playback.duration_ms.to_secs() * px_per_second.as_f64());

    // Drag & Pan loop
    {
        let viewport_ref = viewport_ref.clone();
        let px_per_second_ref = px_per_second_ref.clone();
        let drag_mode = drag_mode.clone();
        let drag_offset_ms = drag_offset_ms.clone();

        use_effect(move || {
            let interval = gloo_timers::callback::Interval::new(16, move || {
                let mode = *drag_mode;
                
                if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    let old_scroll = v.scroll_left();
                    let actual_delta = v.scroll_left() - old_scroll;
                    
                    if mode.is_some() && mode != Some(DragTarget::Playhead) && actual_delta != 0 {
                        let px = px_per_second_ref.borrow().as_f64();
                        let delta_ms = (actual_delta as f64 / px * 1000.0) as i32;
                        drag_offset_ms.set(*drag_offset_ms + delta_ms);
                    }
                }
            });
            move || drop(interval)
        });
    }

    // Playback loop effect
    {
        let playing = props.state.playback.playing;
        let audio_ref = audio_ref.clone();
        let state = props.state.clone();
        
        let current_time_ref = yew::use_mut_ref(|| props.state.playback.current_time_ms);
        *current_time_ref.borrow_mut() = props.state.playback.current_time_ms;

        let bounds_ref = yew::use_mut_ref(|| TimeMs(0));
        *bounds_ref.borrow_mut() = props.state.max_timeline_duration();

        let last_seek_ref = yew::use_mut_ref(|| props.state.playback.last_seek_request);
        *last_seek_ref.borrow_mut() = props.state.playback.last_seek_request;

        let dragging_playhead = *drag_mode == Some(DragTarget::Playhead);

        use_effect_with((playing, dragging_playhead), move |(playing, dragging_playhead)| {
            let mut interval_opt = None;
            
            if *playing && !*dragging_playhead {
                let start_time = *current_time_ref.borrow();
                
                if let Some(audio) = audio_ref.cast::<web_sys::HtmlAudioElement>() {
                    let dur = audio.duration();
                    let audio_dur_ms = if dur.is_nan() || dur.is_infinite() {
                        0
                    } else {
                        (dur * 1000.0) as u32
                    };
                    if start_time.as_u32() < audio_dur_ms {
                        if audio.ready_state() >= 1 {
                            audio.set_current_time(start_time.to_secs());
                        }
                        let _ = audio.play();
                    } else {
                        let _ = audio.pause();
                    }
                }
                
                let last_time = js_sys::Date::now();
                let last_time_ref = std::rc::Rc::new(std::cell::Cell::new(last_time));
                
                let mut local_current_f64 = start_time.as_u32() as f64;
                let mut last_handled_seek = None;
                
                let interval = gloo_timers::callback::Interval::new(16, move || {
                    let now = js_sys::Date::now();
                    let delta = now - last_time_ref.get();
                    last_time_ref.set(now);
                    
                    let current_seek = *last_seek_ref.borrow();
                    let mut seek_just_handled = false;
                    if current_seek != last_handled_seek {
                        last_handled_seek = current_seek;
                        if let Some(seek_time) = current_seek {
                            local_current_f64 = seek_time.as_u32() as f64;
                            seek_just_handled = true;
                        }
                    }
                    
                    if let Some(audio) = audio_ref.cast::<web_sys::HtmlAudioElement>() {
                        let dur = audio.duration();
                        let audio_dur_ms = if dur.is_nan() || dur.is_infinite() {
                            0
                        } else {
                            (dur * 1000.0) as u32
                        };
                        
                        let near_end = audio_dur_ms > 300 && local_current_f64 >= (audio_dur_ms as f64 - 300.0);
                        if local_current_f64 < audio_dur_ms as f64 && !audio.ended() && !near_end {
                            if !audio.seeking() && audio.ready_state() >= 2 && !seek_just_handled { // HAVE_CURRENT_DATA
                                let audio_time_ms = audio.current_time() * 1000.0;
                                local_current_f64 = audio_time_ms;
                                
                                if audio.paused() && !audio.ended() {
                                    let _ = audio.play();
                                }
                            } else {
                                // While seeking or buffering, hold local_current_f64 at the target position.
                                // This prevents the clock from drifting ahead of unbuffered media.
                            }
                        } else {
                            if !seek_just_handled {
                                local_current_f64 += delta;
                            }
                            if !audio.paused() {
                                let _ = audio.pause();
                            }
                        }
                    } else {
                        if !seek_just_handled {
                            local_current_f64 += delta;
                        }
                    }
                    
                    let max_dur = *bounds_ref.borrow();
                    let current_time_ms = TimeMs(local_current_f64 as u32);
                    
                    if current_time_ms.as_u32() >= max_dur.as_u32() {
                        state.dispatch(AppAction::SetTime(max_dur));
                        state.dispatch(AppAction::TogglePlay);
                    } else {
                        state.dispatch(AppAction::SetTime(current_time_ms));
                    }
                });
                
                interval_opt = Some(interval);
            } else {
                if let Some(audio) = audio_ref.cast::<web_sys::HtmlAudioElement>() {
                    let _ = audio.pause();
                }
            }
            
            move || {
                drop(interval_opt);
            }
        });
    }

    // Sync seek & Pan
    {
        let last_seek = props.state.playback.last_seek_request;
        let audio_ref = audio_ref.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        let suppress_panning = suppress_panning.clone();
        let scroll_left_state = scroll_left.clone();
        let ignore_next_scroll = ignore_next_scroll.clone();
        use_effect_with(last_seek, move |seek| {
            let suppress_panning = suppress_panning.clone();
            let ignore_next_scroll = ignore_next_scroll.clone();
            if let Some(time_ms) = seek {
                if let Some(audio) = audio_ref.cast::<HtmlAudioElement>()
                    && audio.ready_state() >= 1 {
                        audio.set_current_time(time_ms.to_secs());
                    }

                if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    let px = px_per_second.as_f64();
                    let playhead_x = time_ms.to_secs() * px;
                    let scroll_left = v.scroll_left() as f64;
                    let client_width = v.client_width() as f64;
                    let safe_zone = 90.0;
                    
                    let is_in_safe_zone = playhead_x < scroll_left + safe_zone || playhead_x > scroll_left + client_width - safe_zone;
                    let is_off_screen = playhead_x < scroll_left || playhead_x > scroll_left + client_width;

                    if is_off_screen || (is_in_safe_zone && !*suppress_panning.borrow()) {
                        *ignore_next_scroll.borrow_mut() = true;
                        let is_to_the_right = playhead_x > scroll_left + client_width - safe_zone;
                        let target_scroll = if is_to_the_right {
                            (playhead_x - safe_zone) as i32
                        } else {
                            (playhead_x - client_width / 2.0) as i32
                        };
                        v.set_scroll_left(target_scroll);
                        scroll_left_state.set(v.scroll_left() as f64);
                    }
                }
            }
            || ()
        });
    }

    // Sync playhead visual position when not animating
    {
        let current_time_ms = props.state.playback.current_time_ms;
        let playing = props.state.playback.playing;
        let drag_mode = *drag_mode;
        let playhead_ref = playhead_ref.clone();
        let px_per_second = px_per_second;

        use_effect_with((current_time_ms, playing, drag_mode, px_per_second), move |(time, playing, mode, px_per_second)| {
            if !*playing && *mode != Some(DragTarget::Playhead)
                && let Some(p) = playhead_ref.cast::<web_sys::HtmlElement>() {
                    let playhead_x = time.to_secs() * px_per_second.as_f64();
                    let _ = p.set_attribute("style", &format!("transform: translateX({}px);", playhead_x));
                }
            || ()
        });
    }

    // Synchronize scroll_left state back to the viewport DOM element
    {
        let viewport_ref = viewport_ref.clone();
        let scroll_left_val = *scroll_left;
        use_effect_with(scroll_left_val, move |&val| {
            if !val.is_nan()
                && let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>()
                    && (v.scroll_left() as f64 - val).abs() > 0.5 {
                        v.set_scroll_left(val as i32);
                    }
            || ()
        });
    }

    let on_viewport_scroll = {
        let scroll_left = scroll_left.clone();
        let viewport_width = viewport_width.clone();
        let viewport_scroll_width = viewport_scroll_width.clone();
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();
        let hover_lyrics_time = hover_lyrics_time.clone();
        let drag_create_start = drag_create_start.clone();
        let drag_create_current = drag_create_current.clone();
        let last_mouse_pos = last_mouse_pos.clone();
        let px_per_second = px_per_second;
        let state = props.state.clone();
        let ignore_next_scroll = ignore_next_scroll.clone();
        let suppress_panning = suppress_panning.clone();
        let last_user_input_time = last_user_input_time.clone();

        Callback::from(move |e: Event| {
            if let Some(viewport) = e.target_dyn_into::<web_sys::HtmlElement>() {
                if *ignore_next_scroll.borrow() {
                    *ignore_next_scroll.borrow_mut() = false;
                } else {
                    *suppress_panning.borrow_mut() = true;
                    *last_user_input_time.borrow_mut() = js_sys::Date::now();
                }

                if !*is_scrollbar_dragged.borrow() {
                    let new_scroll = viewport.scroll_left() as f64;
                    scroll_left.set(new_scroll);
                    viewport_width.set(viewport.client_width() as f64);
                    viewport_scroll_width.set(viewport.scroll_width() as f64);

                    if hover_lyrics_time.is_some() || drag_create_start.is_some() {
                        let rect = viewport.get_bounding_client_rect();
                        let mouse_x = last_mouse_pos.borrow().x;
                        if mouse_x >= rect.left() && mouse_x <= rect.right() {
                            let x = mouse_x - rect.left() + new_scroll;
                            let px = px_per_second.as_f64();
                            let current_time = TimeMs(((x / px) * 1000.0) as u32);
                            
                            if drag_create_start.is_some() {
                                let duration_ms = state.max_timeline_duration();
                                let snapped_current = crate::web_app::editor::timeline::TimelineSnapper::snap_playhead(
                                    &state,
                                    current_time,
                                    duration_ms,
                                    px_per_second,
                                );
                                drag_create_current.set(Some(snapped_current));
                            } else {
                                hover_lyrics_time.set(Some(current_time));
                            }
                        }
                    }
                }
            }
        })
    };

    let on_wheel = {
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let scroll_left_state = scroll_left.clone();
        let px_per_second_val = 92.0 * props.state.view.zoom_level.as_f64();
        let last_user_input_time = last_user_input_time.clone();
        Callback::from(move |e: WheelEvent| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            if e.ctrl_key() || e.meta_key() {
                e.prevent_default();
                if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    let rect = vp.get_bounding_client_rect();
                    let mouse_x = e.client_x() as f64 - rect.left();
                    let world_x = mouse_x + vp.scroll_left() as f64;
                    let cursor_time_s = world_x / px_per_second_val;
                    
                    let vp_width = rect.width();
                    let dur_secs = duration_ms.to_secs();
                    let min_zoom = if vp_width > 0.0 && dur_secs > 0.0 {
                        vp_width / (dur_secs * 92.0)
                    } else {
                        0.001
                    };
                    let zoom_factor = if e.delta_y() < 0.0 { 1.15 } else { 1.0 / 1.15 };
                    let new_zoom = (state.view.zoom_level.as_f64() * zoom_factor).clamp(min_zoom, 10.0);
                    let new_px_per_second = 92.0 * new_zoom;
                    
                    let new_world_x = cursor_time_s * new_px_per_second;
                    let new_scroll_left = new_world_x - mouse_x;
                    
                    state.dispatch(AppAction::SetZoom(ZoomLevel(new_zoom)));
                    vp.set_scroll_left(new_scroll_left as i32);
                    scroll_left_state.set(vp.scroll_left() as f64);
                }
            } else {
                if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    e.prevent_default();
                    let delta = e.delta_y() + e.delta_x();
                    let new_scroll = vp.scroll_left() as f64 + delta;
                    vp.set_scroll_left(new_scroll as i32);
                    scroll_left_state.set(vp.scroll_left() as f64);
                }
            }
        })
    };

    let update_scrollbar = {
        let scroll_left = scroll_left.clone();
        let viewport_width = viewport_width.clone();
        let viewport_scroll_width = viewport_scroll_width.clone();
        let last_user_input_time = last_user_input_time.clone();
        std::rc::Rc::new(move |vp: &web_sys::HtmlElement, track: &web_sys::HtmlElement, handle: &web_sys::HtmlElement, client_x: f64, handle_offset: f64| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            let track_rect = track.get_bounding_client_rect();
            let track_left = track_rect.left();
            let track_width = track_rect.width();

            let vp_width = vp.client_width() as f64;
            let vp_scroll_width = vp.scroll_width() as f64;
            viewport_width.set(vp_width);
            viewport_scroll_width.set(vp_scroll_width);

            // Calculate expected handle width to avoid using stale DOM width
            let expected_handle_width = (track_width * (vp_width / vp_scroll_width).min(1.0)).max(20.0);
            let handle_width = expected_handle_width;

            let track_scrollable_width = track_width - handle_width;
            let viewport_scrollable_width = (vp_scroll_width - vp_width).max(0.0);

            let mouse_x = client_x - track_left;
            let target_handle_left = mouse_x - handle_offset;
            let clamped_handle_left = target_handle_left.clamp(0.0, track_scrollable_width);

            let target_scroll = if track_scrollable_width > 0.0 {
                (clamped_handle_left / track_scrollable_width) * viewport_scrollable_width
            } else {
                0.0
            };

            vp.set_scroll_left(target_scroll as i32);
            scroll_left.set(target_scroll);

            let _ = handle.set_attribute("style", &format!(
                "width: max(20px, {}%); left: {}px;",
                (vp_width / vp_scroll_width).min(1.0) * 100.0,
                clamped_handle_left
            ));

            clamped_handle_left
        })
    };

    let on_scrollbar_mousedown = {
        let viewport_ref = viewport_ref.clone();
        let viewport_width = viewport_width.clone();
        let viewport_scroll_width = viewport_scroll_width.clone();
        let drag_mode = drag_mode.clone();
        let drag_scrollbar_track_left = drag_scrollbar_track_left.clone();
        let drag_scrollbar_track_width = drag_scrollbar_track_width.clone();
        let drag_scrollbar_handle_offset = drag_scrollbar_handle_offset.clone();
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();
        let update_scrollbar = update_scrollbar.clone();
        let scrollbar_track_ref = scrollbar_track_ref.clone();
        let scrollbar_handle_ref = scrollbar_handle_ref.clone();

        Callback::from(move |e: MouseEvent| {
            if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let track_element = if let Some(track) = scrollbar_track_ref.cast::<web_sys::HtmlElement>() {
                    track
                } else {
                    return;
                };

                let rect = track_element.get_bounding_client_rect();
                let track_left = rect.left();
                let track_width = rect.width();
                *drag_scrollbar_track_left.borrow_mut() = track_left;
                *drag_scrollbar_track_width.borrow_mut() = track_width;
                
                let target = e.target().unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
                let is_handle = target.class_list().contains("custom-scrollbar-handle");

                let vp_width = vp.client_width() as f64;
                let vp_scroll_width = vp.scroll_width() as f64;
                viewport_width.set(vp_width);
                viewport_scroll_width.set(vp_scroll_width);

                if is_handle {
                    e.prevent_default();
                    let handle_rect = target.get_bounding_client_rect();
                    *drag_scrollbar_handle_offset.borrow_mut() = e.client_x() as f64 - handle_rect.left();
                    *is_scrollbar_dragged.borrow_mut() = true;
                    drag_mode.set(Some(DragTarget::Scrollbar));
                    e.stop_propagation();
                } else {
                    e.prevent_default();
                    if let Some(handle) = scrollbar_handle_ref.cast::<web_sys::HtmlElement>() {
                        
                        let expected_handle_width = (track_width * (vp_width / vp_scroll_width).min(1.0)).max(20.0);
                        let initial_offset = expected_handle_width / 2.0;
                        
                        *is_scrollbar_dragged.borrow_mut() = true;
                        drag_mode.set(Some(DragTarget::Scrollbar));

                        let clamped_left = update_scrollbar(&vp, &track_element, &handle, e.client_x() as f64, initial_offset);

                        let click_x = e.client_x() as f64 - track_left;
                        let actual_handle_offset = click_x - clamped_left;
                        *drag_scrollbar_handle_offset.borrow_mut() = actual_handle_offset;
                        e.stop_propagation();
                    }
                }
            }
        })
    };

    {
        let viewport_ref = viewport_ref.clone();
        let viewport_width = viewport_width.clone();
        let viewport_scroll_width = viewport_scroll_width.clone();
        use_effect_with((), move |_| {
            let vw_clone = viewport_width.clone();
            let vsw_clone = viewport_scroll_width.clone();
            let vr_clone = viewport_ref.clone();
            let listener = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                if let Some(v) = vr_clone.cast::<web_sys::HtmlElement>() {
                    vw_clone.set(v.client_width() as f64);
                    vsw_clone.set(v.scroll_width() as f64);
                }
            }) as Box<dyn FnMut()>);
            
            let window = web_sys::window().unwrap();
            window.add_event_listener_with_callback("resize", listener.as_ref().unchecked_ref()).unwrap();
            
            if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                viewport_width.set(v.client_width() as f64);
                viewport_scroll_width.set(v.scroll_width() as f64);
            }
            
            move || {
                window.remove_event_listener_with_callback("resize", listener.as_ref().unchecked_ref()).unwrap();
            }
        });
    }

    // Global keyboard shortcuts (Delete/Backspace to delete, Esc to deselect)
    {
        let state = props.state.clone();
        let last_user_input_time = last_user_input_time.clone();
        use_effect_with((), move |_| {
            let state_clone = state.clone();
            let last_user_input_time = last_user_input_time.clone();
            let listener = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                // Ignore if currently typing in an input, textarea, or contenteditable element
                let document = web_sys::window().unwrap().document().unwrap();
                if let Some(active_element) = document.active_element() {
                    let tag_name = active_element.tag_name().to_uppercase();
                    if tag_name == "INPUT" || tag_name == "TEXTAREA" || active_element.has_attribute("contenteditable") {
                        return;
                    }
                }

                *last_user_input_time.borrow_mut() = js_sys::Date::now();

                let key = e.key();
                if key == "Delete" || key == "Backspace" {
                    state_clone.dispatch(AppAction::DeleteSelected);
                } else if key == "Escape" {
                    state_clone.dispatch(AppAction::ClearSelection);
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            let window = web_sys::window().unwrap();
            window.add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref()).unwrap();

            move || {
                window.remove_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref()).unwrap();
            }
        });
    }

    // Keep viewport dimensions updated when layout constraints change (zoom/duration)
    {
        let viewport_ref = viewport_ref.clone();
        let viewport_width = viewport_width.clone();
        let viewport_scroll_width = viewport_scroll_width.clone();
        let zoom_level = props.state.view.zoom_level.as_f64();
        let duration = props.state.max_timeline_duration();
        use_effect_with((zoom_level, duration), move |_| {
            if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                viewport_width.set(v.client_width() as f64);
                viewport_scroll_width.set(v.scroll_width() as f64);
            }
            || ()
        });
    }

    // Auto-clamp zoom level if it is less than the dynamic min_zoom to fit the timeline
    {
        let state = props.state.clone();
        let zoom_level = props.state.view.zoom_level.as_f64();
        let duration = props.state.max_timeline_duration();
        let viewport_width_val = *viewport_width;
        use_effect_with((zoom_level, duration.as_u32(), viewport_width_val), move |(zl, dur_u32, vw)| {
            let vw_val = *vw;
            if vw_val > 0.0 {
                let dur_secs = *dur_u32 as f64 / 1000.0;
                if dur_secs > 0.0 {
                    let min_zoom = vw_val / (dur_secs * 92.0);
                    if *zl < min_zoom - 0.0001 {
                        state.dispatch(AppAction::SetZoom(ZoomLevel(min_zoom)));
                    }
                }
            }
            || ()
        });
    }

    let toggle_play = {
        let state = props.state.clone();
        Callback::from(move |_| {
            state.dispatch(AppAction::TogglePlay);
        })
    };

    let zoom_handlers = crate::web_app::components::timeline::hooks::zoom_handlers::use_zoom_handlers(
        props.state.clone(),
        viewport_ref.clone(),
        scroll_left.clone(),
        viewport_width.clone(),
        last_user_input_time.clone(),
    );
    let zoom_in = zoom_handlers.zoom_in;
    let zoom_out = zoom_handlers.zoom_out;

    // Smooth playhead & auto pan
    crate::web_app::components::timeline::hooks::playback_sync::use_playback_sync(
        props.state.clone(),
        drag_mode.clone(),
        playhead_ref.clone(),
        viewport_ref.clone(),
        timecode_ref.clone(),
        is_scrollbar_dragged.clone(),
        last_mouse_pos.clone(),
        suppress_panning.clone(),
        scroll_left.clone(),
        ignore_next_scroll.clone(),
        current_time_ms_ref.clone(),
        px_per_second_ref.clone(),
        last_user_input_time.clone(),
    );

    let on_timeline_mousedown = {
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_start_y = drag_start_y.clone();
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let last_mouse_pos = last_mouse_pos.clone();
        let last_user_input_time = last_user_input_time.clone();
        
        Callback::from(move |e: MouseEvent| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let _ = viewport.focus();
                
                if !e.ctrl_key() && !e.meta_key() && !e.shift_key() {
                    state.dispatch(AppAction::ClearSelection);
                }
                
                let rect = viewport.get_bounding_client_rect();
                let x = e.client_x() as f64 - rect.left() + viewport.scroll_left() as f64;
                let y = e.client_y() as f64 - rect.top();
                
                drag_mode.set(Some(DragTarget::Selection));
                *drag_start_x.borrow_mut() = x;
                drag_start_y.set(Pixels(y));
                *last_mouse_pos.borrow_mut() = crate::domain::Vec2 { x: e.client_x() as f64, y: e.client_y() as f64 };
            }
        })
    };
 
    let on_ruler_mousedown = {
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        let last_mouse_pos = last_mouse_pos.clone();
        let last_user_input_time = last_user_input_time.clone();
        
        Callback::from(move |e: MouseEvent| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            e.stop_propagation();
            drag_mode.set(Some(DragTarget::Playhead));
            *drag_start_x.borrow_mut() = e.client_x() as f64;
            *last_mouse_pos.borrow_mut() = crate::domain::Vec2 { x: e.client_x() as f64, y: e.client_y() as f64 };
            
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let rect = viewport.get_bounding_client_rect();
                let x = e.client_x() as f64 - rect.left() + viewport.scroll_left() as f64;
                let target_time_ms = (x / px_per_second.as_f64() * 1000.0) as i32;
                state.dispatch(AppAction::SetTime(TimeMs(target_time_ms.max(0) as u32)));
            }
        })
    };
 
    let on_keydown = {
        let state = props.state.clone();
        let last_user_input_time = last_user_input_time.clone();
        Callback::from(move |e: KeyboardEvent| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            if e.key() == "Delete" || e.key() == "Backspace" {
                state.dispatch(AppAction::DeleteSelected);
            }
        })
    };
 
    let on_mousedown_lyrics = {
        let drag_mode = drag_mode.clone();
        let drag_create_start = drag_create_start.clone();
        let drag_create_current = drag_create_current.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        let state = props.state.clone();
        let last_user_input_time = last_user_input_time.clone();
        
        Callback::from(move |e: MouseEvent| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            e.stop_propagation();
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let _ = viewport.focus();
                let rect = viewport.get_bounding_client_rect();
                let x = e.client_x() as f64 - rect.left() + viewport.scroll_left() as f64;
                let clicked_time = TimeMs(((x / px_per_second.as_f64()) * 1000.0) as u32);
                let duration_ms = state.max_timeline_duration();
                let snapped_start = crate::web_app::editor::timeline::TimelineSnapper::snap_playhead(
                    &state,
                    clicked_time,
                    duration_ms,
                    px_per_second,
                );
                drag_mode.set(Some(DragTarget::CreateChunk));
                drag_create_start.set(Some(snapped_start));
                drag_create_current.set(Some(snapped_start));
            }
        })
    };

    let on_mousemove_lyrics = {
        let drag_mode = drag_mode.clone();
        let drag_create_current = drag_create_current.clone();
        let hover_lyrics_time = hover_lyrics_time.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        let state = props.state.clone();
        let last_mouse_pos = last_mouse_pos.clone();
        
        Callback::from(move |e: MouseEvent| {
            *last_mouse_pos.borrow_mut() = crate::domain::Vec2 { x: e.client_x() as f64, y: e.client_y() as f64 };
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let rect = viewport.get_bounding_client_rect();
                let x = e.client_x() as f64 - rect.left() + viewport.scroll_left() as f64;
                let current_time = TimeMs(((x / px_per_second.as_f64()) * 1000.0) as u32);
                
                if *drag_mode == Some(DragTarget::CreateChunk) {
                    let duration_ms = state.max_timeline_duration();
                    let snapped_current = crate::web_app::editor::timeline::TimelineSnapper::snap_playhead(
                        &state,
                        current_time,
                        duration_ms,
                        px_per_second,
                    );
                    drag_create_current.set(Some(snapped_current));
                } else {
                    hover_lyrics_time.set(Some(current_time));
                }
            }
        })
    };

    let on_mouseleave_lyrics = {
        let hover_lyrics_time = hover_lyrics_time.clone();
        Callback::from(move |_| {
            hover_lyrics_time.set(None);
        })
    };

    let on_mousemove = {
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_start_y = drag_start_y.clone();
        let selection_rect = selection_rect.clone();
        let drag_offset_ms = drag_offset_ms.clone();
        let drag_target_uid = drag_target_uid.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second_ref = px_per_second_ref.clone();
        let last_mouse_pos = last_mouse_pos.clone();
        let state = props.state.clone();
        let drag_create_current = drag_create_current.clone();
        let last_user_input_time = last_user_input_time.clone();

        let drag_scrollbar_handle_offset = drag_scrollbar_handle_offset.clone();
        let update_scrollbar = update_scrollbar.clone();
        let scrollbar_track_ref = scrollbar_track_ref.clone();
        let scrollbar_handle_ref = scrollbar_handle_ref.clone();

        Callback::from(move |e: MouseEvent| {
            *last_mouse_pos.borrow_mut() = crate::domain::Vec2 { x: e.client_x() as f64, y: e.client_y() as f64 };
            
            if let Some(mode) = *drag_mode {
                *last_user_input_time.borrow_mut() = js_sys::Date::now();
                let delta_x = e.client_x() as f64 - *drag_start_x.borrow();
                
                if mode == DragTarget::Playhead {
                    // Handled inside playback_sync.rs under request_animation_frame
                } else if mode == DragTarget::Selection {
                    if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                        let rect = v.get_bounding_client_rect();
                        let current_x = e.client_x() as f64 - rect.left() + v.scroll_left() as f64;
                        let current_y = e.client_y() as f64 - rect.top();
                        
                        let start_x = *drag_start_x.borrow();
                        let start_y = drag_start_y.as_f64();
                        
                        let x = start_x.min(current_x);
                        let y = start_y.min(current_y);
                        let w = (current_x - start_x).abs();
                        let h = (current_y - start_y).abs();
                        
                        selection_rect.set(Some(crate::domain::Rect { x, y, w, h }));
                    }
                } else if mode == DragTarget::Scrollbar {
                    if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                        let track_el = scrollbar_track_ref.cast::<web_sys::HtmlElement>();
                        let handle_el = scrollbar_handle_ref.cast::<web_sys::HtmlElement>();

                        if let (Some(track), Some(handle)) = (track_el, handle_el) {
                            let handle_offset = *drag_scrollbar_handle_offset.borrow();
                            update_scrollbar(&v, &track, &handle, e.client_x() as f64, handle_offset);
                        }
                    }
                } else if mode == DragTarget::CreateChunk {
                    if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                        let rect = v.get_bounding_client_rect();
                        let x = e.client_x() as f64 - rect.left() + v.scroll_left() as f64;
                        let current_time = TimeMs(((x / px_per_second_ref.borrow().as_f64()) * 1000.0) as u32);
                        let duration_ms = state.max_timeline_duration();
                        let snapped_current = crate::web_app::editor::timeline::TimelineSnapper::snap_playhead(
                            &state,
                            current_time,
                            duration_ms,
                            *px_per_second_ref.borrow(),
                        );
                        drag_create_current.set(Some(snapped_current));
                    }
                } else {
                    let px = px_per_second_ref.borrow().as_f64();
                    let delta_ms = (delta_x / px * 1000.0) as i32;
                    let duration_ms = state.max_timeline_duration();
                    let snapped_offset = crate::web_app::editor::timeline::TimelineSnapper::snap_drag_offset(
                        &state,
                        mode,
                        *drag_target_uid,
                        delta_ms,
                        duration_ms,
                        *px_per_second_ref.borrow(),
                    );
                    drag_offset_ms.set(snapped_offset);
                }
            }
        })
    };

    let on_global_mouseup = {
        let drag_mode = drag_mode.clone();
        let drag_offset_ms = drag_offset_ms.clone();
        let drag_target_uid = drag_target_uid.clone();
        let selection_rect = selection_rect.clone();
        let state = props.state.clone();
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();
        let px_per_second = px_per_second;
        let suppress_panning = suppress_panning.clone();
        let current_time_ms_ref = current_time_ms_ref.clone();
        let drag_create_start = drag_create_start.clone();
        let drag_create_current = drag_create_current.clone();
        let hover_lyrics_time = hover_lyrics_time.clone();
        let px_per_second_ref = px_per_second_ref.clone();
        let last_user_input_time = last_user_input_time.clone();
        
        Callback::from(move |e: MouseEvent| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            let is_dragged = *is_scrollbar_dragged.borrow();
            if is_dragged {
                let is_scrollbar_dragged_clone = is_scrollbar_dragged.clone();
                gloo_timers::callback::Timeout::new(50, move || {
                    *is_scrollbar_dragged_clone.borrow_mut() = false;
                }).forget();
            } else {
                *is_scrollbar_dragged.borrow_mut() = false;
            }
            
            if let Some(mode) = *drag_mode {
                match mode {
                    DragTarget::Body => {
                        let offset = *drag_offset_ms;
                        if offset != 0 {
                            state.dispatch(AppAction::ShiftSelected(offset));
                        }
                    }
                    DragTarget::LeftEdge => {
                        let offset = *drag_offset_ms;
                        if offset != 0
                            && let Some(uid) = *drag_target_uid {
                                state.dispatch(AppAction::ShiftBoundary(uid, true, false, offset));
                            }
                    }
                    DragTarget::RightEdge => {
                        let offset = *drag_offset_ms;
                        if offset != 0
                            && let Some(uid) = *drag_target_uid {
                                state.dispatch(AppAction::ShiftBoundary(uid, false, false, offset));
                            }
                    }
                    DragTarget::Boundary => {
                        let offset = *drag_offset_ms;
                        if offset != 0
                            && let Some(uid) = *drag_target_uid {
                                state.dispatch(AppAction::ShiftBoundary(uid, false, true, offset));
                            }
                    }
                    DragTarget::Playhead => {
                        *suppress_panning.borrow_mut() = true;
                        let latest_time = *current_time_ms_ref.borrow();
                        state.dispatch(AppAction::Seek(latest_time));
                    }
                    DragTarget::Selection => {
                        if let Some(rect) = *selection_rect {
                            let x = rect.x;
                            let w = rect.w;
                            if let Some(doc) = &state.document.document {
                                let duration = state.max_timeline_duration();
                                let start_time = TimeMs(((x / px_per_second.as_f64()) * 1000.0) as u32);
                                let end_time = TimeMs((((x + w) / px_per_second.as_f64()) * 1000.0) as u32);
                                
                                let mode = if e.shift_key() {
                                    crate::domain::SelectionMode::Add
                                } else if e.ctrl_key() || e.meta_key() {
                                    crate::domain::SelectionMode::Add
                                } else {
                                    crate::domain::SelectionMode::Replace
                                };
 
                                let chunks = doc.timeline_chunks(duration);
                                let mut first = true;
                                for chunk in chunks {
                                    if !chunk.is_empty() {
                                        let c_start = chunk.start_ms();
                                        let c_end = chunk.end_ms();
                                        
                                        if c_start < end_time && c_end > start_time {
                                            let chunk_mode = if mode == crate::domain::SelectionMode::Replace {
                                                if first { crate::domain::SelectionMode::Replace } else { crate::domain::SelectionMode::Add }
                                            } else {
                                                mode
                                            };
                                            state.dispatch(AppAction::SelectEntry(chunk.uid(), chunk_mode));
                                            first = false;
                                        }
                                    }
                                }
                                
                                if mode == crate::domain::SelectionMode::Replace && first {
                                    state.dispatch(AppAction::ClearSelection);
                                }
                            }
                        }
                        selection_rect.set(None);
                    }
                    DragTarget::Scrollbar => {}
                    DragTarget::CreateChunk => {
                        let start_opt = *drag_create_start;
                        let current_opt = *drag_create_current;
                        
                        if let (Some(start), Some(current)) = (start_opt, current_opt) {
                            if start == current {
                                if let Some(hover_t) = *hover_lyrics_time {
                                    let doc = state.document.document.as_ref();
                                    let duration_ms = state.max_timeline_duration();
                                    let gap = crate::web_app::editor::timeline::find_gap(doc, hover_t, duration_ms);
                                    if let Some((gap_start, gap_end)) = gap {
                                        let (ghost_start, ghost_end) = crate::web_app::editor::timeline::calculate_ghost_chunk(
                                            &state,
                                            hover_t,
                                            gap_start,
                                            gap_end,
                                            duration_ms,
                                            *px_per_second_ref.borrow(),
                                        );
                                        state.dispatch(AppAction::AddChunk(ghost_start, ghost_end));
                                    }
                                }
                            } else {
                                let start_t = start.min(current);
                                let end_t = start.max(current);
                                state.dispatch(AppAction::AddChunk(start_t, end_t));
                            }
                        }
                        drag_create_start.set(None);
                        drag_create_current.set(None);
                    }
                }
                drag_mode.set(None);
                drag_offset_ms.set(0);
                drag_target_uid.set(None);
            }
        })
    };

    // Attach global window-level listeners when a drag is active
    {
        let drag_mode = drag_mode.clone();
        let on_mousemove = on_mousemove.clone();
        let on_global_mouseup = on_global_mouseup.clone();

        use_effect_with(*drag_mode, move |mode| {
            let mut mousemove_cb_opt = None;
            let mut mouseup_cb_opt = None;

            if mode.is_some() {
                let window = web_sys::window().unwrap();

                let on_mousemove_clone = on_mousemove.clone();
                let mousemove_cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    on_mousemove_clone.emit(e);
                }) as Box<dyn FnMut(web_sys::MouseEvent)>);

                let on_global_mouseup_clone = on_global_mouseup.clone();
                let mouseup_cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    on_global_mouseup_clone.emit(e);
                }) as Box<dyn FnMut(web_sys::MouseEvent)>);

                window.add_event_listener_with_callback("mousemove", mousemove_cb.as_ref().unchecked_ref()).unwrap();
                window.add_event_listener_with_callback("mouseup", mouseup_cb.as_ref().unchecked_ref()).unwrap();

                mousemove_cb_opt = Some(mousemove_cb);
                mouseup_cb_opt = Some(mouseup_cb);
            }

            move || {
                if let (Some(mm_cb), Some(mu_cb)) = (mousemove_cb_opt, mouseup_cb_opt) {
                    let window = web_sys::window().unwrap();
                    let _ = window.remove_event_listener_with_callback("mousemove", mm_cb.as_ref().unchecked_ref());
                    let _ = window.remove_event_listener_with_callback("mouseup", mu_cb.as_ref().unchecked_ref());
                }
            }
        });
    }

    let on_chunk_drag_start = {
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_target_uid = drag_target_uid.clone();
        let viewport_ref = viewport_ref.clone();
        let last_user_input_time = last_user_input_time.clone();
        Callback::from(move |(uid, e, target): (usize, MouseEvent, DragTarget)| {
            *last_user_input_time.borrow_mut() = js_sys::Date::now();
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let _ = viewport.focus();
            }
            drag_mode.set(Some(target));
            *drag_start_x.borrow_mut() = e.client_x() as f64;
            drag_target_uid.set(Some(uid));
        })
    };

    html! {
        <div class="panel timeline-panel" onmouseup={on_global_mouseup.clone()}>
            <input 
                type="file" 
                accept="audio/*" 
                ref={file_input_ref} 
                style="display: none;" 
                onchange={on_file_change} 
            />
            <input 
                type="file" 
                accept=".lrc,.txt" 
                ref={lrc_input_ref} 
                style="display: none;" 
                onchange={on_lrc_change} 
            />
            {
                if let Some(url) = &*audio_url {
                    html! {
                        <audio 
                            ref={audio_ref} 
                            src={url.clone()} 
                            onloadedmetadata={on_loaded_metadata}
                        />
                    }
                } else {
                    html! {}
                }
            }
            <PlaybackControls 
                state={props.state.clone()}
                timecode_ref={timecode_ref}
                scrollbar_track_ref={scrollbar_track_ref}
                scrollbar_handle_ref={scrollbar_handle_ref}
                on_toggle_play={toggle_play}
                on_zoom_in={zoom_in}
                on_zoom_out={zoom_out}
                scroll_left={*scroll_left}
                viewport_width={*viewport_width}
                total_width={width_px.as_f64()}
                on_scrollbar_mousedown={on_scrollbar_mousedown}
            />
            <div class="timeline-body">
                <TrackPads 
                    on_import_audio={import_click.clone()}
                    on_import_lrc={import_lrc_click}
                    on_export_lrc={export_lrc}
                />
                <TimelineLanes 
                    state={props.state.clone()}
                    viewport_ref={viewport_ref.clone()}
                    canvas_ref={canvas_ref}
                    playhead_ref={playhead_ref.clone()}
                    audio_url={(*audio_url).clone()}
                    waveform_summary={(*waveform_summary).clone()}
                    scroll_left={*scroll_left}
                    viewport_width={*viewport_width}
                    duration_ms={duration_ms}
                    width_px={width_px}
                    audio_width_px={audio_width_px}
                    px_per_second={px_per_second}
                    drag_mode={*drag_mode}
                    drag_offset_ms={*drag_offset_ms}
                    drag_target_id={*drag_target_uid}
                    on_viewport_scroll={on_viewport_scroll}
                    on_wheel={on_wheel}
                    on_keydown={on_keydown}
                    on_mousemove={on_mousemove}
                    on_mousedown_content={on_timeline_mousedown}
                    on_mousedown_ruler={on_ruler_mousedown}
                    on_import_audio={import_click}
                    on_chunk_drag_start={on_chunk_drag_start}
                    selection_rect={*selection_rect}
                    hover_lyrics_time={*hover_lyrics_time}
                    drag_create_start={*drag_create_start}
                    drag_create_current={*drag_create_current}
                    on_mousedown_lyrics={on_mousedown_lyrics}
                    on_mousemove_lyrics={on_mousemove_lyrics}
                    on_mouseleave_lyrics={on_mouseleave_lyrics}
                />
            </div>
        </div>
    }
}

pub fn downsample_audio(audio_buffer: web_sys::AudioBuffer) -> WaveformSummary {
    let sample_rate = audio_buffer.sample_rate();
    let data = audio_buffer.get_channel_data(0).unwrap_or_else(|_| vec![0.0]);
    let bins_per_second = 200;
    let samples_per_bin = (sample_rate as f64 / bins_per_second as f64).max(1.0) as usize;
    let mut bins = Vec::with_capacity(data.len() / samples_per_bin);
    for i in (0..data.len()).step_by(samples_per_bin) {
        let mut min = 1.0f32;
        let mut max = -1.0f32;
        let end = (i + samples_per_bin).min(data.len());
        for j in i..end {
            let val = data[j];
            if val < min { min = val; }
            if val > max { max = val; }
        }
        bins.push((min, max));
    }
    WaveformSummary { bins, samples_per_bin, sample_rate }
}
