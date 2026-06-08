//! `#title`, `#artist`, `#genre` and related display metadata.
//!
//! This module also defines [`DifficultyLevel`], the domain type for
//! `#DIFFICULTY`.

use std::fmt;
use std::str::FromStr;

use crate::error::BmsTokenizeError;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDifficultyError(pub String);

impl fmt::Display for ParseDifficultyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid #DIFFICULTY value: {:?} (expected 1–5)", self.0)
    }
}

impl std::error::Error for ParseDifficultyError {}

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

impl From<ParseDifficultyError> for BmsTokenizeError {
    fn from(e: ParseDifficultyError) -> Self {
        BmsTokenizeError::InvalidDifficulty(e.0)
    }
}

/// Display and difficulty headers.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsHeaderDisplay<'a> {
    /// `#STAGEFILE`
    StageFile(&'a str),
    /// `#BANNER`
    Banner(&'a str),
    /// `#BACKBMP`
    BackBmp(&'a str),
    /// `#CHARFILE`
    CharFile(&'a str),
    /// `#PLAYLEVEL`
    PlayLevel(f64),
    /// `#DIFFICULTY`
    Difficulty(DifficultyLevel),
    /// `#PREVIEW` (beatoraja extension)
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
