use yew::prelude::*;
use web_sys::{HtmlAudioElement, HtmlInputElement, Url, AudioContext, Request, RequestInit, RequestMode, Response};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use std::rc::Rc;
use crate::web_app::actions::{AppState, AppAction};
use crate::domain::{TimeMs, Pixels};
use super::timeline::{PlaybackControls, TrackPads, TimelineLanes, DragTarget};
use super::timeline::waveform_canvas::WaveformSummary;

#[derive(Properties, PartialEq)]
pub struct TimelinePanelProps {
    pub state: UseReducerHandle<AppState>,
}

#[function_component(TimelinePanel)]
pub fn timeline_panel(props: &TimelinePanelProps) -> Html {
    let px_per_second = Pixels(92.0 * props.state.zoom_level);
    
    let audio_ref = use_node_ref();
    let file_input_ref = use_node_ref();
    let lrc_input_ref = use_node_ref();
    let canvas_ref = use_node_ref();
    let viewport_ref = use_node_ref();
    let playhead_ref = use_node_ref();
    let timecode_ref = use_node_ref();
    
    let audio_url = use_state(|| None::<String>);
    let waveform_summary = use_state(|| None::<Rc<WaveformSummary>>);
    let scroll_left = use_state(|| 0.0);
    let viewport_width = use_state(|| 0.0);

    let drag_mode = use_state(|| None::<DragTarget>);
    let suppress_panning = use_state(|| false);
    let drag_start_x = use_state(|| Pixels(0.0));
    let drag_start_y = use_state(|| Pixels(0.0));
    let selection_rect = use_state(|| None::<(f64, f64, f64, f64)>);
    let drag_offset_ms = use_state(|| 0i32);
    let drag_target_uid = use_state(|| None::<usize>);
    let drag_start_scroll = use_state(|| 0.0);
    let drag_scrollbar_track_width = use_state(|| 1.0);
    let is_scrollbar_dragged = use_mut_ref(|| false);
    let last_mouse_pos = use_mut_ref(|| (0.0, 0.0));

    let on_file_change = {
        let audio_url = audio_url.clone();
        let waveform_summary = waveform_summary.clone();
        let state = props.state.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    state.dispatch(AppAction::SetAudioFilename(file.name()));
                    if let Ok(url) = Url::create_object_url_with_blob(&file) {
                        audio_url.set(Some(url.clone()));
                        
                        let waveform_summary = waveform_summary.clone();
                        spawn_local(async move {
                            let opts = RequestInit::new();
                            opts.set_method("GET");
                            opts.set_mode(RequestMode::Cors);
                            let request = Request::new_with_str_and_init(&url, &opts).unwrap();
                            let window = web_sys::window().unwrap();
                            
                            if let Ok(resp_value) = JsFuture::from(window.fetch_with_request(&request)).await {
                                let resp: Response = resp_value.dyn_into().unwrap();
                                let array_buffer = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
                                
                                let audio_ctx = AudioContext::new().unwrap();
                                let audio_buffer_promise = audio_ctx.decode_audio_data(&array_buffer.into()).unwrap();
                                if let Ok(audio_buffer_value) = JsFuture::from(audio_buffer_promise).await {
                                    let audio_buffer: web_sys::AudioBuffer = audio_buffer_value.dyn_into().unwrap();
                                    waveform_summary.set(Some(Rc::new(downsample_audio(audio_buffer))));
                                }
                            }
                        });
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

    let import_lrc_click = {
        let lrc_input_ref = lrc_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = lrc_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_lrc_change = {
        let state = props.state.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    state.dispatch(AppAction::SetLrcFilename(file.name()));
                    let state = state.clone();
                    spawn_local(async move {
                        let text_promise = file.text();
                        if let Ok(text_value) = JsFuture::from(text_promise).await {
                            if let Some(text) = text_value.as_string() {
                                state.dispatch(AppAction::UpdateSource(text.clone()));
                                state.dispatch(AppAction::SaveHistory(text));
                            }
                        }
                    });
                }
            }
        })
    };

    let export_lrc = {
        let state = props.state.clone();
        Callback::from(move |_| {
            let text = state.source_text.clone();
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            
            let filename = if let Some(audio_name) = &state.audio_filename {
                let base = audio_name.rfind('.').map(|i| &audio_name[..i]).unwrap_or(audio_name);
                format!("{}.lrc", base)
            } else if let Some(lrc_name) = &state.lrc_filename {
                lrc_name.clone()
            } else {
                "lyrics.lrc".to_string()
            };

            let blob = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&text.into())).unwrap();
            let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
            
            let a = document.create_element("a").unwrap().dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
            a.set_href(&url);
            a.set_download(&filename);
            a.click();
            let _ = web_sys::Url::revoke_object_url(&url);
        })
    };

    let on_loaded_metadata = {
        let state = props.state.clone();
        Callback::from(move |e: Event| {
            if let Some(audio) = e.target_dyn_into::<web_sys::HtmlAudioElement>() {
                state.dispatch(AppAction::SetDuration(TimeMs((audio.duration() * 1000.0) as u32)));
                state.dispatch(AppAction::Seek(state.current_time_ms));
            }
        })
    };

    let pan_velocity = use_mut_ref(|| 0.0);
    
    let current_time_ms_ref = use_mut_ref(|| props.state.current_time_ms);
    *current_time_ms_ref.borrow_mut() = props.state.current_time_ms;

    let duration_ms = props.state.max_timeline_duration();
    let width_px = Pixels(duration_ms.to_secs() * px_per_second.as_f64());
    let audio_width_px = Pixels(props.state.duration_ms.to_secs() * px_per_second.as_f64());

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
        let suppress_panning = suppress_panning.clone();
        let scroll_left_state = scroll_left.clone();

        use_effect(move || {
            let suppress_panning = suppress_panning.clone();
            let interval = gloo_timers::callback::Interval::new(16, move || {
                let vel = *pan_velocity.borrow();
                let mode = *drag_mode;
                
                if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    let old_scroll = v.scroll_left();
                    if vel != 0.0 && mode.is_some() {
                        v.set_scroll_left(old_scroll + vel as i32);
                        scroll_left_state.set(v.scroll_left() as f64);
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

                        let playhead_x = absolute_x;
                        let scroll_left = v.scroll_left() as f64;
                        let client_width = v.client_width() as f64;
                        let safe_zone = 90.0;

                        let is_in_safe_zone = playhead_x < scroll_left + safe_zone || playhead_x > scroll_left + client_width - safe_zone;
                        let hit_right_border = playhead_x >= scroll_left + client_width - 1.0;

                        if hit_right_border || !is_in_safe_zone {
                            suppress_panning.set(false);
                        } else if is_in_safe_zone {
                            suppress_panning.set(true);
                        }
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

        let bounds_ref = yew::use_mut_ref(|| TimeMs(0));
        *bounds_ref.borrow_mut() = {
            let audio_dur = props.state.duration_ms;
            let last_lyric = props.state.document.as_ref().and_then(|d| d.last_entry_time_ms()).unwrap_or(TimeMs(0));
            TimeMs(audio_dur.as_u32().max(last_lyric.as_u32()) + 10000)
        };

        let last_seek_ref = yew::use_mut_ref(|| props.state.last_seek_request);
        *last_seek_ref.borrow_mut() = props.state.last_seek_request;

        let dragging_playhead = *drag_mode == Some(DragTarget::Playhead);

        use_effect_with((playing, dragging_playhead), move |(playing, dragging_playhead)| {
            let mut interval_opt = None;
            
            if *playing && !*dragging_playhead {
                let start_time = *current_time_ref.borrow();
                
                if let Some(audio) = audio_ref.cast::<web_sys::HtmlAudioElement>() {
                    let audio_dur_ms = (audio.duration() * 1000.0) as u32;
                    if start_time.as_u32() < audio_dur_ms {
                        let _ = audio.play();
                    } else {
                        let _ = audio.pause();
                    }
                }
                
                let last_time = js_sys::Date::now();
                let last_time_ref = std::rc::Rc::new(std::cell::Cell::new(last_time));
                
                let mut local_current_f64 = start_time.as_u32() as f64;
                let mut last_handled_seek = *last_seek_ref.borrow();
                
                let interval = gloo_timers::callback::Interval::new(16, move || {
                    let now = js_sys::Date::now();
                    let delta = now - last_time_ref.get();
                    last_time_ref.set(now);
                    
                    local_current_f64 += delta;
                    
                    let current_seek = *last_seek_ref.borrow();
                    if current_seek != last_handled_seek {
                        last_handled_seek = current_seek;
                        if let Some(seek_time) = current_seek {
                            local_current_f64 = seek_time.as_u32() as f64;
                        }
                    }
                    
                    if let Some(audio) = audio_ref.cast::<web_sys::HtmlAudioElement>() {
                        let audio_dur_ms = (audio.duration() * 1000.0) as u32;
                        if local_current_f64 + 200.0 < audio_dur_ms as f64 && !audio.paused() && !audio.ended() {
                            let audio_time_ms = audio.current_time() * 1000.0;
                            if audio_time_ms > 0.0 || local_current_f64 < 500.0 {
                                local_current_f64 = audio_time_ms;
                            }
                        } else if local_current_f64 < audio_dur_ms as f64 && audio.paused() && !audio.ended() {
                            if local_current_f64 + 100.0 < audio_dur_ms as f64 {
                                let _ = audio.play();
                            }
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
        let last_seek = props.state.last_seek_request;
        let audio_ref = audio_ref.clone();
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        let suppress_panning = suppress_panning.clone();
        let scroll_left_state = scroll_left.clone();
        use_effect_with(last_seek, move |seek| {
            let suppress_panning = suppress_panning.clone();
            if let Some(time_ms) = seek {
                if let Some(audio) = audio_ref.cast::<HtmlAudioElement>() {
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

                    if is_off_screen || (is_in_safe_zone && !*suppress_panning) {
                        v.set_scroll_left((playhead_x - client_width / 2.0) as i32);
                        scroll_left_state.set(v.scroll_left() as f64);
                    }
                }
            }
            || ()
        });
    }

    // Sync playhead visual position when not animating
    {
        let current_time_ms = props.state.current_time_ms;
        let playing = props.state.playing;
        let drag_mode = *drag_mode;
        let playhead_ref = playhead_ref.clone();
        let px_per_second = px_per_second;

        use_effect_with((current_time_ms, playing, drag_mode, px_per_second), move |(time, playing, mode, px_per_second)| {
            if !*playing && *mode != Some(DragTarget::Playhead) {
                if let Some(p) = playhead_ref.cast::<web_sys::HtmlElement>() {
                    let playhead_x = time.to_secs() * px_per_second.as_f64();
                    let _ = p.set_attribute("style", &format!("transform: translateX({}px);", playhead_x));
                }
            }
            || ()
        });
    }

    let on_viewport_scroll = {
        let scroll_left = scroll_left.clone();
        Callback::from(move |e: Event| {
            if let Some(viewport) = e.target_dyn_into::<web_sys::HtmlElement>() {
                scroll_left.set(viewport.scroll_left() as f64);
            }
        })
    };

    let on_wheel = {
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let scroll_left_state = scroll_left.clone();
        let px_per_second_val = 92.0 * props.state.zoom_level;
        Callback::from(move |e: WheelEvent| {
            if e.ctrl_key() || e.meta_key() {
                e.prevent_default();
                if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                    let rect = vp.get_bounding_client_rect();
                    let mouse_x = e.client_x() as f64 - rect.left();
                    let world_x = mouse_x + vp.scroll_left() as f64;
                    let cursor_time_s = world_x / px_per_second_val;
                    
                    let zoom_factor = if e.delta_y() < 0.0 { 1.15 } else { 1.0 / 1.15 };
                    let new_zoom = (state.zoom_level * zoom_factor).clamp(0.1, 10.0);
                    let new_px_per_second = 92.0 * new_zoom;
                    
                    let new_world_x = cursor_time_s * new_px_per_second;
                    let new_scroll_left = new_world_x - mouse_x;
                    
                    state.dispatch(AppAction::SetZoom(new_zoom));
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

    let on_scrollbar_mousedown = {
        let viewport_ref = viewport_ref.clone();
        let total_width = width_px.as_f64();
        let scroll_left_state = scroll_left.clone();
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_start_scroll = drag_start_scroll.clone();
        let drag_scrollbar_track_width = drag_scrollbar_track_width.clone();
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();

        Callback::from(move |e: MouseEvent| {
            if let Some(vp) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let track_element = e.current_target().unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
                let rect = track_element.get_bounding_client_rect();
                drag_scrollbar_track_width.set(rect.width());
                
                let target = e.target().unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
                let is_handle = target.class_list().contains("custom-scrollbar-handle");

                if is_handle {
                    *is_scrollbar_dragged.borrow_mut() = true;
                    drag_mode.set(Some(DragTarget::Scrollbar));
                    drag_start_x.set(Pixels(e.client_x() as f64));
                    drag_start_scroll.set(*scroll_left_state);
                    e.stop_propagation();
                } else {
                    let click_x = e.client_x() as f64 - rect.left();
                    let ratio = (click_x / rect.width()).clamp(0.0, 1.0);
                    let target_scroll = ratio * total_width - vp.client_width() as f64 / 2.0;
                    vp.set_scroll_left(target_scroll as i32);
                    let new_scroll = vp.scroll_left() as f64;
                    scroll_left_state.set(new_scroll);
                    
                    *is_scrollbar_dragged.borrow_mut() = true;
                    drag_mode.set(Some(DragTarget::Scrollbar));
                    drag_start_x.set(Pixels(e.client_x() as f64));
                    drag_start_scroll.set(new_scroll);
                }
            }
        })
    };

    {
        let viewport_ref = viewport_ref.clone();
        let viewport_width = viewport_width.clone();
        use_effect_with((), move |_| {
            let vw_clone = viewport_width.clone();
            let vr_clone = viewport_ref.clone();
            let listener = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                if let Some(v) = vr_clone.cast::<web_sys::HtmlElement>() {
                    vw_clone.set(v.client_width() as f64);
                }
            }) as Box<dyn FnMut()>);
            
            let window = web_sys::window().unwrap();
            window.add_event_listener_with_callback("resize", listener.as_ref().unchecked_ref()).unwrap();
            
            if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                viewport_width.set(v.client_width() as f64);
            }
            
            move || {
                window.remove_event_listener_with_callback("resize", listener.as_ref().unchecked_ref()).unwrap();
            }
        });
    }

    let toggle_play = {
        let state = props.state.clone();
        Callback::from(move |_| {
            state.dispatch(AppAction::TogglePlay);
        })
    };

    let zoom_in = {
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let zoom = state.zoom_level;
        let current_time_ms = state.current_time_ms;
        let scroll_left_state = scroll_left.clone();
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
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let zoom = state.zoom_level;
        let current_time_ms = state.current_time_ms;
        let scroll_left_state = scroll_left.clone();
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
                let scroll_left_state = scroll_left_state.clone();
                let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                    vp_clone.set_scroll_left(new_scroll as i32);
                    scroll_left_state.set(vp_clone.scroll_left() as f64);
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
        let last_mouse_pos = last_mouse_pos.clone();
        let suppress_panning = suppress_panning.clone();
        let scroll_left_state = scroll_left.clone();
        
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
            let last_mouse_pos = last_mouse_pos.clone();
            let is_dragging = *dragging_playhead;
            let suppress_panning = suppress_panning.clone();
            let scroll_left_state = scroll_left_state.clone();

            if *playing || is_dragging {
                *cb_clone.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                    if let (Some(p), Some(v)) = (
                        playhead.cast::<web_sys::HtmlElement>(),
                        viewport.cast::<web_sys::HtmlElement>(),
                    ) {
                        let rect = v.get_bounding_client_rect();
                        let px = px_per_second_ref.borrow().as_f64();
                        let current_time_ms = *current_time_ms_ref.borrow();
                        
                        let playhead_x = if is_dragging {
                            let (mouse_x, _) = *last_mouse_pos.borrow();
                            mouse_x - rect.left() + v.scroll_left() as f64
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

                            if !*suppress_panning || is_off_screen {
                                if world_x > scroll_left + client_width - safe_zone {
                                    v.set_scroll_left((world_x - safe_zone) as i32);
                                    scroll_left_state.set(v.scroll_left() as f64);
                                } else if world_x < scroll_left + safe_zone {
                                    v.set_scroll_left((world_x - client_width / 2.0) as i32);
                                    scroll_left_state.set(v.scroll_left() as f64);
                                }
                            }

                            let hit_right_border = world_x >= scroll_left + client_width - 1.0;
                            if hit_right_border || !is_in_safe_zone {
                                suppress_panning.set(false);
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
        let drag_mode = drag_mode.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_start_y = drag_start_y.clone();
        let state = props.state.clone();
        let viewport_ref = viewport_ref.clone();
        let last_mouse_pos = last_mouse_pos.clone();
        
        Callback::from(move |e: MouseEvent| {
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let _ = viewport.focus();
                
                if !e.ctrl_key() && !e.meta_key() && !e.shift_key() {
                    state.dispatch(AppAction::ClearSelection);
                }
                
                let rect = viewport.get_bounding_client_rect();
                let x = e.client_x() as f64 - rect.left() + viewport.scroll_left() as f64;
                let y = e.client_y() as f64 - rect.top();
                
                drag_mode.set(Some(DragTarget::Selection));
                drag_start_x.set(Pixels(x));
                drag_start_y.set(Pixels(y));
                *last_mouse_pos.borrow_mut() = (e.client_x() as f64, e.client_y() as f64);
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
        
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            drag_mode.set(Some(DragTarget::Playhead));
            drag_start_x.set(Pixels(e.client_x() as f64));
            *last_mouse_pos.borrow_mut() = (e.client_x() as f64, e.client_y() as f64);
            
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
        let drag_start_y = drag_start_y.clone();
        let selection_rect = selection_rect.clone();
        let drag_offset_ms = drag_offset_ms.clone();
        let viewport_ref = viewport_ref.clone();
        let pan_velocity = pan_velocity.clone();
        let px_per_second = px_per_second;
        let last_mouse_pos = last_mouse_pos.clone();
        let scroll_left_state = scroll_left.clone();

        let drag_start_scroll = drag_start_scroll.clone();
        let drag_scrollbar_track_width = drag_scrollbar_track_width.clone();

        Callback::from(move |e: MouseEvent| {
            *last_mouse_pos.borrow_mut() = (e.client_x() as f64, e.client_y() as f64);
            
            if let Some(mode) = *drag_mode {
                let delta_x = e.client_x() as f64 - drag_start_x.as_f64();
                
                if mode == DragTarget::Playhead {
                    if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                        let rect = v.get_bounding_client_rect();
                        let mouse_x = e.client_x() as f64;
                        
                        let safe_zone = 90.0;
                        if mouse_x < rect.left() + safe_zone {
                            let ratio = (rect.left() + safe_zone - mouse_x) / safe_zone;
                            *pan_velocity.borrow_mut() = -10.0 * ratio.min(1.0);
                        } else if mouse_x > rect.right() - safe_zone {
                            let ratio = (mouse_x - (rect.right() - safe_zone)) / safe_zone;
                            *pan_velocity.borrow_mut() = 10.0 * ratio.min(1.0);
                        } else {
                            *pan_velocity.borrow_mut() = 0.0;
                        }
                        scroll_left_state.set(v.scroll_left() as f64);
                    }
                } else if mode == DragTarget::Selection {
                    if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                        let rect = v.get_bounding_client_rect();
                        let current_x = e.client_x() as f64 - rect.left() + v.scroll_left() as f64;
                        let current_y = e.client_y() as f64 - rect.top();
                        
                        let start_x = drag_start_x.as_f64();
                        let start_y = drag_start_y.as_f64();
                        
                        let x = start_x.min(current_x);
                        let y = start_y.min(current_y);
                        let w = (current_x - start_x).abs();
                        let h = (current_y - start_y).abs();
                        
                        selection_rect.set(Some((x, y, w, h)));
                    }
                } else if mode == DragTarget::Scrollbar {
                    if let Some(v) = viewport_ref.cast::<web_sys::HtmlElement>() {
                        let delta_scroll = (delta_x / *drag_scrollbar_track_width) * width_px.as_f64();
                        let new_scroll = *drag_start_scroll + delta_scroll;
                        v.set_scroll_left(new_scroll as i32);
                        scroll_left_state.set(v.scroll_left() as f64);
                    }
                } else {
                    let delta_ms = (delta_x / px_per_second.as_f64() * 1000.0) as i32;
                    drag_offset_ms.set(delta_ms);
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
        let pan_velocity = pan_velocity.clone();
        let px_per_second = px_per_second;
        
        Callback::from(move |e: MouseEvent| {
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
                            if let Some(uid) = *drag_target_uid {
                                state.dispatch(AppAction::ShiftBoundary(uid, true, false, offset));
                            }
                        }
                    }
                    DragTarget::RightEdge => {
                        let offset = *drag_offset_ms;
                        if offset != 0 {
                            if let Some(uid) = *drag_target_uid {
                                state.dispatch(AppAction::ShiftBoundary(uid, false, false, offset));
                            }
                        }
                    }
                    DragTarget::Boundary => {
                        let offset = *drag_offset_ms;
                        if offset != 0 {
                            if let Some(uid) = *drag_target_uid {
                                state.dispatch(AppAction::ShiftBoundary(uid, false, true, offset));
                            }
                        }
                    }
                    DragTarget::Playhead => {
                        state.dispatch(AppAction::Seek(state.current_time_ms));
                    }
                    DragTarget::Selection => {
                        if let Some((x, _y, w, _h)) = *selection_rect {
                            if let Some(doc) = &state.document {
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
        Callback::from(move |(uid, e, target): (usize, MouseEvent, DragTarget)| {
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let _ = viewport.focus();
            }
            drag_mode.set(Some(target));
            drag_start_x.set(Pixels(e.client_x() as f64));
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
                />
            </div>
        </div>
    }
}

fn downsample_audio(audio_buffer: web_sys::AudioBuffer) -> WaveformSummary {
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
