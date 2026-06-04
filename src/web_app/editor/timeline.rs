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
    
    let final_intervals = resolve_overlaps(statics, moved);
    build_lrc(doc, final_intervals)
}

pub fn shift_boundary(doc: &LrcDocument, chunk_id: usize, left_edge: bool, both: bool, delta_ms: i32, duration_ms: TimeMs) -> String {
    let chunks = doc.timeline_chunks(duration_ms);
    
    let mut moved = Vec::new();
    let mut statics = Vec::new();
    
    // Determine which chunks are "moved" (in terms of boundary change)
    let mut moved_ids = vec![chunk_id];
    if both {
        if left_edge {
            if let Some(prev_id) = doc.previous_entry_id(chunk_id) {
                moved_ids.push(prev_id);
            }
        } else {
            if let Some(next_id) = doc.next_entry_id(chunk_id) {
                moved_ids.push(next_id);
            }
        }
    }
    
    for c in chunks {
        let mut i = Interval {
            start: c.start_ms(),
            end: c.end_ms(),
            raw_text: c.raw_text().to_string(),
            is_empty: c.is_empty(),
        };
        
        if moved_ids.contains(&c.entry_id()) {
            if c.entry_id() == chunk_id {
                if left_edge {
                    i.start = TimeMs((i.start.as_u32() as i32 + delta_ms).max(0) as u32);
                } else {
                    i.end = TimeMs((i.end.as_u32() as i32 + delta_ms).max(0) as u32);
                }
            } else {
                // The other chunk in a "both" move
                if left_edge {
                    // moving start of chunk_id. This is the end of the previous chunk.
                    i.end = TimeMs((i.end.as_u32() as i32 + delta_ms).max(0) as u32);
                } else {
                    // moving end of chunk_id. This is the start of the next chunk.
                    i.start = TimeMs((i.start.as_u32() as i32 + delta_ms).max(0) as u32);
                }
            }
            // Only push if still valid (start < end)
            // But wait, if it's collapsed, we might still want to keep it as a "mover" that deletes others?
            // Actually, if it's collapsed, it shouldn't exist anymore.
            if i.end > i.start {
                moved.push(i);
            }
        } else {
            statics.push(i);
        }
    }
    
    let final_intervals = resolve_overlaps(statics, moved);
    build_lrc(doc, final_intervals)
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
