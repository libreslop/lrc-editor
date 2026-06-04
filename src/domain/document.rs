use crate::domain::entry::{LyricEntry, TimelineChunk};
use crate::domain::metadata::MetadataTag;
use crate::domain::time::TimeMs;

/// Parsed LRC document state used by the UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LrcDocument {
    entries: Vec<LyricEntry>,
    metadata: Vec<MetadataTag>,
    line_count: usize,
}

impl LrcDocument {
    pub(crate) fn new(mut entries: Vec<LyricEntry>, metadata: Vec<MetadataTag>, line_count: usize) -> Self {
        if entries.iter().all(|e| e.is_empty()) {
            entries.clear();
        }
        Self {
            entries,
            metadata,
            line_count,
        }
    }

    /// Timed entries sorted by timestamp.
    pub fn entries(&self) -> &[LyricEntry] {
        &self.entries
    }

    /// Mutable access for identity reconciliation.
    pub fn entries_mut(&mut self) -> &mut Vec<LyricEntry> {
        &mut self.entries
    }

    /// Untimed metadata tags.
    pub fn metadata(&self) -> &[MetadataTag] {
        &self.metadata
    }

    /// Number of source lines.
    pub fn line_count(&self) -> usize {
        self.line_count
    }

    /// Last timed lyric position, if any.
    pub fn last_entry_time_ms(&self) -> Option<TimeMs> {
        self.entries.last().map(LyricEntry::time_ms)
    }

    /// Find an entry by stable uid.
    pub fn entry_by_uid(&self, uid: usize) -> Option<&LyricEntry> {
        self.entries.iter().find(|entry| entry.uid() == uid)
    }

    /// Entry active at `time_ms`, using standard LRC "latest timestamp wins" behavior.
    pub fn current_entry(&self, time_ms: TimeMs) -> Option<&LyricEntry> {
        if self.entries.first().is_none_or(|entry| time_ms < entry.time_ms()) {
            return None;
        }

        self.entries
            .partition_point(|entry| entry.time_ms() <= time_ms)
            .checked_sub(1)
            .and_then(|index| self.entries.get(index))
    }

    /// The previous entry relative to an active uid.
    pub fn previous_entry_uid(&self, uid: usize) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.uid() == uid)
            .and_then(|index| index.checked_sub(1))
            .map(|index| self.entries[index].uid())
    }

    /// The next entry relative to an active uid.
    pub fn next_entry_uid(&self, uid: usize) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.uid() == uid)
            .and_then(|index| self.entries.get(index + 1))
            .map(|entry| entry.uid())
    }

    /// Non-empty timeline chunks with end times derived from the next entry.
    pub fn timeline_chunks(&self, duration_ms: TimeMs) -> Vec<TimelineChunk> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| TimelineChunk {
                entry_id: entry.id(),
                uid: entry.uid(),
                color_index: entry.color_index(),
                start_ms: entry.time_ms(),
                end_ms: self
                    .entries
                    .get(index + 1)
                    .map_or(duration_ms, LyricEntry::time_ms),
                text: entry.display_text().to_owned(),
                raw_text: entry.text().to_owned(),
                is_empty: entry.is_empty(),
            })
            .collect()
    }

    /// Regenerate the LRC source text from current metadata and entries.
    pub fn to_source_text(&self) -> String {
        if self.entries.iter().all(|entry| entry.is_empty()) {
            return String::new();
        }

        let mut text = String::new();
        for tag in &self.metadata {
            text.push_str(&format!("[{}:{}]\n", tag.key(), tag.value()));
        }
        for entry in &self.entries {
            let trimmed = entry.text().trim_start();
            if trimmed.is_empty() {
                text.push_str(&format!("[{}]\n", entry.time_ms().as_timestamp()));
            } else {
                text.push_str(&format!("[{}] {}\n", entry.time_ms().as_timestamp(), trimmed));
            }
        }
        text
    }
}

#[allow(dead_code)]
pub fn reconcile_identity(
    old_doc: Option<&LrcDocument>,
    new_doc: &mut LrcDocument,
    next_uid: &mut usize,
) {
    let mut old_entries = old_doc.map(|d| d.entries().to_vec()).unwrap_or_default();
    let new_entries = new_doc.entries_mut();

    // 1. First pass: exact matches (timestamp and text)
    for entry in new_entries.iter_mut() {
        if let Some(pos) = old_entries.iter().position(|e| e.time_ms() == entry.time_ms() && e.text() == entry.text()) {
            let matched = old_entries.remove(pos);
            entry.uid = matched.uid();
            entry.color_index = matched.color_index();
        }
    }

    // 2. Second pass: text matches (timestamp changed)
    for entry in new_entries.iter_mut() {
        if entry.uid == 0
            && let Some(pos) = old_entries.iter().position(|e| e.text() == entry.text()) {
                let matched = old_entries.remove(pos);
                entry.uid = matched.uid();
                entry.color_index = matched.color_index();
            }
    }

    // 3. Third pass: timestamp matches (text changed)
    for entry in new_entries.iter_mut() {
        if entry.uid == 0
            && let Some(pos) = old_entries.iter().position(|e| e.time_ms() == entry.time_ms()) {
                let matched = old_entries.remove(pos);
                entry.uid = matched.uid();
                entry.color_index = matched.color_index();
            }
    }

    // 4. Final pass: brand new entries
    for i in 0..new_entries.len() {
        if new_entries[i].uid == 0 {
            new_entries[i].uid = *next_uid;
            *next_uid += 1;
            
            let prev_color = if i > 0 { Some(new_entries[i-1].color_index()) } else { None };
            let next_color = if i + 1 < new_entries.len() && new_entries[i+1].uid != 0 {
                Some(new_entries[i+1].color_index())
            } else {
                None
            };
            
            let mut best_color = (prev_color.map(|c| c as usize).unwrap_or(5) + 1) % 6;
            if Some(best_color as u8) == next_color {
                best_color = (best_color + 1) % 6;
            }
            new_entries[i].color_index = best_color as u8;
        }
    }
}
