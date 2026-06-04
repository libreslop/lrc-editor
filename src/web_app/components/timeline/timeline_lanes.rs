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
                            let chunks: Vec<crate::web_app::editor::timeline::Interval> = if let Some(mode) = props.drag_mode {
                                match mode {
                                    DragTarget::Body | DragTarget::LeftEdge | DragTarget::RightEdge | DragTarget::Boundary => {
                                        crate::web_app::editor::timeline::preview_intervals(
                                            doc,
                                            props.duration_ms,
                                            props.state.selection.selected_ids(),
                                            mode,
                                            props.drag_target_id,
                                            props.drag_offset_ms
                                        )
                                    }
                                    _ => doc.timeline_chunks(props.duration_ms).into_iter().map(|c| crate::web_app::editor::timeline::Interval {
                                        entry_id: c.entry_id(),
                                        uid: c.uid(),
                                        color_index: c.color_index(),
                                        start: c.start_ms(),
                                        end: c.end_ms(),
                                        raw_text: c.raw_text().to_owned(),
                                        is_empty: c.is_empty(),
                                    }).collect()
                                }
                            } else {
                                doc.timeline_chunks(props.duration_ms).into_iter().map(|c| crate::web_app::editor::timeline::Interval {
                                    entry_id: c.entry_id(),
                                    uid: c.uid(),
                                    color_index: c.color_index(),
                                    start: c.start_ms(),
                                    end: c.end_ms(),
                                    raw_text: c.raw_text().to_owned(),
                                    is_empty: c.is_empty(),
                                }).collect()
                            };

                            let mut chunk_html = Vec::new();
                            let visible_chunks: Vec<_> = chunks.into_iter().filter(|c| !c.is_empty).collect();
                            
                            for (index, chunk) in visible_chunks.iter().enumerate() {
                                let start_px = Pixels(chunk.start.to_secs() * props.px_per_second.as_f64());
                                let end_px = Pixels(chunk.end.to_secs() * props.px_per_second.as_f64());
                                
                                let is_selected = state.selection.contains(chunk.uid);
                                let width = Pixels((end_px.as_f64() - start_px.as_f64()).max(1.0));
                                let chunk_uid = chunk.uid;
                                let on_drag_start = {
                                    let on_chunk_drag_start = props.on_chunk_drag_start.clone();
                                    Callback::from(move |(e, target)| {
                                        on_chunk_drag_start.emit((chunk_uid, e, target));
                                    })
                                };

                                chunk_html.push(html! {
                                    <LyricChunk 
                                        key={chunk.uid}
                                        state={props.state.clone()}
                                        entry_id={chunk.uid}
                                        color_index={chunk.color_index}
                                        text={chunk.raw_text.to_owned()}
                                        is_empty={chunk.is_empty}
                                        start_px={start_px}
                                        width={width}
                                        is_selected={is_selected}
                                        on_drag_start={on_drag_start.clone()}
                                    />
                                });

                                // Render boundary handle between this and next if adjacent
                                if let Some(next) = visible_chunks.get(index + 1) {
                                    if !next.is_empty && next.start == chunk.end {
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
