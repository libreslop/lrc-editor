/// Milliseconds from the beginning of the audio.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Milliseconds(u32);

impl Milliseconds {
    fn parse(tag: &str, line: SourceLine) -> Result<Option<Self>, ParseError> {
        let Some((minutes, seconds_and_fraction)) = tag.split_once(':') else {
            return Ok(None);
        };

        if minutes.is_empty() {
            return Err(ParseError::new(line, "timestamp minutes are missing"));
        }

        if !minutes.chars().all(|character| character.is_ascii_digit()) {
            return Ok(None);
        }

        let Some(seconds_head) = seconds_and_fraction.get(0..2) else {
            return Err(ParseError::new(
                line,
                "timestamp seconds must use two digits",
            ));
        };

        if !seconds_head
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            return Err(ParseError::new(
                line,
                "timestamp seconds must use two digits",
            ));
        }

        let fraction_ms = match &seconds_and_fraction[2..] {
            "" => 0,
            suffix => {
                let Some(fraction) = suffix.strip_prefix('.') else {
                    return Err(ParseError::new(
                        line,
                        "timestamp fraction must start with a period",
                    ));
                };

                Self::parse_fraction(fraction, line)?
            }
        };

        let minutes = minutes
            .parse::<u32>()
            .map_err(|_| ParseError::new(line, "timestamp minutes are too large"))?;
        let seconds = seconds_head
            .parse::<u32>()
            .map_err(|_| ParseError::new(line, "timestamp seconds are invalid"))?;

        if seconds >= 60 {
            return Err(ParseError::new(line, "timestamp seconds must be below 60"));
        }

        let total = minutes
            .checked_mul(60_000)
            .and_then(|value| value.checked_add(seconds * 1_000))
            .and_then(|value| value.checked_add(fraction_ms))
            .ok_or_else(|| ParseError::new(line, "timestamp is too large"))?;

        Ok(Some(Self(total)))
    }

    fn parse_fraction(fraction: &str, line: SourceLine) -> Result<u32, ParseError> {
        if fraction.is_empty() || fraction.len() > 3 {
            return Err(ParseError::new(
                line,
                "timestamp fraction must use one to three digits",
            ));
        }

        if !fraction.chars().all(|character| character.is_ascii_digit()) {
            return Err(ParseError::new(
                line,
                "timestamp fraction must only contain digits",
            ));
        }

        let value = fraction
            .parse::<u32>()
            .map_err(|_| ParseError::new(line, "timestamp fraction is invalid"))?;

        Ok(match fraction.len() {
            1 => value * 100,
            2 => value * 10,
            3 => value,
            _ => unreachable!("fraction length is checked above"),
        })
    }

    fn as_u32(self) -> u32 {
        self.0
    }

    fn as_timestamp(self) -> String {
        let minutes = self.0 / 60_000;
        let seconds = (self.0 % 60_000) / 1_000;
        let centiseconds = (self.0 % 1_000) / 10;

        format!("{minutes:02}:{seconds:02}.{centiseconds:02}")
    }
}

/// Zero-based source line number.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SourceLine(usize);

impl SourceLine {
    fn as_display_number(self) -> usize {
        self.0 + 1
    }

    fn as_zero_based(self) -> usize {
        self.0
    }
}

/// Untimed LRC metadata such as `[ar:Artist]` or `[ti:Title]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataTag {
    key: String,
    value: String,
    source_line: SourceLine,
}

impl MetadataTag {
    pub fn key(&self) -> &str {
        &self.key
    }
    
    pub fn value(&self) -> &str {
        &self.value
    }

    fn from_line(line: &str, source_line: SourceLine) -> Option<Self> {
        let close_index = line.find(']')?;

        if close_index != line.len() - 1 {
            return None;
        }

        let inner = &line[1..close_index];
        let (key, value) = inner.split_once(':')?;

        if key.is_empty() || key.chars().all(|character| character.is_ascii_digit()) {
            return None;
        }

        Some(Self {
            key: key.to_owned(),
            value: value.to_owned(),
            source_line,
        })
    }
}

/// A timed lyric line after expanding repeated timestamps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LyricEntry {
    id: usize,
    time: Milliseconds,
    timestamp: String,
    text: String,
    display_text: String,
    source_line: SourceLine,
    source_order: usize,
    lyric_start_utf16: usize,
    lyric_end_utf16: usize,
}

impl LyricEntry {
    /// Stable id in timestamp-sorted order.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Timestamp in milliseconds.
    pub fn time_ms(&self) -> u32 {
        self.time.as_u32()
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
    entry_id: usize,
    start_ms: u32,
    end_ms: u32,
    text: String,
    raw_text: String,
    is_empty: bool,
}

impl TimelineChunk {
    /// Entry id represented by this chunk.
    pub fn entry_id(&self) -> usize {
        self.entry_id
    }

    /// Chunk start timestamp.
    pub fn start_ms(&self) -> u32 {
        self.start_ms
    }

    /// Chunk end timestamp.
    pub fn end_ms(&self) -> u32 {
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

/// Parsed LRC document state used by the UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LrcDocument {
    entries: Vec<LyricEntry>,
    metadata: Vec<MetadataTag>,
    line_count: usize,
}

impl LrcDocument {
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
    pub fn last_entry_time_ms(&self) -> Option<u32> {
        self.entries.last().map(LyricEntry::time_ms)
    }

    /// Find an entry by stable id.
    pub fn entry_by_id(&self, id: usize) -> Option<&LyricEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// Entry active at `time_ms`, using standard LRC "latest timestamp wins" behavior.
    pub fn current_entry(&self, time_ms: u32) -> Option<&LyricEntry> {
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
            .position(|entry| entry.id == id)
            .and_then(|index| index.checked_sub(1))
            .map(|index| self.entries[index].id)
    }

    /// The next entry relative to an active id.
    pub fn next_entry_id(&self, id: usize) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.id == id)
            .and_then(|index| self.entries.get(index + 1))
            .map(|entry| entry.id)
    }

    /// Non-empty timeline chunks with end times derived from the next entry.
    pub fn timeline_chunks(&self, duration_ms: u32) -> Vec<TimelineChunk> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| TimelineChunk {
                entry_id: entry.id,
                start_ms: entry.time_ms(),
                end_ms: self
                    .entries
                    .get(index + 1)
                    .map_or(duration_ms, LyricEntry::time_ms),
                text: entry.display_text.clone(),
                raw_text: entry.text().to_owned(),
                is_empty: entry.is_empty(),
            })
            .collect()
    }

    /// Regenerate the LRC source text from current metadata and entries.
    pub fn to_source_text(&self) -> String {
        let mut text = String::new();
        for tag in &self.metadata {
            text.push_str(&format!("[{}:{}]\n", tag.key, tag.value));
        }
        for entry in &self.entries {
            let mins = entry.time_ms() / 60000;
            let secs = (entry.time_ms() % 60000) / 1000;
            let hund = (entry.time_ms() % 1000) / 10;
            text.push_str(&format!("[{:02}:{:02}.{:02}]{}\n", mins, secs, hund, entry.text()));
        }
        text
    }
}

/// Stateful parser for a single LRC source string.
pub struct LrcParser<'source> {
    source: &'source str,
}

impl<'source> LrcParser<'source> {
    /// Create a parser over source text.
    pub fn new(source: &'source str) -> Self {
        Self { source }
    }

    /// Parse LRC source into a structured document.
    pub fn parse(&self) -> Result<LrcDocument, ParseError> {
        let mut entries = Vec::new();
        let mut metadata = Vec::new();
        let mut line_count = 0;
        let mut line_start_utf16 = 0;

        for (line_index, raw_line) in self.source.lines().enumerate() {
            line_count = line_index + 1;
            let source_line = SourceLine(line_index);

            if !raw_line.trim().is_empty() {
                self.parse_line(
                    raw_line,
                    source_line,
                    line_start_utf16,
                    &mut entries,
                    &mut metadata,
                )?;
            }

            line_start_utf16 += raw_line.encode_utf16().count() + 1;
        }

        if self.source.ends_with('\n') {
            line_count += 1;
        }

        entries.sort_by_key(|entry| (entry.time, entry.source_line, entry.source_order));

        for (entry_id, entry) in entries.iter_mut().enumerate() {
            entry.id = entry_id;
        }

        Ok(LrcDocument {
            entries,
            metadata,
            line_count,
        })
    }

    fn parse_line(
        &self,
        raw_line: &str,
        source_line: SourceLine,
        line_start_utf16: usize,
        entries: &mut Vec<LyricEntry>,
        metadata: &mut Vec<MetadataTag>,
    ) -> Result<(), ParseError> {
        if !raw_line.starts_with('[') {
            return Err(ParseError::new(
                source_line,
                "non-empty LRC lines must start with a timestamp or metadata tag",
            ));
        }

        let mut rest = raw_line;
        let mut prefix_utf16 = 0;
        let mut times = Vec::new();

        while let Some((tag, full_tag, after_tag)) = Self::take_bracket_tag(rest, source_line)? {
            match Milliseconds::parse(tag, source_line)? {
                Some(time) => {
                    times.push(time);
                    prefix_utf16 += full_tag.encode_utf16().count();
                    rest = after_tag;
                }
                None => break,
            }
        }

        if times.is_empty() {
            if let Some(tag) = MetadataTag::from_line(raw_line, source_line) {
                metadata.push(tag);
                return Ok(());
            }

            return Err(ParseError::new(
                source_line,
                "expected a timestamp like [00:12.34] or metadata like [ti:Song]",
            ));
        }

        let text = rest.to_owned();
        let display_text = text.trim().to_owned();
        let lyric_start_utf16 = line_start_utf16 + prefix_utf16;
        let lyric_end_utf16 = lyric_start_utf16 + text.encode_utf16().count();
        let next_order = entries.len();

        entries.extend(
            times
                .into_iter()
                .enumerate()
                .map(|(offset, time)| LyricEntry {
                    id: 0,
                    timestamp: time.as_timestamp(),
                    time,
                    text: text.clone(),
                    display_text: display_text.clone(),
                    source_line,
                    source_order: next_order + offset,
                    lyric_start_utf16,
                    lyric_end_utf16,
                }),
        );

        Ok(())
    }

    fn take_bracket_tag(
        line: &str,
        source_line: SourceLine,
    ) -> Result<Option<(&str, &str, &str)>, ParseError> {
        if !line.starts_with('[') {
            return Ok(None);
        }

        let close_index = line
            .find(']')
            .ok_or_else(|| ParseError::new(source_line, "missing closing bracket"))?;

        Ok(Some((
            &line[1..close_index],
            &line[..=close_index],
            &line[close_index + 1..],
        )))
    }
}

/// Human-readable parse failure with source location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    line: SourceLine,
    message: String,
}

impl ParseError {
    fn new(line: SourceLine, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }

    /// User-facing error message.
    pub fn prefixed_message(&self) -> String {
        format!("Line {}: {}", self.line.as_display_number(), self.message)
    }

    /// Zero-based source line index.
    pub fn line_index(&self) -> usize {
        self.line.as_zero_based()
    }
}

/// How a chunk click should update selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionMode {
    /// Replace selection with the clicked chunk.
    Replace,
    /// Toggle the clicked chunk.
    Toggle,
    /// Expand from the anchor to the clicked chunk.
    Range,
}

/// Selected lyric chunk ids and range anchor.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionState {
    selected_ids: Vec<usize>,
    anchor_id: Option<usize>,
}

impl SelectionState {
    /// Current selected ids.
    pub fn selected_ids(&self) -> &[usize] {
        &self.selected_ids
    }

    /// Anchor used for range selection.
    pub fn anchor_id(&self) -> Option<usize> {
        self.anchor_id
    }

    /// Whether an id is currently selected.
    pub fn contains(&self, id: usize) -> bool {
        self.selected_ids.contains(&id)
    }

    /// Keep only selections that still exist in the document.
    pub fn prune(&mut self, document: &LrcDocument) {
        self.selected_ids
            .retain(|id| document.entry_by_id(*id).is_some());

        if self
            .anchor_id
            .is_some_and(|id| document.entry_by_id(id).is_none())
        {
            self.anchor_id = None;
        }
    }

    /// Replace with the active lyric unless multiple chunks are selected.
    pub fn sync_to_active(&mut self, entry: Option<&LyricEntry>, preserve_selection: bool) {
        if preserve_selection {
            return;
        }

        self.selected_ids.clear();

        if let Some(entry) = entry.filter(|entry| !entry.is_empty()) {
            self.selected_ids.push(entry.id());
            self.anchor_id = Some(entry.id());
        }
    }

    /// Select every non-empty entry.
    pub fn select_all(&mut self, document: &LrcDocument) {
        self.selected_ids = document
            .entries()
            .iter()
            .filter(|entry| !entry.is_empty())
            .map(LyricEntry::id)
            .collect();
        self.anchor_id = self.selected_ids.first().copied();
    }

    /// Apply a click selection transition.
    pub fn select_entry(&mut self, document: &LrcDocument, entry_id: usize, mode: SelectionMode) {
        let Some(entry) = document.entry_by_id(entry_id) else {
            return;
        };

        match mode {
            SelectionMode::Replace => {
                self.selected_ids.clear();
                if !entry.is_empty() {
                    self.selected_ids.push(entry_id);
                }
                self.anchor_id = Some(entry_id);
            }
            SelectionMode::Toggle => {
                if let Some(index) = self.selected_ids.iter().position(|id| *id == entry_id) {
                    self.selected_ids.remove(index);
                } else if !entry.is_empty() {
                    self.selected_ids.push(entry_id);
                    self.selected_ids.sort_unstable();
                }
                self.anchor_id = Some(entry_id);
            }
            SelectionMode::Range => {
                let anchor_id = self.anchor_id.unwrap_or(entry_id);
                let start = anchor_id.min(entry_id);
                let end = anchor_id.max(entry_id);
                self.selected_ids = document
                    .entries()
                    .iter()
                    .filter(|entry| {
                        !entry.is_empty() && (start..=end).contains(&entry.id())
                    })
                    .map(LyricEntry::id)
                    .collect();
            }
        }
    }

    /// True when source text selection should be suppressed.
    pub fn suppresses_source_selection(&self) -> bool {
        self.selected_ids.len() > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> LrcDocument {
        LrcParser::new(source).parse().expect("source should parse")
    }

    #[test]
    fn parses_timed_lines() {
        let document = parse_ok("[00:09.24] Some people watch\n[01:02.05] Next line");

        assert_eq!(document.entries.len(), 2);
        assert_eq!(document.entries[0].time, Milliseconds(9_240));
        assert_eq!(document.entries[0].text, " Some people watch");
        assert_eq!(document.entries[1].time, Milliseconds(62_050));
    }

    #[test]
    fn preserves_empty_timed_lines() {
        let document = parse_ok("[02:03.02] \n[02:08.54] Just look");

        assert_eq!(document.entries.len(), 2);
        assert_eq!(document.entries[0].text, " ");
        assert_eq!(document.entries[0].display_text, "");
        assert_eq!(document.entries[1].text, " Just look");
    }

    #[test]
    fn expands_multiple_timestamps_on_one_line() {
        let document = parse_ok("[00:01.00][00:02.50] Echo");

        assert_eq!(document.entries.len(), 2);
        assert_eq!(document.entries[0].time, Milliseconds(1_000));
        assert_eq!(document.entries[1].time, Milliseconds(2_500));
        assert_eq!(document.entries[0].text, " Echo");
        assert_eq!(document.entries[1].text, " Echo");
    }

    #[test]
    fn sorts_entries_by_timestamp() {
        let document = parse_ok("[00:10.00] Later\n[00:05.00] Earlier");

        assert_eq!(document.entries[0].text, " Earlier");
        assert_eq!(document.entries[0].id, 0);
        assert_eq!(document.entries[1].text, " Later");
        assert_eq!(document.entries[1].id, 1);
    }

    #[test]
    fn parses_metadata() {
        let document = parse_ok("[ti:Fake Your Death]\n[ar:My Chemical Romance]\n[00:09.24] Lyric");

        assert_eq!(document.metadata.len(), 2);
        assert_eq!(document.metadata[0].key, "ti");
        assert_eq!(document.metadata[0].value, "Fake Your Death");
        assert_eq!(document.entries.len(), 1);
    }

    #[test]
    fn computes_lyric_selection_offsets_in_utf16() {
        let document = parse_ok("[00:01.00] Hi 😄\n[00:02.00] Next");
        let first = &document.entries[0];
        let second = &document.entries[1];

        assert_eq!(first.lyric_start_utf16(), 10);
        assert_eq!(first.lyric_end_utf16(), 16);
        assert_eq!(second.lyric_start_utf16(), 27);
    }

    #[test]
    fn finds_current_entry_by_time() {
        let document = parse_ok("[00:01.00] One\n[00:04.00] Two");

        assert_eq!(document.current_entry(999), None);
        assert_eq!(document.current_entry(1_000).map(LyricEntry::id), Some(0));
        assert_eq!(document.current_entry(3_999).map(LyricEntry::id), Some(0));
        assert_eq!(document.current_entry(4_000).map(LyricEntry::id), Some(1));
    }

    #[test]
    fn includes_empty_lines_in_timeline_chunks() {
        let document = parse_ok("[00:01.00] One\n[00:02.00] \n[00:03.00] Three");
        let chunks = document.timeline_chunks(5_000);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].entry_id(), 0);
        assert_eq!(chunks[0].end_ms(), 2_000);
        assert_eq!(chunks[1].entry_id(), 1);
        assert!(chunks[1].is_empty());
        assert_eq!(chunks[1].end_ms(), 3_000);
        assert_eq!(chunks[2].entry_id(), 2);
    }

    #[test]
    fn selection_range_skips_empty_entries() {
        let document = parse_ok("[00:01.00] One\n[00:02.00] \n[00:03.00] Three");
        let mut selection = SelectionState::default();

        selection.select_entry(&document, 0, SelectionMode::Replace);
        selection.select_entry(&document, 2, SelectionMode::Range);

        assert_eq!(selection.selected_ids(), &[0, 2]);
    }

    #[test]
    fn rejects_plain_text_lines() {
        let error = LrcParser::new("plain lyric")
            .parse()
            .expect_err("plain text should be invalid");

        assert_eq!(error.line_index(), 0);
        assert!(error.prefixed_message().contains("must start"));
    }

    #[test]
    fn rejects_seconds_above_lrc_range() {
        let error = LrcParser::new("[00:60.00] Bad")
            .parse()
            .expect_err("seconds above 59 should be invalid");

        assert!(error.prefixed_message().contains("below 60"));
    }

    #[test]
    fn rejects_long_fraction() {
        let error = LrcParser::new("[00:01.1234] Bad")
            .parse()
            .expect_err("long fractions should be invalid");

        assert!(error.prefixed_message().contains("one to three"));
    }
}
