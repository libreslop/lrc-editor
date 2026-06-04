use yew::prelude::*;
use web_sys::{HtmlCanvasElement};
use wasm_bindgen::JsCast;
use crate::domain::{Pixels};
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub struct WaveformSummary {
    pub bins: Vec<(f32, f32)>, // (min, max)
    pub samples_per_bin: usize,
    pub sample_rate: f32,
}

#[derive(Properties, PartialEq)]
pub struct WaveformCanvasProps {
    pub canvas_ref: NodeRef,
    pub summary: Option<Rc<WaveformSummary>>,
    pub width: Pixels,
    pub scroll_left: f64,
    pub viewport_width: f64,
}

#[function_component(WaveformCanvas)]
pub fn waveform_canvas(props: &WaveformCanvasProps) -> Html {
    let canvas_ref = props.canvas_ref.clone();
    let summary = props.summary.clone();
    let width = props.width;
    let scroll_left = props.scroll_left;
    let viewport_width = props.viewport_width;

    use_effect_with((summary, width, scroll_left, viewport_width), move |(summary, width, scroll_left, viewport_width)| {
        if let Some(s) = summary {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                // Ensure canvas internal resolution matches display size
                let display_width = width.as_f64();
                let display_height = canvas.client_height() as f64;
                
                if canvas.width() as f64 != display_width {
                    canvas.set_width(display_width as u32);
                }
                if canvas.height() as f64 != display_height {
                    canvas.set_height(display_height as u32);
                }

                draw_waveform_tiled(&canvas, s, *scroll_left, *viewport_width);
            }
        }
        || ()
    });

    html! {
        <canvas ref={props.canvas_ref.clone()} class="waveform-canvas" style={format!("width: {}px;", width.as_f64())}></canvas>
    }
}

fn draw_waveform_tiled(canvas: &HtmlCanvasElement, summary: &WaveformSummary, scroll_left: f64, viewport_width: f64) {
    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
        
    let full_width = canvas.width() as f64;
    let height = canvas.height() as f64;
    let amp = height / 2.0;

    // Calculate visible range with 50% overscroll buffer
    let buffer = viewport_width * 0.5;
    let start_x = (scroll_left - buffer).max(0.0);
    let end_x = (scroll_left + viewport_width + buffer).min(full_width);

    // Strictly no caching: clear the entire giant canvas
    // (In a future update, we should switch to a viewport-sized canvas)
    ctx.clear_rect(0.0, 0.0, full_width, height);
    
    ctx.set_fill_style_str("#1fb7b0"); // --teal

    let total_bins = summary.bins.len() as f64;
    let bins_per_pixel = total_bins / full_width;

    for x in (start_x as usize)..(end_x as usize) {
        let bin_idx_start = (x as f64 * bins_per_pixel) as usize;
        let bin_idx_end = ((x + 1) as f64 * bins_per_pixel) as usize;
        
        let mut min = 1.0f32;
        let mut max = -1.0f32;
        
        if bin_idx_start < summary.bins.len() {
            let actual_end = bin_idx_end.min(summary.bins.len());
            if bin_idx_start == actual_end {
                // At least one bin
                let (b_min, b_max) = summary.bins[bin_idx_start];
                min = b_min;
                max = b_max;
            } else {
                for i in bin_idx_start..actual_end {
                    let (b_min, b_max) = summary.bins[i];
                    if b_min < min { min = b_min; }
                    if b_max > max { max = b_max; }
                }
            }
            
            let y = amp + (min as f64 * amp);
            let h = (max - min) as f64 * amp;
            ctx.fill_rect(x as f64, y, 1.0, h.max(1.0));
        }
    }
}
