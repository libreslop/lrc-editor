use yew::prelude::*;
use web_sys::{HtmlAudioElement, HtmlInputElement, HtmlCanvasElement, Url, AudioContext, Request, RequestInit, RequestMode, Response};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use wasm_bindgen::JsCast;
use crate::web_app::app::{AppState, AppAction};
use crate::domain::SelectionMode;

fn draw_waveform(canvas: HtmlCanvasElement, url: String) {
    spawn_local(async move {
        // We do a simple fetch to get the ArrayBuffer
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

#[function_component(TimelinePanel)]
pub fn timeline_panel(props: &TimelinePanelProps) -> Html {
    let px_per_second = 92.0;
    
    let audio_ref = use_node_ref();
    let file_input_ref = use_node_ref();
    let canvas_ref = use_node_ref();
    let audio_url = use_state(|| None::<String>);

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

    let duration_ms = props.state.duration_ms.max(10_000); 
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

    let time_str = {
        let total_secs = props.state.current_time_ms / 1000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        let ms = props.state.current_time_ms % 1000;
        format!("{:02}:{:02}.{:03}", mins, secs, ms)
    };

    html! {
        <div class="panel timeline-panel">
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
                <span class="timecode">{ time_str }</span>
                <button class="transport-button" title={if props.state.playing { "Pause" } else { "Play" }} onclick={toggle_play}>
                    if props.state.playing {
                        <svg viewBox="0 0 24 24"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
                    } else {
                        <svg viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                    }
                </button>
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
                <div class="timeline-viewport" tabindex="0">
                    <div class="timeline-content" style={format!("width: {}px;", width_px)}>
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
                                    doc.timeline_chunks(duration_ms).iter().map(|chunk| {
                                        let start_px = (chunk.start_ms() as f64 / 1000.0) * px_per_second;
                                        let end_px = (chunk.end_ms() as f64 / 1000.0) * px_per_second;
                                        let width = (end_px - start_px).max(18.0);
                                        let is_selected = props.state.selection.contains(chunk.entry_id());
                                        let is_current = doc.current_entry(props.state.current_time_ms).map(|e| e.id()) == Some(chunk.entry_id());
                                        
                                        let mut classes = classes!("lyric-chunk");
                                        if is_selected { classes.push("selected"); }
                                        if is_current { classes.push("current"); }
                                        
                                        let onclick = {
                                            let state = props.state.clone();
                                            let id = chunk.entry_id();
                                            Callback::from(move |e: MouseEvent| {
                                                let mode = if e.shift_key() {
                                                    SelectionMode::Range
                                                } else if e.ctrl_key() || e.meta_key() {
                                                    SelectionMode::Toggle
                                                } else {
                                                    SelectionMode::Replace
                                                };
                                                state.dispatch(AppAction::SelectEntry(id, mode));
                                            })
                                        };

                                        html! {
                                            <div 
                                                class={classes} 
                                                style={format!("left: {}px; width: {}px;", start_px, width)}
                                                {onclick}
                                            >
                                                { chunk.text() }
                                            </div>
                                        }
                                    }).collect::<Html>()
                                } else {
                                    html! {}
                                }
                            }
                        </div>
                        <div class="playhead" style={format!("transform: translateX({}px);", (props.state.current_time_ms as f64 / 1000.0) * px_per_second)}>
                            <span></span>
                        </div>
                    </div>
                </div>
            </div>
            <div class="timeline-controls">
                <input type="range" class="timeline-scroll" min="0" max="100" value="0" />
                <div class="zoom-controls">
                    <button class="icon-button" title="Zoom Out">
                        <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
                    </button>
                    <button class="icon-button" title="Zoom In">
                        <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
                    </button>
                </div>
            </div>
        </div>
    }
}
