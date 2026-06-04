use yew::prelude::*;
use crate::domain::{Pixels, TimeMs};
use crate::web_app::actions::{AppState};
use super::{WaveformCanvas, LyricChunk, DragTarget};

#[derive(Properties, PartialEq)]
pub struct TimelineLanesProps {
    pub state: UseReducerHandle<AppState>,
    pub viewport_ref: NodeRef,
    pub canvas_ref: NodeRef,
    pub playhead_ref: NodeRef,
    pub audio_url: Option<String>,
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
                        audio_url={props.audio_url.clone()} 
                        width={props.audio_width_px} 
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
                            doc.timeline_chunks(props.duration_ms).into_iter().filter(|chunk| !chunk.is_empty()).map(|chunk| {
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
                                                } else if Some(chunk.entry_id()) == doc.previous_entry_id(id) {
                                                    end_px = Pixels(end_px.as_f64() + offset_px);
                                                }
                                            }
                                        }
                                        DragTarget::RightEdge => {
                                            if let Some(id) = props.drag_target_id {
                                                if id == chunk.entry_id() {
                                                    end_px = Pixels(end_px.as_f64() + offset_px);
                                                } else if doc.previous_entry_id(chunk.entry_id()) == Some(id) {
                                                    start_px = Pixels(start_px.as_f64() + offset_px);
                                                }
                                            }
                                        }
                                        DragTarget::Playhead => {}
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

                                html! {
                                    <LyricChunk 
                                        key={chunk.entry_id()}
                                        state={props.state.clone()}
                                        entry_id={chunk.entry_id()}
                                        text={chunk.text().to_owned()}
                                        is_empty={chunk.is_empty()}
                                        start_px={start_px}
                                        width={width}
                                        is_selected={is_selected}
                                        on_drag_start={on_drag_start}
                                    />
                                }
                            }).collect::<Html>()
                        } else {
                            html! {}
                        }
                    }
                </div>
            </div>
            <div class="playhead" ref={props.playhead_ref.clone()}>
                <span></span>
            </div>
        </div>
    }
}
