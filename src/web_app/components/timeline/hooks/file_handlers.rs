use yew::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, Url, AudioContext, RequestInit, RequestMode, Request, Response};

use crate::web_app::actions::{AppState, AppAction};
use crate::web_app::components::timeline::waveform_canvas::WaveformSummary;

pub struct FileHandlers {
    pub on_file_change: Callback<Event>,
    pub import_click: Callback<MouseEvent>,
    pub import_lrc_click: Callback<MouseEvent>,
    pub on_lrc_change: Callback<Event>,
    pub export_lrc: Callback<MouseEvent>,
    pub on_loaded_metadata: Callback<Event>,
}

#[hook]
pub fn use_file_handlers(
    state: UseReducerHandle<AppState>,
    audio_url: UseStateHandle<Option<String>>,
    waveform_summary: UseStateHandle<Option<Rc<WaveformSummary>>>,
    file_input_ref: NodeRef,
    lrc_input_ref: NodeRef,
) -> FileHandlers {
    let on_file_change = {
        let audio_url = audio_url.clone();
        let waveform_summary = waveform_summary.clone();
        let state = state.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    state.dispatch(AppAction::SetAudioFilename(file.name()));
                    if let Ok(url) = Url::create_object_url_with_blob(&file) {
                        audio_url.set(Some(url.clone()));
                        
                        let waveform_summary = waveform_summary.clone();
                        spawn_local(async move {
                            // Fetch as array buffer and decode
                            let opts = RequestInit::new();
                            opts.set_method("GET");
                            opts.set_mode(RequestMode::Cors);
                            if let Ok(request) = Request::new_with_str_and_init(&url, &opts) {
                                let window = web_sys::window().unwrap();
                                if let Ok(resp_value) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await {
                                    let resp: Response = resp_value.dyn_into().unwrap();
                                    if let Ok(array_buffer) = wasm_bindgen_futures::JsFuture::from(resp.array_buffer().unwrap()).await {
                                        if let Ok(audio_ctx) = AudioContext::new() {
                                            if let Ok(audio_buffer_promise) = audio_ctx.decode_audio_data(&array_buffer.into()) {
                                                if let Ok(audio_buffer_value) = wasm_bindgen_futures::JsFuture::from(audio_buffer_promise).await {
                                                    let audio_buffer: web_sys::AudioBuffer = audio_buffer_value.dyn_into().unwrap();
                                                    waveform_summary.set(Some(Rc::new(crate::web_app::components::timeline_panel::downsample_audio(audio_buffer))));
                                                }
                                            }
                                        }
                                    }
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
        let state = state.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    state.dispatch(AppAction::SetLrcFilename(file.name()));
                    let state = state.clone();
                    spawn_local(async move {
                        let text_promise = file.text();
                        if let Ok(text_value) = wasm_bindgen_futures::JsFuture::from(text_promise).await {
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
        let state = state.clone();
        Callback::from(move |_| {
            let text = state.document.source_text.clone();
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            
            let filename = if let Some(audio_name) = &state.project.audio_filename {
                let base = audio_name.rfind('.').map(|i| &audio_name[..i]).unwrap_or(audio_name);
                format!("{}.lrc", base)
            } else if let Some(lrc_name) = &state.project.lrc_filename {
                lrc_name.clone()
            } else {
                "lyrics.lrc".to_string()
            };

            if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&text.into())) {
                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                    if let Ok(a) = document.create_element("a") {
                        if let Ok(a) = a.dyn_into::<web_sys::HtmlAnchorElement>() {
                            a.set_href(&url);
                            a.set_download(&filename);
                            a.click();
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        })
    };

    let on_loaded_metadata = {
        let state = state.clone();
        Callback::from(move |e: Event| {
            if let Some(audio) = e.target_dyn_into::<web_sys::HtmlAudioElement>() {
                state.dispatch(AppAction::SetDuration(crate::domain::TimeMs((audio.duration() * 1000.0) as u32)));
                state.dispatch(AppAction::Seek(state.playback.current_time_ms));
            }
        })
    };

    FileHandlers {
        on_file_change,
        import_click,
        import_lrc_click,
        on_lrc_change,
        export_lrc,
        on_loaded_metadata,
    }
}
