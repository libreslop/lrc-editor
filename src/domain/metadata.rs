use crate::domain::parser::SourceLine;

/// Untimed LRC metadata such as `[ar:Artist]` or `[ti:Title]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataTag {
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) source_line: SourceLine,
}

impl MetadataTag {
    pub fn key(&self) -> &str {
        &self.key
    }
    
    pub fn value(&self) -> &str {
        &self.value
    }

    pub(crate) fn from_line(line: &str, source_line: SourceLine) -> Option<Self> {
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
