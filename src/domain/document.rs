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
    pub(crate) fn new(entries: Vec<LyricEntry>, metadata: Vec<MetadataTag>, line_count: usize) -> Self {
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

    /// Find an entry by stable id.
    pub fn entry_by_id(&self, id: usize) -> Option<&LyricEntry> {
        self.entries.iter().find(|entry| entry.id() == id)
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

    /// The previous entry relative to an active id.
    pub fn previous_entry_id(&self, id: usize) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.id() == id)
            .and_then(|index| index.checked_sub(1))
            .map(|index| self.entries[index].id())
    }

    /// The next entry relative to an active id.
    pub fn next_entry_id(&self, id: usize) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.id() == id)
            .and_then(|index| self.entries.get(index + 1))
            .map(|entry| entry.id())
    }

    /// Non-empty timeline chunks with end times derived from the next entry.
    pub fn timeline_chunks(&self, duration_ms: TimeMs) -> Vec<TimelineChunk> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| TimelineChunk {
                entry_id: entry.id(),
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
        let mut text = String::new();
        for tag in &self.metadata {
            text.push_str(&format!("[{}:{}]\n", tag.key(), tag.value()));
        }
        for entry in &self.entries {
            text.push_str(&format!("[{}]{}\n", entry.time_ms().as_timestamp(), entry.text()));
        }
        text
    }
}
