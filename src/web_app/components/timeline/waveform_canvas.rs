use yew::prelude::*;
use web_sys::{HtmlCanvasElement, AudioContext, Request, RequestInit, RequestMode, Response};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use wasm_bindgen::JsCast;
use crate::domain::Pixels;

#[derive(Properties, PartialEq)]
pub struct WaveformCanvasProps {
    pub canvas_ref: NodeRef,
    pub audio_url: Option<String>,
    pub width: Pixels,
}

#[function_component(WaveformCanvas)]
pub fn waveform_canvas(props: &WaveformCanvasProps) -> Html {
    let canvas_ref = props.canvas_ref.clone();
    let url = props.audio_url.clone();
    let width = props.width;

    use_effect_with((url.clone(), width), move |(url, width)| {
        if let Some(u) = url {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                canvas.set_width(width.as_f64() as u32);
                canvas.set_height(canvas.client_height() as u32);
                draw_waveform(canvas, u.clone());
            }
        }
        || ()
    });

    html! {
        <canvas ref={props.canvas_ref.clone()} class="waveform-canvas" style={format!("width: {}px;", width.as_f64())}></canvas>
    }
}

fn draw_waveform(canvas: HtmlCanvasElement, url: String) {
    spawn_local(async move {
        let opts = RequestInit::new();
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
