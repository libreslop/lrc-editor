use crate::domain::document::LrcDocument;
use crate::domain::metadata::MetadataTag;
use crate::domain::entry::LyricEntry;
use crate::domain::time::TimeMs;

/// Zero-based source line number.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SourceLine(pub(crate) usize);

impl SourceLine {
    pub(crate) fn as_display_number(self) -> usize {
        self.0 + 1
    }

    pub(crate) fn as_zero_based(self) -> usize {
        self.0
    }
}

/// Human-readable parse failure with source location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    line: SourceLine,
    message: String,
}

impl ParseError {
    pub(crate) fn new(line: SourceLine, message: impl Into<String>) -> Self {
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

        Ok(LrcDocument::new(entries, metadata, line_count))
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
            match TimeMs::parse(tag, source_line)? {
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
