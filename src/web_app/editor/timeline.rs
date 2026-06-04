use crate::domain::{LrcDocument, TimeMs};

#[derive(Clone, Debug)]
pub struct Interval {
    pub start: TimeMs,
    pub end: TimeMs,
    pub raw_text: String,
    pub is_empty: bool,
}

pub fn shift_selected(doc: &LrcDocument, selected_ids: &[usize], delta_ms: i32, duration_ms: TimeMs) -> String {
    let chunks = doc.timeline_chunks(duration_ms);
    
    let mut moved = Vec::new();
    let mut statics = Vec::new();
    
    for c in chunks {
        let mut i = Interval {
            start: c.start_ms(),
            end: c.end_ms(),
            raw_text: c.raw_text().to_string(),
            is_empty: c.is_empty(),
        };
        
        if selected_ids.contains(&c.entry_id()) {
            i.start = TimeMs((i.start.as_u32() as i32 + delta_ms).max(0) as u32);
            i.end = TimeMs((i.end.as_u32() as i32 + delta_ms).max(0) as u32);
            moved.push(i);
        } else {
            statics.push(i);
        }
    }
    
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
    
    build_lrc(doc, final_intervals)
}

pub fn shift_boundary(doc: &LrcDocument, chunk_id: usize, left_edge: bool, delta_ms: i32, duration_ms: TimeMs) -> String {
    let chunks = doc.timeline_chunks(duration_ms);
    let mut intervals = Vec::new();
    
    let mut boundary_time = None;
    
    for c in &chunks {
        if c.entry_id() == chunk_id {
            boundary_time = Some(if left_edge { c.start_ms() } else { c.end_ms() });
        }
        intervals.push(Interval {
            start: c.start_ms(),
            end: c.end_ms(),
            raw_text: c.raw_text().to_string(),
            is_empty: c.is_empty(),
        });
    }
    
    if let Some(t) = boundary_time {
        let new_t = TimeMs((t.as_u32() as i32 + delta_ms).max(0) as u32);
        
        for i in &mut intervals {
            if i.start == t {
                i.start = new_t;
            }
            if i.end == t {
                i.end = new_t;
            }
        }
        
        // Remove collapsed intervals
        intervals.retain(|i| i.end > i.start);
    }
    
    intervals.sort_by_key(|i| i.start);
    build_lrc(doc, intervals)
}

fn build_lrc(doc: &LrcDocument, final_intervals: Vec<Interval>) -> String {
    let mut resolved = Vec::new();
    let mut current_time = final_intervals.first().map(|i| i.start).unwrap_or(TimeMs(0));

    for i in final_intervals {
        if i.start > current_time {
            resolved.push(Interval {
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
    for tag in doc.metadata() {
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
