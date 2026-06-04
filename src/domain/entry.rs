use crate::domain::time::TimeMs;
use crate::domain::parser::SourceLine;

/// A timed lyric line after expanding repeated timestamps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LyricEntry {
    pub(crate) id: usize,
    pub(crate) uid: usize,
    pub(crate) color_index: u8,
    pub(crate) time: TimeMs,
    pub(crate) timestamp: String,
    pub(crate) text: String,
    pub(crate) display_text: String,
    pub(crate) source_line: SourceLine,
    pub(crate) source_order: usize,
    pub(crate) lyric_start_utf16: usize,
    pub(crate) lyric_end_utf16: usize,
}

impl LyricEntry {
    /// Stable id in timestamp-sorted order.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Stable unique id across re-parses.
    pub fn uid(&self) -> usize {
        self.uid
    }

    /// Color index (0-3) for visual distinction.
    pub fn color_index(&self) -> u8 {
        self.color_index
    }

    /// Timestamp in milliseconds.
    pub fn time_ms(&self) -> TimeMs {
        self.time
    }

    /// LRC timestamp formatted as `mm:ss.cc`.
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// Raw lyric text after the timestamp prefix.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Trimmed text used for previews and timeline chunks.
    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    /// Whether this entry should be omitted from the chunk lane.
    pub fn is_empty(&self) -> bool {
        self.display_text.is_empty()
    }

    /// UTF-16 selection start for the lyric portion in the source textarea.
    pub fn lyric_start_utf16(&self) -> usize {
        self.lyric_start_utf16
    }

    /// UTF-16 selection end for the lyric portion in the source textarea.
    pub fn lyric_end_utf16(&self) -> usize {
        self.lyric_end_utf16
    }
}

/// A lyric chunk ready for timeline rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineChunk {
    pub(crate) entry_id: usize,
    pub(crate) uid: usize,
    pub(crate) color_index: u8,
    pub(crate) start_ms: TimeMs,
    pub(crate) end_ms: TimeMs,
    pub(crate) text: String,
    pub(crate) raw_text: String,
    pub(crate) is_empty: bool,
}

impl TimelineChunk {
    /// Entry id represented by this chunk.
    pub fn entry_id(&self) -> usize {
        self.entry_id
    }

    /// Stable unique id.
    pub fn uid(&self) -> usize {
        self.uid
    }

    /// Color index.
    pub fn color_index(&self) -> u8 {
        self.color_index
    }

    /// Chunk start timestamp.
    pub fn start_ms(&self) -> TimeMs {
        self.start_ms
    }

    /// Chunk end timestamp.
    pub fn end_ms(&self) -> TimeMs {
        self.end_ms
    }

    /// Visible text on the chunk.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The original untrimmed text.
    pub fn raw_text(&self) -> &str {
        &self.raw_text
    }

    /// Whether this chunk represents an empty gap.
    pub fn is_empty(&self) -> bool {
        self.is_empty
    }
}
