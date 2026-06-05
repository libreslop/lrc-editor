use crate::domain::{LrcDocument, TimeMs, Pixels};
use crate::web_app::components::timeline::DragTarget;
use crate::web_app::actions::AppState;

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

    /// Adds a new chunk to the timeline track.
    pub fn add_chunk(&self, start: TimeMs, end: TimeMs, duration_ms: TimeMs) -> String {
        let chunks = self.doc.timeline_chunks(duration_ms);
        let mut statics = Vec::new();
        
        for c in chunks {
            if !c.is_empty() {
                statics.push(Interval {
                    entry_id: c.entry_id(),
                    uid: c.uid(),
                    color_index: c.color_index(),
                    start: c.start_ms(),
                    end: c.end_ms(),
                    raw_text: c.raw_text().to_string(),
                    is_empty: c.is_empty(),
                });
            }
        }
        
        let new_chunk = Interval {
            entry_id: 0,
            uid: 0,
            color_index: 0,
            start,
            end,
            raw_text: "Change me".to_string(),
            is_empty: false,
        };
        
        let mut resolved = Self::resolve_overlaps(statics, vec![new_chunk]);
        
        // Find the color index (0..6) with the furthest neighbors of the same color
        let mut best_color = 0;
        let mut max_dist = -1;
        
        let non_empty: Vec<&Interval> = resolved.iter().filter(|i| !i.is_empty).collect();
        if let Some(new_idx) = non_empty.iter().position(|i| i.uid == 0) {
            for c in 0..6 {
                let mut left_dist = i32::MAX;
                for l in (0..new_idx).rev() {
                    if non_empty[l].color_index == c {
                        left_dist = (new_idx - l) as i32;
                        break;
                    }
                }
                
                let mut right_dist = i32::MAX;
                for r in (new_idx + 1)..non_empty.len() {
                    if non_empty[r].color_index == c {
                        right_dist = (r - new_idx) as i32;
                        break;
                    }
                }
                
                let min_dist = left_dist.min(right_dist);
                if min_dist > max_dist {
                    max_dist = min_dist;
                    best_color = c;
                }
            }
        }
        
        // Assign the best color to our new chunk
        for i in &mut resolved {
            if i.uid == 0 {
                i.color_index = best_color;
            }
        }
        
        self.build_lrc(resolved)
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
        
        let mut first_non_empty_seen = false;
        for i in &merged {
            let trimmed = i.raw_text.trim_start();
            if trimmed.is_empty() {
                if first_non_empty_seen {
                    text.push_str(&format!("[{}]\n", i.start.as_timestamp()));
                }
            } else {
                first_non_empty_seen = true;
                text.push_str(&format!("[{}] {}\n", i.start.as_timestamp(), trimmed));
            }
        }
        
        if let Some(last) = merged.last() {
            if first_non_empty_seen {
                text.push_str(&format!("[{}]\n", last.end.as_timestamp()));
            }
        }

        text
    }
}

/// A utility for calculating snapping positions on the timeline.
#[derive(Debug)]
pub struct TimelineSnapper;

impl TimelineSnapper {
    /// Snaps a playhead time to the nearest chunk boundary or timeline edge if within the visual threshold.
    pub fn snap_playhead(
        state: &AppState,
        target_time: TimeMs,
        duration_ms: TimeMs,
        px_per_second: Pixels,
    ) -> TimeMs {
        let px_per_ms = px_per_second.as_f64() / 1000.0;
        let snap_threshold_px = 10.0;
        let snap_threshold_ms = (snap_threshold_px / px_per_ms) as i32;

        let mut snap_points = Vec::new();
        snap_points.push(0);
        snap_points.push(duration_ms.as_u32());
        snap_points.push(state.playback.duration_ms.as_u32());
        if let Some(doc) = &state.document.document {
            let chunks = doc.timeline_chunks(duration_ms);
            for chunk in chunks {
                snap_points.push(chunk.start_ms().as_u32());
                snap_points.push(chunk.end_ms().as_u32());
            }
        }

        let target_ms = target_time.as_u32() as i32;
        let mut best_adjust: Option<i32> = None;

        for p in snap_points {
            let adjust = p as i32 - target_ms;
            if best_adjust.is_none() || adjust.abs() < best_adjust.unwrap().abs() {
                best_adjust = Some(adjust);
            }
        }

        if let Some(adjust) = best_adjust {
            if adjust.abs() <= snap_threshold_ms {
                return TimeMs((target_ms + adjust).max(0) as u32);
            }
        }

        target_time
    }

    /// Snaps a drag offset delta based on the current drag target and static timeline points.
    pub fn snap_drag_offset(
        state: &AppState,
        drag_mode: DragTarget,
        drag_target_uid: Option<usize>,
        raw_offset_ms: i32,
        duration_ms: TimeMs,
        px_per_second: Pixels,
    ) -> i32 {
        let px_per_ms = px_per_second.as_f64() / 1000.0;
        let snap_threshold_px = 10.0;
        let snap_threshold_ms = (snap_threshold_px / px_per_ms) as i32;

        let mut moved_uids = Vec::new();
        match drag_mode {
            DragTarget::Body => {
                moved_uids.extend(state.view.selection.selected_ids());
            }
            DragTarget::LeftEdge | DragTarget::RightEdge => {
                if let Some(uid) = drag_target_uid {
                    moved_uids.push(uid);
                }
            }
            DragTarget::Boundary => {
                if let Some(uid) = drag_target_uid {
                    moved_uids.push(uid);
                    if let Some(doc) = &state.document.document {
                        if let Some(next_uid) = doc.next_entry_uid(uid) {
                            moved_uids.push(next_uid);
                        }
                    }
                }
            }
            _ => return raw_offset_ms,
        }

        // Get static snap points (boundaries of all chunks not currently moving, plus timeline edges, playhead position, and audio end)
        let mut static_points = Vec::new();
        static_points.push(0);
        static_points.push(duration_ms.as_u32());
        static_points.push(state.playback.current_time_ms.as_u32());
        static_points.push(state.playback.duration_ms.as_u32());
        if let Some(doc) = &state.document.document {
            let chunks = doc.timeline_chunks(duration_ms);
            for chunk in chunks {
                if !moved_uids.contains(&chunk.uid()) {
                    static_points.push(chunk.start_ms().as_u32());
                    static_points.push(chunk.end_ms().as_u32());
                }
            }
        }
        static_points.sort();
        static_points.dedup();

        // Get moving edges that we want to snap
        let mut moving_edges = Vec::new();
        if let Some(doc) = &state.document.document {
            let chunks = doc.timeline_chunks(duration_ms);
            for chunk in chunks {
                if moved_uids.contains(&chunk.uid()) {
                    match drag_mode {
                        DragTarget::Body => {
                            moving_edges.push(chunk.start_ms().as_u32() as i32 + raw_offset_ms);
                            moving_edges.push(chunk.end_ms().as_u32() as i32 + raw_offset_ms);
                        }
                        DragTarget::LeftEdge => {
                            if Some(chunk.uid()) == drag_target_uid {
                                moving_edges.push(chunk.start_ms().as_u32() as i32 + raw_offset_ms);
                            }
                        }
                        DragTarget::RightEdge => {
                            if Some(chunk.uid()) == drag_target_uid {
                                moving_edges.push(chunk.end_ms().as_u32() as i32 + raw_offset_ms);
                            }
                        }
                        DragTarget::Boundary => {
                            if Some(chunk.uid()) == drag_target_uid {
                                moving_edges.push(chunk.end_ms().as_u32() as i32 + raw_offset_ms);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let mut best_adjust: Option<i32> = None;
        for &edge in &moving_edges {
            for &p in &static_points {
                let adjust = p as i32 - edge;
                if best_adjust.is_none() || adjust.abs() < best_adjust.unwrap().abs() {
                    best_adjust = Some(adjust);
                }
            }
        }

        if let Some(adjust) = best_adjust {
            if adjust.abs() <= snap_threshold_ms {
                return raw_offset_ms + adjust;
            }
        }

        raw_offset_ms
    }
}

/// Helper function to find the gap containing the given time.
pub fn find_gap(doc: Option<&LrcDocument>, t: TimeMs, duration_ms: TimeMs) -> Option<(TimeMs, TimeMs)> {
    if let Some(doc) = doc {
        let entries = doc.entries();
        if entries.is_empty() {
            return Some((TimeMs(0), duration_ms));
        }
        
        let first_start = entries[0].time_ms();
        if t < first_start {
            return Some((TimeMs(0), first_start));
        }
        
        let chunks = doc.timeline_chunks(duration_ms);
        for chunk in chunks {
            if t >= chunk.start_ms() && t < chunk.end_ms() {
                if chunk.is_empty() {
                    return Some((chunk.start_ms(), chunk.end_ms()));
                } else {
                    return None;
                }
            }
        }
        
        let last_end = entries.last().unwrap().time_ms();
        if t >= last_end && t < duration_ms {
            return Some((last_end, duration_ms));
        }
    } else {
        return Some((TimeMs(0), duration_ms));
    }
    None
}

/// Helper function to calculate the ghost chunk boundaries.
pub fn calculate_ghost_chunk(
    state: &AppState,
    t: TimeMs,
    gap_start: TimeMs,
    gap_end: TimeMs,
    duration_ms: TimeMs,
    px_per_second: Pixels,
) -> (TimeMs, TimeMs) {
    let gap_len = gap_end.as_u32() as i32 - gap_start.as_u32() as i32;
    if gap_len <= 5000 {
        return (gap_start, gap_end);
    }
    
    let t_val = t.as_u32() as i32;
    let mut ghost_start = t_val - 2500;
    let mut ghost_end = t_val + 2500;
    
    let gap_start_val = gap_start.as_u32() as i32;
    let gap_end_val = gap_end.as_u32() as i32;
    
    if ghost_start < gap_start_val {
        ghost_start = gap_start_val;
        ghost_end = gap_start_val + 5000;
    } else if ghost_end > gap_end_val {
        ghost_end = gap_end_val;
        ghost_start = gap_end_val - 5000;
    }
    
    let start_time = TimeMs(ghost_start.max(0) as u32);
    let end_time = TimeMs(ghost_end.max(0) as u32);
    
    let snapped_start = TimelineSnapper::snap_playhead(
        state,
        start_time,
        duration_ms,
        px_per_second,
    );
    let snapped_end = TimelineSnapper::snap_playhead(
        state,
        end_time,
        duration_ms,
        px_per_second,
    );
    
    let final_start = snapped_start.max(gap_start).min(gap_end);
    let final_end = snapped_end.max(gap_start).min(gap_end);
    
    if final_start < final_end {
        (final_start, final_end)
    } else {
        (start_time, end_time)
    }
}

