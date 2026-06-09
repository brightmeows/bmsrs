//! `#difficulty`, `#playlevel`, `#poorbga` and related display settings.
//!
//! This module also defines the domain types used by [`BmsHeaderDisplay`]:
//! [`DifficultyLevel`] and [`PoorBgaMode`].

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

use crate::BmsTokenAttr;
use crate::IntoTokensError;

/// The difficulty category specified by `#DIFFICULTY` (values 1–5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifficultyLevel(u8);

impl DifficultyLevel {
    /// The raw numeric value (1–5).
    #[must_use]
    pub fn get(&self) -> u8 {
        self.0
    }
}

/// Error returned when a `#DIFFICULTY` value is not in the valid range (1–5).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid #DIFFICULTY value: {0} (expected 1-5)")]
pub struct ParseDifficultyError(pub String);

impl<'a> IntoTokensError<'a> for ParseDifficultyError {
    fn into_error(self, context: &'static str, value: &'a str) -> crate::BmsTokenizeError<'a> {
        crate::BmsTokenizeError::OutOfRange {
            context,
            value,
            expected: "1-5",
        }
    }
}

impl FromStr for DifficultyLevel {
    type Err = ParseDifficultyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: u8 = s.parse().map_err(|_| ParseDifficultyError(s.to_owned()))?;
        if !(1..=5).contains(&v) {
            return Err(ParseDifficultyError(s.to_owned()));
        }
        Ok(DifficultyLevel(v))
    }
}

impl fmt::Display for DifficultyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Poor BGA display mode specified by `#POORBGA`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum PoorBgaMode {
    /// `#POORBGA 0` — use default BGA display behaviour.
    #[bms_token("0")]
    Default,
    /// `#POORBGA 1` — overlay the poor BGA on top of the current BGA.
    #[bms_token("1")]
    Overlay,
    /// `#POORBGA 2` — hide the current BGA when displaying the poor BGA.
    #[bms_token("2")]
    Hidden,
}

/// Display and difficulty headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderDisplay<'a> {
    /// `#STAGEFILE`
    #[bms_token("#STAGEFILE {value}")]
    StageFile(&'a str),
    /// `#BANNER`
    #[bms_token("#BANNER {value}")]
    Banner(&'a str),
    /// `#BACKBMP`
    #[bms_token("#BACKBMP {value}")]
    BackBmp(&'a str),
    /// `#CHARFILE`
    #[bms_token("#CHARFILE {value}")]
    CharFile(&'a str),
    /// `#PLAYLEVEL`
    #[bms_token("#PLAYLEVEL {value}")]
    PlayLevel(f64),
    /// `#DIFFICULTY`
    #[bms_token("#DIFFICULTY {value}")]
    Difficulty(DifficultyLevel),
    /// `#PREVIEW` (beatoraja extension)
    #[bms_token("#PREVIEW {value}")]
    Preview(&'a str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_valid_range() {
        for i in 1..=5u8 {
            let d: DifficultyLevel = i.to_string().parse().unwrap();
            assert_eq!(d.get(), i);
        }
    }

    #[test]
    fn difficulty_out_of_range() {
        assert!("0".parse::<DifficultyLevel>().is_err());
        assert!("6".parse::<DifficultyLevel>().is_err());
        assert!("100".parse::<DifficultyLevel>().is_err());
    }

    #[test]
    fn difficulty_non_numeric() {
        assert!("abc".parse::<DifficultyLevel>().is_err());
        assert!("".parse::<DifficultyLevel>().is_err());
    }
}
