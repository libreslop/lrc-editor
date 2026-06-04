use crate::domain::{LrcDocument, TimeMs};
use crate::web_app::components::timeline::DragTarget;

/// Represents an interval of text on the timeline.
#[derive(Clone, Debug)]
pub struct Interval {
    pub entry_id: usize,
    pub uid: usize,
    pub color_index: u8,
    pub start: TimeMs,
    pub end: TimeMs,
    pub raw_text: String,
    pub is_empty: bool,
}

/// Provides functionality for editing the timeline by dragging or shifting chunks.
pub struct TimelineEditor<'a> {
    doc: &'a LrcDocument,
}

impl<'a> TimelineEditor<'a> {
    /// Creates a new TimelineEditor for the given document.
    pub fn new(doc: &'a LrcDocument) -> Self {
        Self { doc }
    }

    /// Shifts selected chunks by a given delta in milliseconds.
    pub fn shift_selected(&self, selected_uids: &[usize], delta_ms: i32, duration_ms: TimeMs) -> String {
        let intervals = self.preview_intervals(duration_ms, selected_uids, DragTarget::Body, None, delta_ms);
        self.build_lrc(intervals)
    }

    /// Shifts the boundary of a specific chunk.
    pub fn shift_boundary(&self, chunk_uid: usize, left_edge: bool, both: bool, delta_ms: i32, duration_ms: TimeMs) -> String {
        let mode = if both {
            DragTarget::Boundary
        } else if left_edge {
            DragTarget::LeftEdge
        } else {
            DragTarget::RightEdge
        };
        
        let intervals = self.preview_intervals(duration_ms, &[], mode, Some(chunk_uid), delta_ms);
        self.build_lrc(intervals)
    }

    pub fn preview_intervals(
        &self,
        duration_ms: TimeMs,
        selected_uids: &[usize],
        drag_mode: DragTarget,
        drag_target_uid: Option<usize>,
        drag_offset_ms: i32,
    ) -> Vec<Interval> {
        let chunks = self.doc.timeline_chunks(duration_ms);
        let mut moved = Vec::new();
        let mut statics = Vec::new();

        let mut moved_uids = Vec::new();
        match drag_mode {
            DragTarget::Body => moved_uids.extend_from_slice(selected_uids),
            DragTarget::LeftEdge | DragTarget::RightEdge => {
                if let Some(uid) = drag_target_uid {
                    moved_uids.push(uid);
                }
            }
            DragTarget::Boundary => {
                if let Some(uid) = drag_target_uid {
                    moved_uids.push(uid);
                    if let Some(next_uid) = self.doc.next_entry_uid(uid) {
                        moved_uids.push(next_uid);
                    }
                }
            }
            _ => {}
        }

        for c in chunks {
            let mut i = Interval {
                entry_id: c.entry_id(),
                uid: c.uid(),
                color_index: c.color_index(),
                start: c.start_ms(),
                end: c.end_ms(),
                raw_text: c.raw_text().to_string(),
                is_empty: c.is_empty(),
            };

            if moved_uids.contains(&c.uid()) {
                match drag_mode {
                    DragTarget::Body => {
                        i.start = TimeMs((i.start.as_u32() as i32 + drag_offset_ms).max(0) as u32);
                        i.end = TimeMs((i.end.as_u32() as i32 + drag_offset_ms).max(0) as u32);
                    }
                    DragTarget::LeftEdge => {
                        if Some(c.uid()) == drag_target_uid {
                            i.start = TimeMs((i.start.as_u32() as i32 + drag_offset_ms).max(0) as u32);
                        }
                    }
                    DragTarget::RightEdge => {
                        if Some(c.uid()) == drag_target_uid {
                            i.end = TimeMs((i.end.as_u32() as i32 + drag_offset_ms).max(0) as u32);
                        }
                    }
                    DragTarget::Boundary => {
                        if Some(c.uid()) == drag_target_uid {
                            // Dragging end of this chunk
                            i.end = TimeMs((i.end.as_u32() as i32 + drag_offset_ms).max(0) as u32);
                        } else {
                            // Dragging start of the next chunk
                            i.start = TimeMs((i.start.as_u32() as i32 + drag_offset_ms).max(0) as u32);
                        }
                    }
                    _ => {}
                }
                if i.end > i.start {
                    moved.push(i);
                }
            } else {
                statics.push(i);
            }
        }

        Self::resolve_overlaps(statics, moved)
    }

    fn resolve_overlaps(statics: Vec<Interval>, moved: Vec<Interval>) -> Vec<Interval> {
        let mut next_static = Vec::new();
        for st in statics {
            let mut fragments = vec![st];
            for m in &moved {
                let mut new_fragments = Vec::new();
                for f in fragments {
                    if m.start <= f.start && m.end >= f.end {
                        // fully covered, delete
                    } else if m.start > f.start && m.end < f.end {
                        // split
                        let mut left = f.clone();
                        left.end = m.start;
                        let mut right = f.clone();
                        right.start = m.end;
                        new_fragments.push(left);
                        new_fragments.push(right);
                    } else if m.start <= f.start && m.end > f.start {
                        // overlap left
                        let mut right = f.clone();
                        right.start = m.end;
                        new_fragments.push(right);
                    } else if m.start < f.end && m.end >= f.end {
                        // overlap right
                        let mut left = f.clone();
                        left.end = m.start;
                        new_fragments.push(left);
                    } else {
                        new_fragments.push(f);
                    }
                }
                fragments = new_fragments;
            }
            next_static.extend(fragments);
        }
        
        let mut final_intervals = next_static;
        final_intervals.extend(moved);
        final_intervals.sort_by_key(|i| i.start);
        final_intervals
    }

    fn build_lrc(&self, final_intervals: Vec<Interval>) -> String {
        let mut resolved = Vec::new();
        let mut current_time = final_intervals.first().map(|i| i.start).unwrap_or(TimeMs(0));

        for i in final_intervals {
            if i.start > current_time {
                resolved.push(Interval {
                    entry_id: 0,
                    uid: 0,
                    color_index: 0,
                    start: current_time,
                    end: i.start,
                    raw_text: String::new(),
                    is_empty: true,
                });
            }
            resolved.push(i.clone());
            current_time = i.end;
        }

        let mut merged: Vec<Interval> = Vec::new();
        for i in resolved {
            if let Some(last) = merged.last_mut() {
                if last.is_empty && i.is_empty {
                    last.end = i.end;
                    continue;
                }
            }
            merged.push(i);
        }

        merged.retain(|i| i.end > i.start);

        if let Some(last) = merged.last() {
            if last.is_empty {
                merged.pop();
            }
        }

        let mut text = String::new();
        for tag in self.doc.metadata() {
            text.push_str(&format!("[{}:{}]\n", tag.key(), tag.value()));
        }
        
        for i in &merged {
            text.push_str(&format!("[{}]{}\n", i.start.as_timestamp(), i.raw_text));
        }
        
        if let Some(last) = merged.last() {
            text.push_str(&format!("[{}]\n", last.end.as_timestamp()));
        }

        text
    }
}
