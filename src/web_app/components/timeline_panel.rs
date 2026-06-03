use yew::prelude::*;
use web_sys::{HtmlAudioElement, HtmlInputElement, HtmlCanvasElement, Url, AudioContext, Request, RequestInit, RequestMode, Response};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use wasm_bindgen::JsCast;
use crate::web_app::app::{AppState, AppAction};
use crate::domain::SelectionMode;

fn draw_waveform(canvas: HtmlCanvasElement, url: String) {
    spawn_local(async move {
        let mut opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);
        let request = Request::new_with_str_and_init(&url, &opts).unwrap();
        let window = web_sys::window().unwrap();
        
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
        let resp: Response = resp_value.dyn_into().unwrap();
        let array_buffer = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
        
        let audio_ctx = AudioContext::new().unwrap();
        let audio_buffer_promise = audio_ctx.decode_audio_data(&array_buffer.into()).unwrap();
        let audio_buffer_value = JsFuture::from(audio_buffer_promise).await.unwrap();
        let audio_buffer: web_sys::AudioBuffer = audio_buffer_value.dyn_into().unwrap();
        
        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();
            
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        ctx.clear_rect(0.0, 0.0, width, height);
        ctx.set_fill_style_str("#1fb7b0"); // --teal
        
        if let Ok(data) = audio_buffer.get_channel_data(0) {
            let step = (data.len() as f64 / width).ceil() as usize;
            let amp = height / 2.0;
            
            for i in 0..(width as usize) {
                let mut min = 1.0f32;
                let mut max = -1.0f32;
                for j in 0..step {
                    let idx = i * step + j;
                    if idx < data.len() {
                        let val = data[idx];
                        if val < min { min = val; }
                        if val > max { max = val; }
                    }
                }
                ctx.fill_rect(i as f64, amp as f64 + (min as f64 * amp), 1.0, (max - min) as f64 * amp);
            }
        }
    });
}

#[derive(Properties, PartialEq)]
pub struct TimelinePanelProps {
    pub state: UseReducerHandle<AppState>,
}

#[derive(Clone, Copy, PartialEq)]
enum DragMode {
    Body,
    LeftEdge,
    RightEdge,
}

#[function_component(TimelinePanel)]
pub fn timeline_panel(props: &TimelinePanelProps) -> Html {
    let px_per_second = 92.0 * props.state.zoom_level;
    
    let audio_ref = use_node_ref();
    let file_input_ref = use_node_ref();
    let canvas_ref = use_node_ref();
    let viewport_ref = use_node_ref();
    let scrollbar_ref = use_node_ref();
    let playhead_ref = use_node_ref();
    let timecode_ref = use_node_ref();
    
    let audio_url = use_state(|| None::<String>);

    let drag_mode = use_state(|| None::<DragMode>);
    let drag_start_x = use_state(|| 0.0);
    let drag_offset_ms = use_state(|| 0i32);
    let drag_target_id = use_state(|| None::<usize>);
    let is_scrollbar_dragged = use_mut_ref(|| false);

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

    let on_time_update = {
        let state = props.state.clone();
        Callback::from(move |e: Event| {
            if let Some(audio) = e.target_dyn_into::<HtmlAudioElement>() {
                state.dispatch(AppAction::SetTime((audio.current_time() * 1000.0) as u32));
            }
        })
    };

    let on_loaded_metadata = {
        let state = props.state.clone();
        Callback::from(move |e: Event| {
            if let Some(audio) = e.target_dyn_into::<HtmlAudioElement>() {
                state.dispatch(AppAction::SetDuration((audio.duration() * 1000.0) as u32));
            }
        })
    };

    let on_ended = {
        let state = props.state.clone();
        Callback::from(move |_| {
            if state.playing {
                state.dispatch(AppAction::TogglePlay);
            }
        })
    };

    // Sync play/pause from state
    {
        let playing = props.state.playing;
        let audio_ref = audio_ref.clone();
        use_effect_with(playing, move |playing| {
            if let Some(audio) = audio_ref.cast::<HtmlAudioElement>() {
                if *playing {
                    let _ = audio.play();
                } else {
                    let _ = audio.pause();
                }
            }
            || ()
        });
    }

    // Sync seek
    {
        let last_seek = props.state.last_seek_request;
        let audio_ref = audio_ref.clone();
        use_effect_with(last_seek, move |seek| {
            if let Some(time_ms) = seek {
                if let Some(audio) = audio_ref.cast::<HtmlAudioElement>() {
                    audio.set_current_time(*time_ms as f64 / 1000.0);
                }
            }
            || ()
        });
    }

    let last_lyric_ms = props.state.document.as_ref().and_then(|doc| doc.last_entry_time_ms()).unwrap_or(0);
    let duration_ms = props.state.duration_ms.max(last_lyric_ms) + 10000;
    let width_px = (duration_ms as f64 / 1000.0) * px_per_second;

    // Draw waveform once duration is set and url is present
    {
        let url = (*audio_url).clone();
        let canvas_ref = canvas_ref.clone();
        use_effect_with((url.clone(), duration_ms), move |(url, _duration_ms)| {
            if let Some(u) = url {
                if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                    canvas.set_width(width_px as u32);
                    canvas.set_height(canvas.client_height() as u32);
                    draw_waveform(canvas, u.clone());
                }
            }
            || ()
        });
    }

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
            let playhead_x_old = (current_time_ms as f64 / 1000.0) * old_px_per_second;
            let playhead_x_new = (current_time_ms as f64 / 1000.0) * new_px_per_second;
            
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
            let playhead_x_old = (current_time_ms as f64 / 1000.0) * old_px_per_second;
            let playhead_x_new = (current_time_ms as f64 / 1000.0) * new_px_per_second;
            
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

    let time_str = {
        let total_secs = props.state.current_time_ms / 1000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        let ms = props.state.current_time_ms % 1000;
        format!("{:02}:{:02}.{:03}", mins, secs, ms)
    };

    // Smooth playhead & auto pan
    {
        let playing = props.state.playing;
        let audio_ref = audio_ref.clone();
        let playhead_ref = playhead_ref.clone();
        let viewport_ref = viewport_ref.clone();
        let timecode_ref = timecode_ref.clone();
        let px_per_second_ref = use_mut_ref(|| px_per_second);
        *px_per_second_ref.borrow_mut() = px_per_second;
        let is_scrollbar_dragged = is_scrollbar_dragged.clone();

        use_effect_with(playing, move |playing| {
            use wasm_bindgen::closure::Closure;
            use std::rc::Rc;
            use std::cell::RefCell;

            let cb = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
            let cb_clone = cb.clone();
            
            let audio = audio_ref.clone();
            let playhead = playhead_ref.clone();
            let viewport = viewport_ref.clone();
            let timecode = timecode_ref.clone();

            if *playing {
                *cb_clone.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                    if let (Some(a), Some(p), Some(v)) = (
                        audio.cast::<HtmlAudioElement>(),
                        playhead.cast::<web_sys::HtmlElement>(),
                        viewport.cast::<web_sys::HtmlElement>(),
                    ) {
                        let px = *px_per_second_ref.borrow();
                        let ct = a.current_time();
                        let playhead_x = ct * px;
                        let _ = p.set_attribute("style", &format!("transform: translateX({}px);", playhead_x));

                        let scroll_left = v.scroll_left() as f64;
                        let client_width = v.client_width() as f64;
                        if !*is_scrollbar_dragged.borrow() {
                            if playhead_x < scroll_left || playhead_x > scroll_left + client_width {
                                v.set_scroll_left(playhead_x as i32);
                            }
                        }

                        if let Some(tc) = timecode.cast::<web_sys::HtmlElement>() {
                            let total_secs = ct as u32;
                            let mins = total_secs / 60;
                            let secs = total_secs % 60;
                            let ms = (ct * 1000.0) as u32 % 1000;
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
        let viewport_ref = viewport_ref.clone();
        let px_per_second = px_per_second;
        Callback::from(move |e: MouseEvent| {
            if let Some(viewport) = viewport_ref.cast::<web_sys::HtmlElement>() {
                let rect = viewport.get_bounding_client_rect();
                let x = e.client_x() as f64 - rect.left() + viewport.scroll_left() as f64;
                let time = x / px_per_second;
                state.dispatch(AppAction::Seek((time * 1000.0) as u32));
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
        Callback::from(move |e: MouseEvent| {
            if drag_mode.is_some() {
                let delta_x = e.client_x() as f64 - *drag_start_x;
                let delta_ms = (delta_x / px_per_second * 1000.0) as i32;
                drag_offset_ms.set(delta_ms);
            }
        })
    };

    // on_global_mouseup handles global mouse ups, so we don't need on_mouseup on the viewport anymore.

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
        
        Callback::from(move |_| {
            *is_scrollbar_dragged.borrow_mut() = false;
            
            if let Some(mode) = *drag_mode {
                let offset = *drag_offset_ms;
                if offset != 0 {
                    match mode {
                        DragMode::Body => state.dispatch(AppAction::ShiftSelected(offset)),
                        DragMode::LeftEdge => {
                            if let Some(id) = *drag_target_id {
                                state.dispatch(AppAction::ShiftBoundary(id, true, offset));
                            }
                        }
                        DragMode::RightEdge => {
                            if let Some(id) = *drag_target_id {
                                state.dispatch(AppAction::ShiftBoundary(id, false, offset));
                            }
                        }
                    }
                }
                drag_mode.set(None);
                drag_offset_ms.set(0);
                drag_target_id.set(None);
            }
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
                            ontimeupdate={on_time_update}
                            onloadedmetadata={on_loaded_metadata}
                            onended={on_ended}
                        />
                    }
                } else {
                    html! {}
                }
            }
            <div class="transport-strip">
                <span class="timecode" ref={timecode_ref}>{ time_str }</span>
                <button class="transport-button" title={if props.state.playing { "Pause" } else { "Play" }} onclick={toggle_play}>
                    if props.state.playing {
                        <svg viewBox="0 0 24 24"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
                    } else {
                        <svg viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                    }
                </button>
                <div class="zoom-controls">
                    <button class="icon-button" title="Zoom Out" onclick={zoom_out}>
                        <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
                    </button>
                    <button class="icon-button" title="Zoom In" onclick={zoom_in}>
                        <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
                    </button>
                </div>
            </div>
            <div class="timeline-body">
                <div class="track-pads">
                    <div class="track-pad ruler-pad"></div>
                    <div class="track-pad audio-pad">
                        <button class="icon-button track-button" title="Import Audio" onclick={import_click.clone()}>
                            <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                        </button>
                    </div>
                    <div class="track-pad lyrics-pad">
                        <button class="icon-button track-button" title="Select All" onclick={select_all}>
                            <svg viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>
                        </button>
                    </div>
                </div>
                <div 
                    class="timeline-viewport" 
                    tabindex="0" 
                    ref={viewport_ref} 
                    onscroll={on_viewport_scroll}
                    onkeydown={on_keydown}
                    onmousemove={on_mousemove}
                >
                    <div class="timeline-content" style={format!("width: {}px;", width_px)} onmousedown={on_timeline_mousedown}>
                        <div class="ruler"></div>
                        <div class="track-lane audio-lane">
                            <canvas ref={canvas_ref} class="waveform-canvas"></canvas>
                            if audio_url.is_none() {
                                <div class="import-audio-button" onclick={import_click}>
                                    { "Import audio" }
                                </div>
                            }
                        </div>
                        <div class="track-lane lyrics-lane">
                            {
                                if let Some(doc) = &props.state.document {
                                    doc.timeline_chunks(duration_ms).into_iter().filter(|chunk| !chunk.is_empty()).map(|chunk| {
                                        let mut start_px = (chunk.start_ms() as f64 / 1000.0) * px_per_second;
                                        let mut end_px = (chunk.end_ms() as f64 / 1000.0) * px_per_second;
                                        
                                        let is_selected = props.state.selection.contains(chunk.entry_id());
                                        let is_drag_target = Some(chunk.entry_id()) == *drag_target_id;
                                        
                                        if let Some(mode) = *drag_mode {
                                            let offset_px = (*drag_offset_ms as f64 / 1000.0) * px_per_second;
                                            match mode {
                                                DragMode::Body => {
                                                    if is_selected {
                                                        start_px += offset_px;
                                                        end_px += offset_px;
                                                    }
                                                }
                                                DragMode::LeftEdge => {
                                                    if let Some(id) = *drag_target_id {
                                                        if id == chunk.entry_id() {
                                                            start_px += offset_px;
                                                        } else if Some(chunk.entry_id()) == doc.previous_entry_id(id) {
                                                            end_px += offset_px;
                                                        }
                                                    }
                                                }
                                                DragMode::RightEdge => {
                                                    if let Some(id) = *drag_target_id {
                                                        if id == chunk.entry_id() {
                                                            end_px += offset_px;
                                                        } else if doc.previous_entry_id(chunk.entry_id()) == Some(id) {
                                                            start_px += offset_px;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        let width = (end_px - start_px).max(1.0);
                                        
                                        let mut classes = classes!("lyric-chunk");
                                        if is_selected { classes.push("selected"); }
                                        if chunk.is_empty() { classes.push("empty-gap"); }
                                        
                                        let onmousedown_body = {
                                            let drag_mode = drag_mode.clone();
                                            let drag_start_x = drag_start_x.clone();
                                            let drag_target_id = drag_target_id.clone();
                                            let id = chunk.entry_id();
                                            let state = props.state.clone();
                                            let is_selected = is_selected;
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
                                                    state.dispatch(AppAction::SelectEntry(id, mode));
                                                }
                                                
                                                drag_mode.set(Some(DragMode::Body));
                                                drag_start_x.set(e.client_x() as f64);
                                                drag_target_id.set(Some(id));
                                            })
                                        };
                                        
                                        let onmousedown_left = {
                                            let drag_mode = drag_mode.clone();
                                            let drag_start_x = drag_start_x.clone();
                                            let drag_target_id = drag_target_id.clone();
                                            let id = chunk.entry_id();
                                            let state = props.state.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.stop_propagation();
                                                state.dispatch(AppAction::ClearSelection);
                                                drag_mode.set(Some(DragMode::LeftEdge));
                                                drag_start_x.set(e.client_x() as f64);
                                                drag_target_id.set(Some(id));
                                            })
                                        };

                                        let onmousedown_right = {
                                            let drag_mode = drag_mode.clone();
                                            let drag_start_x = drag_start_x.clone();
                                            let drag_target_id = drag_target_id.clone();
                                            let id = chunk.entry_id();
                                            let state = props.state.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.stop_propagation();
                                                state.dispatch(AppAction::ClearSelection);
                                                drag_mode.set(Some(DragMode::RightEdge));
                                                drag_start_x.set(e.client_x() as f64);
                                                drag_target_id.set(Some(id));
                                            })
                                        };

                                        html! {
                                            <div 
                                                class={classes} 
                                                style={format!("left: {}px; width: {}px;", start_px, width)}
                                                onmousedown={onmousedown_body}
                                            >
                                                { chunk.text() }
                                                <div class="edge-handle left" onmousedown={onmousedown_left}></div>
                                                <div class="edge-handle right" onmousedown={onmousedown_right}></div>
                                            </div>
                                        }
                                    }).collect::<Html>()
                                } else {
                                    html! {}
                                }
                            }
                        </div>
                        <div class="playhead" ref={playhead_ref} style={format!("transform: translateX({}px);", (props.state.current_time_ms as f64 / 1000.0) * px_per_second)}>
                            <span></span>
                        </div>
                    </div>
                </div>
            </div>
            <div class="timeline-controls">
                <div class="timeline-scroll" ref={scrollbar_ref} onscroll={on_scrollbar_scroll} onmousedown={on_scrollbar_mousedown}>
                    <div style={format!("width: {}px; height: 1px;", width_px)}></div>
                </div>
            </div>
        </div>
    }
}
