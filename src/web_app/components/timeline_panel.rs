use yew::prelude::*;
use web_sys::{HtmlAudioElement, HtmlInputElement, Url};
use wasm_bindgen::JsCast;
use crate::web_app::actions::{AppState, AppAction};
use crate::domain::{TimeMs, Pixels};
use super::timeline::{PlaybackControls, TrackPads, TimelineLanes, DragTarget};

#[derive(Properties, PartialEq)]
pub struct TimelinePanelProps {
    pub state: UseReducerHandle<AppState>,
}

#[function_component(TimelinePanel)]
pub fn timeline_panel(props: &TimelinePanelProps) -> Html {
    let px_per_second = Pixels(92.0 * props.state.zoom_level);
    
    let audio_ref = use_node_ref();
    let file_input_ref = use_node_ref();
    let canvas_ref = use_node_ref();
    let viewport_ref = use_node_ref();
    let scrollbar_ref = use_node_ref();
    let playhead_ref = use_node_ref();
    let timecode_ref = use_node_ref();
    
    let audio_url = use_state(|| None::<String>);

    let drag_mode = use_state(|| None::<DragTarget>);
    let drag_start_x = use_state(|| Pixels(0.0));
    let drag_offset_ms = use_state(|| 0i32);
    let drag_target_id = use_state(|| None::<usize>);
    let is_scrollbar_dragged = use_mut_ref(|| false);
    let last_mouse_pos = use_mut_ref(|| (0.0, 0.0));

    let on_file_change = {
        let audio_url = audio_url.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    if let Ok(url) = Url::create_object_url_with_blob(&file) {
                        audio_url.set(Some(url));
                    }
                }
            }
        })
    };

    let import_click = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_loaded_metadata = {
        let state = props.state.clone();
        Callback::from(move |e: Event| {
            if let Some(audio) = e.target_dyn_into::<web_sys::HtmlAudioElement>() {
                state.dispatch(AppAction::SetDuration(TimeMs((audio.duration() * 1000.0) as u32)));
            }
        })
    };

    let pan_velocity = use_mut_ref(|| 0.0);
    
    // Keep live references for the RAF loop to avoid stale closures
    let current_time_ms_ref = use_mut_ref(|| props.state.current_time_ms);
    *current_time_ms_ref.borrow_mut() = props.state.current_time_ms;

    // Drag & Pan loop
    {
        let pan_velocity = pan_velocity.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        let drag_mode = drag_mode.clone();
        let state = props.state.clone();
        let drag_offset_ms = drag_offset_ms.clone();
        let last_mouse_pos = last_mouse_pos.clone();
        let current_time_ms_ref = current_time_ms_ref.clone();

        use_effect(move || {
            let interval = gloo_timers::callback::Interval::new(16, move || {
                let vel = *pan_velocity.borrow();
                let mode = *drag_mode;
                
                if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    let old_scroll = v.scroll_left();
                    if vel != 0.0 && mode.is_some() {
                        v.set_scroll_left(old_scroll + vel as i32);
                    }
                    let actual_delta = v.scroll_left() - old_scroll;
                    
                    if mode == Some(DragTarget::Playhead) {
                        let rect = v.get_bounding_client_rect();
                        let (mouse_x, _) = *last_mouse_pos.borrow();
                        let absolute_x = mouse_x - rect.left() + v.scroll_left() as f64;
                        let target_time_ms = (absolute_x / px_per_second.as_f64() * 1000.0) as i32;
                        let new_time = TimeMs(target_time_ms.max(0) as u32);
                        
                        *current_time_ms_ref.borrow_mut() = new_time;
                        state.dispatch(AppAction::SetTime(new_time));
                    } else if mode.is_some() && actual_delta != 0 {
                        let delta_ms = (actual_delta as f64 / px_per_second.as_f64() * 1000.0) as i32;
                        drag_offset_ms.set(*drag_offset_ms + delta_ms);
                    }
                }
            });
            move || drop(interval)
        });
    }

    // Playback loop effect
    {
        let playing = props.state.playing;
        let audio_ref = audio_ref.clone();
        let state = props.state.clone();
        
        let current_time_ref = yew::use_mut_ref(|| props.state.current_time_ms);
        *current_time_ref.borrow_mut() = props.state.current_time_ms;

        let bounds_ref = yew::use_mut_ref(|| (TimeMs(0), TimeMs(0)));
        *bounds_ref.borrow_mut() = {
            let audio_dur = props.state.duration_ms;
            let last_nonempty = props.state.document.as_ref().and_then(|d| {
                d.entries().iter().rev().find(|e| !e.is_empty()).map(|e| e.time_ms())
            }).unwrap_or(TimeMs(0));
            
            let last_lyric = props.state.document.as_ref().and_then(|d| d.last_entry_time_ms()).unwrap_or(TimeMs(0));
            let timeline_dur = TimeMs(audio_dur.as_u32().max(last_lyric.as_u32()) + 10000);
            
            let base_max = if last_nonempty.as_u32() > audio_dur.as_u32() {
                TimeMs(last_nonempty.as_u32() + 10000)
            } else {
                audio_dur
            };
            (base_max, timeline_dur)
        };

        let last_seek_ref = yew::use_mut_ref(|| props.state.last_seek_request);
        *last_seek_ref.borrow_mut() = props.state.last_seek_request;

        use_effect_with(playing, move |playing| {
            let mut interval_opt = None;
            
            if *playing {
                let start_time = *current_time_ref.borrow();
                
                if let Some(audio) = audio_ref.cast::<web_sys::HtmlAudioElement>() {
                    let audio_dur_ms = (audio.duration() * 1000.0) as u32;
                    if start_time.as_u32() < audio_dur_ms {
                        let _ = audio.play();
                    } else {
                        // Ensure audio is paused if we are past duration
                        let _ = audio.pause();
                    }
                }
                
                let last_time = js_sys::Date::now();
                let last_time_ref = std::rc::Rc::new(std::cell::Cell::new(last_time));
                
                let mut local_current = start_time;
                let mut last_handled_seek = *last_seek_ref.borrow();
                
                let interval = gloo_timers::callback::Interval::new(16, move || {
                    let now = js_sys::Date::now();
                    let delta = now - last_time_ref.get();
                    last_time_ref.set(now);
                    
                    let current_seek = *last_seek_ref.borrow();
                    if current_seek != last_handled_seek {
                        last_handled_seek = current_seek;
                        if let Some(seek_time) = current_seek {
                            local_current = seek_time;
                        }
                    }
                    
                    let mut synced = false;
                    if let Some(audio) = audio_ref.cast::<web_sys::HtmlAudioElement>() {
                        let audio_dur_ms = (audio.duration() * 1000.0) as u32;
                        if local_current.as_u32() < audio_dur_ms && !audio.paused() && !audio.ended() {
                            local_current = TimeMs((audio.current_time() * 1000.0) as u32);
                            synced = true;
                        } else if local_current.as_u32() < audio_dur_ms && audio.paused() {
                            // If we are before audio end but it's paused (e.g. buffering or just started)
                            // We should probably wait for it or just drift. 
                            // For now, let's just drift to keep it simple, but try to play if possible.
                            let _ = audio.play();
                        }
                    }
                    
                    if !synced {
                        local_current = TimeMs(local_current.as_u32() + delta as u32);
                    }
                    
                    let (base_max, timeline_dur) = *bounds_ref.borrow();
                    let max_dur = if start_time.as_u32() >= base_max.as_u32() {
                        timeline_dur
                    } else {
                        base_max
                    };
                    
                    if local_current.as_u32() >= max_dur.as_u32() {
                        local_current = max_dur;
                        state.dispatch(AppAction::SetTime(local_current));
                        state.dispatch(AppAction::TogglePlay);
                    } else {
                        state.dispatch(AppAction::SetTime(local_current));
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

    // Sync seek
    {
        let last_seek = props.state.last_seek_request;
        let audio_ref = audio_ref.clone();
        use_effect_with(last_seek, move |seek| {
            if let Some(time_ms) = seek {
                if let Some(audio) = audio_ref.cast::<HtmlAudioElement>() {
                    audio.set_current_time(time_ms.to_secs());
                }
            }
            || ()
        });
    }

    // Pan to playhead on seek (when paused)
    {
        let current_time_ms = props.state.current_time_ms;
        let playing = props.state.playing;
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        use_effect_with((current_time_ms, playing), move |(time, playing)| {
            if !*playing {
                if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    let px = px_per_second.as_f64();
                    let playhead_x = time.to_secs() * px;
                    let scroll_left = v.scroll_left() as f64;
                    let client_width = v.client_width() as f64;
                    if playhead_x < scroll_left || playhead_x > scroll_left + client_width {
                        v.set_scroll_left((playhead_x - client_width / 2.0) as i32);
                    }
                }
            }
            || ()
        });
    }

    let last_lyric_ms = props.state.document.as_ref().and_then(|doc| doc.last_entry_time_ms()).unwrap_or(TimeMs(0));
    let duration_ms = TimeMs(props.state.duration_ms.as_u32().max(last_lyric_ms.as_u32()) + 10000);
    let width_px = Pixels(duration_ms.to_secs() * px_per_second.as_f64());
    let audio_width_px = Pixels(props.state.duration_ms.to_secs() * px_per_second.as_f64());

    let on_viewport_scroll = {
        let scrollbar_ref = scrollbar_ref.clone();
        Callback::from(move |e: Event| {
            if let Some(viewport) = e.target_dyn_into::<web_sys::HtmlElement>() {
                if let Some(scrollbar) = scrollbar_ref.cast::<web_sys::HtmlElement>() {
                    if (scrollbar.scroll_left() - viewport.scroll_left()).abs() > 1 {
                        scrollbar.set_scroll_left(viewport.scroll_left());
                    }
                }
            }
        })
    };

    let on_scrollbar_scroll = {
        let viewport_ref = viewport_ref.clone();
        Callback::from(move |e: Event| {
            if let Some(scrollbar) = e.target_dyn_into::<web_sys::HtmlElement>() {
                if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    if (viewport.scroll_left() - scrollbar.scroll_left()).abs() > 1 {
                        viewport.set_scroll_left(scrollbar.scroll_left());
                    }
                }
            }
        })
    };

    let toggle_play = {
        let state = props.state.clone();
        Callback::from(move |_| {
            state.dispatch(AppAction::TogglePlay);
        })
    };

    let select_all = {
        let state = props.state.clone();
        Callback::from(move |_| {
            state.dispatch(AppAction::SelectAll);
        })
    };

    let zoom_in = {
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let zoom = state.zoom_level;
        let current_time_ms = state.current_time_ms;
        Callback::from(move |_| {
            let old_px_per_second = 92.0 * zoom;
            let new_zoom = zoom * 1.25;
            let new_px_per_second = 92.0 * new_zoom;
            let playhead_x_old = current_time_ms.to_secs() * old_px_per_second;
            let playhead_x_new = current_time_ms.to_secs() * new_px_per_second;
            
            let screen_x = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                playhead_x_old - vp.scroll_left() as f64
            } else {
                0.0
            };
            let new_scroll = playhead_x_new - screen_x;

            state.dispatch(AppAction::SetZoom(new_zoom));
            
            if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let vp_clone = vp.clone();
                let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                    vp_clone.set_scroll_left(new_scroll as i32);
                });
                let _ = web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
            }
        })
    };

    let zoom_out = {
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let zoom = state.zoom_level;
        let current_time_ms = state.current_time_ms;
        Callback::from(move |_| {
            let old_px_per_second = 92.0 * zoom;
            let new_zoom = zoom / 1.25;
            let new_px_per_second = 92.0 * new_zoom;
            let playhead_x_old = current_time_ms.to_secs() * old_px_per_second;
            let playhead_x_new = current_time_ms.to_secs() * new_px_per_second;
            
            let screen_x = if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                playhead_x_old - vp.scroll_left() as f64
            } else {
                0.0
            };
            let new_scroll = playhead_x_new - screen_x;

            state.dispatch(AppAction::SetZoom(new_zoom));
            
            if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let vp_clone = vp.clone();
                let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                    vp_clone.set_scroll_left(new_scroll as i32);
                });
                let _ = web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
            }
        })
    };

    // Smooth playhead & auto pan
    {
        let playing = props.state.playing;
        let dragging_playhead = *drag_mode == Some(DragTarget::Playhead);
        let playhead_ref = playhead_ref.clone();
        let viewport_ref = viewport_ref.clone();
        let timecode_ref = timecode_ref.clone();
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();
        
        // Use the live reference for the RAF loop
        let current_time_ms_ref = current_time_ms_ref.clone();
        
        let px_per_second_ref = use_mut_ref(|| px_per_second);
        *px_per_second_ref.borrow_mut() = px_per_second;

        use_effect_with((playing, dragging_playhead), move |(playing, dragging_playhead)| {
            use wasm_bindgen::closure::Closure;
            use std::rc::Rc;
            use std::cell::RefCell;

            let cb = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
            let cb_clone = cb.clone();
            
            let playhead = playhead_ref.clone();
            let viewport = viewport_ref.clone();
            let timecode = timecode_ref.clone();

            if *playing || *dragging_playhead {
                *cb_clone.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                    if let (Some(p), Some(v)) = (
                        playhead.cast::<web_sys::HtmlElement>(),
                        viewport.cast::<web_sys::HtmlElement>(),
                    ) {
                        let px = px_per_second_ref.borrow().as_f64();
                        let current_time_ms = *current_time_ms_ref.borrow();
                        let playhead_x = current_time_ms.to_secs() * px;
                        let _ = p.set_attribute("style", &format!("transform: translateX({}px);", playhead_x));

                        let scroll_left = v.scroll_left() as f64;
                        let client_width = v.client_width() as f64;
                        if !*is_scrollbar_dragged.borrow() {
                            if playhead_x < scroll_left || playhead_x > scroll_left + client_width {
                                v.set_scroll_left(playhead_x as i32);
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
                    if let Some(window) = web_sys::window() {
                        if let Some(closure) = cb.borrow().as_ref() {
                            let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                        }
                    }
                }) as Box<dyn FnMut()>));
                
                if let Some(window) = web_sys::window() {
                    let _ = window.request_animation_frame(cb_clone.borrow().as_ref().unwrap().as_ref().unchecked_ref());
                }
            } else {
                *cb_clone.borrow_mut() = None;
            }
            
            move || {
                *cb_clone.borrow_mut() = None;
            }
        });
    }

    let on_timeline_mousedown = {
        let state = props.state.clone();
        Callback::from(move |e: MouseEvent| {
            state.dispatch(AppAction::ClearSelection);
        })
    };

    let on_ruler_mousedown = {
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        let last_mouse_pos = last_mouse_pos.clone();
        
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            drag_mode.set(Some(DragTarget::Playhead));
            drag_start_x.set(Pixels(e.client_x() as f64));
            *last_mouse_pos.borrow_mut() = (e.client_x() as f64, e.client_y() as f64);
            
            // Make playhead jump to cursor immediately
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
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Delete" || e.key() == "Backspace" {
                state.dispatch(AppAction::DeleteSelected);
            }
        })
    };

    let on_mousemove = {
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_offset_ms = drag_offset_ms.clone();
        let viewport_ref = viewport_ref.clone();
        let pan_velocity = pan_velocity.clone();
        let px_per_second = px_per_second;
        let last_mouse_pos = last_mouse_pos.clone();

        Callback::from(move |e: MouseEvent| {
            *last_mouse_pos.borrow_mut() = (e.client_x() as f64, e.client_y() as f64);
            
            if let Some(mode) = *drag_mode {
                let delta_x = e.client_x() as f64 - drag_start_x.as_f64();
                
                if mode == DragTarget::Playhead {
                    if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                        let rect = v.get_bounding_client_rect();
                        let mouse_x = e.client_x() as f64;
                        
                        let safe_zone = 60.0;
                        if mouse_x < rect.left() + safe_zone {
                            let ratio = (rect.left() + safe_zone - mouse_x) / safe_zone;
                            *pan_velocity.borrow_mut() = -10.0 * ratio.min(1.0);
                        } else if mouse_x > rect.right() - safe_zone {
                            let ratio = (mouse_x - (rect.right() - safe_zone)) / safe_zone;
                            *pan_velocity.borrow_mut() = 10.0 * ratio.min(1.0);
                        } else {
                            *pan_velocity.borrow_mut() = 0.0;
                        }
                    }
                } else {
                    let delta_ms = (delta_x / px_per_second.as_f64() * 1000.0) as i32;
                    drag_offset_ms.set(delta_ms);
                }
            }
        })
    };

    let on_scrollbar_mousedown = {
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();
        Callback::from(move |_| {
            *is_scrollbar_dragged.borrow_mut() = true;
        })
    };

    let on_global_mouseup = {
        let drag_mode = drag_mode.clone();
        let drag_offset_ms = drag_offset_ms.clone();
        let drag_target_id = drag_target_id.clone();
        let state = props.state.clone();
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();
        let pan_velocity = pan_velocity.clone();
        
        Callback::from(move |_| {
            *is_scrollbar_dragged.borrow_mut() = false;
            *pan_velocity.borrow_mut() = 0.0;
            
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
                        if offset != 0 {
                            if let Some(id) = *drag_target_id {
                                state.dispatch(AppAction::ShiftBoundary(id, true, offset));
                            }
                        }
                    }
                    DragTarget::RightEdge => {
                        let offset = *drag_offset_ms;
                        if offset != 0 {
                            if let Some(id) = *drag_target_id {
                                state.dispatch(AppAction::ShiftBoundary(id, false, offset));
                            }
                        }
                    }
                    DragTarget::Playhead => {
                        state.dispatch(AppAction::Seek(state.current_time_ms));
                    }
                }
                drag_mode.set(None);
                drag_offset_ms.set(0);
                drag_target_id.set(None);
            }
        })
    };

    let on_chunk_drag_start = {
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_target_id = drag_target_id.clone();
        Callback::from(move |(id, e, target): (usize, MouseEvent, DragTarget)| {
            drag_mode.set(Some(target));
            drag_start_x.set(Pixels(e.client_x() as f64));
            drag_target_id.set(Some(id));
        })
    };

    html! {
        <div class="panel timeline-panel" onmouseup={on_global_mouseup.clone()} onmouseleave={on_global_mouseup.clone()}>
            <input 
                type="file" 
                accept="audio/*" 
                ref={file_input_ref} 
                style="display: none;" 
                onchange={on_file_change} 
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
                on_toggle_play={toggle_play}
                on_zoom_in={zoom_in}
                on_zoom_out={zoom_out}
            />
            <div class="timeline-body">
                <TrackPads 
                    on_import_audio={import_click.clone()}
                    on_select_all={select_all}
                />
                <TimelineLanes 
                    state={props.state.clone()}
                    viewport_ref={viewport_ref}
                    canvas_ref={canvas_ref}
                    playhead_ref={playhead_ref}
                    audio_url={(*audio_url).clone()}
                    duration_ms={duration_ms}
                    width_px={width_px}
                    audio_width_px={audio_width_px}
                    px_per_second={px_per_second}
                    drag_mode={*drag_mode}
                    drag_offset_ms={*drag_offset_ms}
                    drag_target_id={*drag_target_id}
                    on_viewport_scroll={on_viewport_scroll}
                    on_keydown={on_keydown}
                    on_mousemove={on_mousemove}
                    on_mousedown_content={on_timeline_mousedown}
                    on_mousedown_ruler={on_ruler_mousedown}
                    on_import_audio={import_click}
                    on_chunk_drag_start={on_chunk_drag_start}
                />
            </div>
            <div class="timeline-controls">
                <div class="timeline-scroll" ref={scrollbar_ref} onscroll={on_scrollbar_scroll} onmousedown={on_scrollbar_mousedown}>
                    <div style={format!("width: {}px; height: 1px;", width_px.as_f64())}></div>
                </div>
            </div>
        </div>
    }
}
