use yew::prelude::*;
use crate::domain::{Pixels, TimeMs};
use crate::web_app::actions::{AppState};
use super::{WaveformCanvas, LyricChunk, DragTarget};
use super::waveform_canvas::WaveformSummary;
use std::rc::Rc;

#[derive(Properties, PartialEq)]
pub struct TimelineLanesProps {
    pub state: UseReducerHandle<AppState>,
    pub viewport_ref: NodeRef,
    pub canvas_ref: NodeRef,
    pub playhead_ref: NodeRef,
    pub audio_url: Option<String>,
    pub waveform_summary: Option<Rc<WaveformSummary>>,
    pub scroll_left: f64,
    pub viewport_width: f64,
    pub duration_ms: TimeMs,
    pub width_px: Pixels,
    pub audio_width_px: Pixels,
    pub px_per_second: Pixels,
    pub drag_mode: Option<DragTarget>,
    pub drag_offset_ms: i32,
    pub drag_target_id: Option<usize>,
    pub on_viewport_scroll: Callback<Event>,
    pub on_keydown: Callback<KeyboardEvent>,
    pub on_mousemove: Callback<MouseEvent>,
    pub on_mousedown_content: Callback<MouseEvent>,
    pub on_mousedown_ruler: Callback<MouseEvent>,
    pub on_import_audio: Callback<MouseEvent>,
    pub on_chunk_drag_start: Callback<(usize, MouseEvent, DragTarget)>,
    pub selection_rect: Option<(f64, f64, f64, f64)>,
}

#[function_component(TimelineLanes)]
pub fn timeline_lanes(props: &TimelineLanesProps) -> Html {
    let state = &props.state;
    let doc = state.document.as_ref();

    html! {
        <div 
            class="timeline-viewport" 
            tabindex="0" 
            ref={props.viewport_ref.clone()} 
            onscroll={props.on_viewport_scroll.clone()}
            onkeydown={props.on_keydown.clone()}
            onmousemove={props.on_mousemove.clone()}
        >
            <div class="timeline-content" style={format!("width: {}px;", props.width_px.as_f64())} onmousedown={props.on_mousedown_content.clone()}>
                <div class="ruler" onmousedown={props.on_mousedown_ruler.clone()}></div>
                <div class="track-lane audio-lane">
                    <WaveformCanvas 
                        canvas_ref={props.canvas_ref.clone()} 
                        summary={props.waveform_summary.clone()} 
                        width={props.audio_width_px} 
                        scroll_left={props.scroll_left}
                        viewport_width={props.viewport_width}
                    />
                    if props.audio_url.is_none() {
                        <div class="import-audio-button" onclick={props.on_import_audio.clone()}>
                            { "Import audio" }
                        </div>
                    }
                </div>
                <div class="track-lane lyrics-lane">
                    {
                        if let Some(doc) = doc {
                            let chunks: Vec<_> = doc.timeline_chunks(props.duration_ms).into_iter().filter(|c| !c.is_empty()).collect();
                            let mut chunk_html = Vec::new();
                            
                            for (index, chunk) in chunks.iter().enumerate() {
                                let mut start_px = Pixels(chunk.start_ms().to_secs() * props.px_per_second.as_f64());
                                let mut end_px = Pixels(chunk.end_ms().to_secs() * props.px_per_second.as_f64());
                                
                                let is_selected = state.selection.contains(chunk.entry_id());
                                
                                if let Some(mode) = props.drag_mode {
                                    let offset_px = (props.drag_offset_ms as f64 / 1000.0) * props.px_per_second.as_f64();
                                    match mode {
                                        DragTarget::Body => {
                                            if is_selected {
                                                start_px = Pixels(start_px.as_f64() + offset_px);
                                                end_px = Pixels(end_px.as_f64() + offset_px);
                                            }
                                        }
                                        DragTarget::LeftEdge => {
                                            if let Some(id) = props.drag_target_id {
                                                if id == chunk.entry_id() {
                                                    start_px = Pixels(start_px.as_f64() + offset_px);
                                                }
                                            }
                                        }
                                        DragTarget::RightEdge => {
                                            if let Some(id) = props.drag_target_id {
                                                if id == chunk.entry_id() {
                                                    end_px = Pixels(end_px.as_f64() + offset_px);
                                                }
                                            }
                                        }
                                        DragTarget::Boundary => {
                                            if let Some(id) = props.drag_target_id {
                                                if id == chunk.entry_id() {
                                                    // Boundary after this chunk
                                                    end_px = Pixels(end_px.as_f64() + offset_px);
                                                } else if doc.previous_entry_id(chunk.entry_id()) == Some(id) {
                                                    // This chunk is after the boundary
                                                    start_px = Pixels(start_px.as_f64() + offset_px);
                                                }
                                            }
                                        }
                                        DragTarget::Playhead | DragTarget::Selection => {}
                                    }
                                }

                                let width = Pixels((end_px.as_f64() - start_px.as_f64()).max(1.0));
                                let chunk_id = chunk.entry_id();
                                let on_drag_start = {
                                    let on_chunk_drag_start = props.on_chunk_drag_start.clone();
                                    Callback::from(move |(e, target)| {
                                        on_chunk_drag_start.emit((chunk_id, e, target));
                                    })
                                };

                                chunk_html.push(html! {
                                    <LyricChunk 
                                        key={chunk.entry_id()}
                                        state={props.state.clone()}
                                        entry_id={chunk.entry_id()}
                                        text={chunk.text().to_owned()}
                                        is_empty={chunk.is_empty()}
                                        start_px={start_px}
                                        width={width}
                                        is_selected={is_selected}
                                        on_drag_start={on_drag_start.clone()}
                                    />
                                });

                                // Render boundary handle between this and next if adjacent
                                if let Some(next) = chunks.get(index + 1) {
                                    if !next.is_empty() && next.start_ms() == chunk.end_ms() {
                                        let b_pos = end_px;
                                        let on_b_mousedown = {
                                            let on_drag_start = on_drag_start.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.stop_propagation();
                                                on_drag_start.emit((e, DragTarget::Boundary));
                                            })
                                        };
                                        chunk_html.push(html! {
                                            <div 
                                                class="boundary-handle" 
                                                style={format!("left: {}px;", b_pos.as_f64() - 4.0)}
                                                onmousedown={on_b_mousedown}
                                            ></div>
                                        });
                                    }
                                }
                            }
                            chunk_html.into_iter().collect::<Html>()
                        } else {
                            html! {}
                        }
                    }
                </div>
            </div>
            {
                if let Some((x, y, w, h)) = props.selection_rect {
                    html! {
                        <div 
                            class="selection-rect" 
                            style={format!("left: {}px; top: {}px; width: {}px; height: {}px;", x, y, w, h)}
                        ></div>
                    }
                } else {
                    html! {}
                }
            }
            <div class="playhead" ref={props.playhead_ref.clone()}>
                <span></span>
            </div>
        </div>
    }
}
