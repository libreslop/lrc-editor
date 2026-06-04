use crate::domain::parser::{SourceLine, ParseError};

/// Milliseconds from the beginning of the audio.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Default, Hash)]
pub struct TimeMs(pub u32);

impl TimeMs {
    pub fn to_secs(self) -> f64 {
        self.0 as f64 / 1000.0
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }

    pub fn as_timestamp(self) -> String {
        let minutes = self.0 / 60_000;
        let seconds = (self.0 % 60_000) / 1_000;
        let centiseconds = (self.0 % 1_000) / 10;

        format!("{minutes:02}:{seconds:02}.{centiseconds:02}")
    }

    pub(crate) fn parse(tag: &str, line: SourceLine) -> Result<Option<Self>, ParseError> {
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
}

/// A physical pixel distance or coordinate on the screen.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Pixels(pub f64);

impl Pixels {
    pub fn as_f64(self) -> f64 {
        self.0
    }
}
