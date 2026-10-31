//! Display and difficulty headers: `#STAGEFILE`, `#BANNER`, `#BACKBMP`,
//! `#CHARFILE`, `#PLAYLEVEL`, `#DIFFICULTY`, and `#PREVIEW`.
//!
//! This module also defines the domain types used by [`BmsHeaderDisplay`]:
//! [`DifficultyLevel`] and [`PoorBgaMode`].

use std::str::FromStr;

use thiserror::Error;

use crate::BmsTokenAttr;
use crate::IntoTokensError;

/// Difficulty classification specified by `#DIFFICULTY` (values 1-5).
///
/// Used to sort and filter charts in the song selection screen. Common
/// mappings:
///
/// | Value | Typical label |
/// |-------|---------------|
/// | `1` | BEGINNER / EASY / LIGHT |
/// | `2` | NORMAL / STANDARD |
/// | `3` | HYPER / HARD |
/// | `4` | ANOTHER / EX |
/// | `5` | INSANE / BLACK ANOTHER |
///
/// Omitting `#DIFFICULTY` is allowed, but it means the chart cannot be filtered
/// by difficulty classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display, derive_more::Deref)]
#[display("{}", _0)]
pub struct DifficultyLevel(u8);

impl DifficultyLevel {
    /// Raw numeric value (1-5).
    #[must_use]
    pub const fn get(&self) -> u8 {
        self.0
    }
}

/// Error returned when the `#DIFFICULTY` value is outside the valid range
/// (1-5).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid #DIFFICULTY value: {0} (expected 1-5)")]
pub struct ParseDifficultyError(pub String);

impl IntoTokensError for ParseDifficultyError {
    fn into_error(self, context: &'static str, value: String) -> crate::BmsTokenizeError {
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
        let v: u8 = s.parse().map_err(|_e| ParseDifficultyError(s.to_owned()))?;
        if !(1..=5).contains(&v) {
            return Err(ParseDifficultyError(s.to_owned()));
        }
        Ok(Self(v))
    }
}

/// Poor BGA display mode specified by `#POORBGA`.
///
/// Controls how miss/poor images (channel `#xxx06`) are displayed:
///
/// | Value | Behavior |
/// |-------|-----------|
/// | `0` | **Default**: on a miss, the whole BGA switches to `#xxx06` briefly, then returns to the normal image sequence. |
/// | `1` | **Overlay**: the `#xxx06` image is drawn over the current BGA (similar to the beatmaniaIIDX miss character animation). |
/// | `2` | **Hidden**: miss images are never shown; the normal BGA continues undisturbed. |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum PoorBgaMode {
    /// `#POORBGA 0`: use the default BGA display behavior.
    #[bms_token("0")]
    Default,
    /// `#POORBGA 1`: overlay the poor BGA on top of the current BGA.
    #[bms_token("1")]
    Overlay,
    /// `#POORBGA 2`: hide the current BGA while displaying the poor BGA.
    #[bms_token("2")]
    Hidden,
}

/// Display and difficulty headers.
///
/// These commands control what the player *sees* outside the actual playable
/// notes: loading screens, banners, difficulty labels, and so on.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderDisplay {
    /// `#STAGEFILE`: splash image shown during loading (usually 640x480).
    ///
    /// Optional. When omitted, the player shows its default loading screen.
    #[bms_token("#STAGEFILE {}")]
    StageFile(String),
    /// `#BANNER`: banner image for the song selection and result screens
    /// (300x80).
    ///
    /// Optional. Supports relative paths (descending only). Path length is
    /// limited to 260 bytes.
    #[bms_token("#BANNER {}")]
    Banner(String),
    /// `#BACKBMP`: background image for the gameplay screen (usually 640x480).
    ///
    /// Original specification: the image fills the gameplay area background. In
    /// some LR2 skins it is used as a title card. Size and behavior depend on
    /// the skin.
    #[bms_token("#BACKBMP {}")]
    BackBmp(String),
    /// `#CHARFILE`: pop'n music style character file (pomu2 extension).
    ///
    /// A `.chp` file defining an animated character shown during gameplay. Only
    /// pomu2 and PMChr-V support it.
    #[bms_token("#CHARFILE {}")]
    CharFile(String),
    /// `#PLAYLEVEL`: difficulty value shown in the song list.
    ///
    /// The display format varies by player (stars, bar graphs, integers). It is
    /// usually an integer, but some players accept non-numeric strings. Default
    /// when omitted: `3` (BM98 convention).
    ///
    /// The value `0` has a special meaning in BM98 and several other players:
    /// it displays as a question mark (`?`) instead of a number, commonly used
    /// for charts whose difficulty changes with `#RANDOM` / `#SWITCH`.
    #[bms_token("#PLAYLEVEL {}")]
    PlayLevel(f64),
    /// `#DIFFICULTY`: difficulty *classification* used for chart filtering
    /// (1-5).
    #[bms_token("#DIFFICULTY {}")]
    Difficulty(DifficultyLevel),
    /// `#PREVIEW`: audio file played in the song selection screen (beatoraja
    /// extension).
    ///
    /// When omitted, beatoraja automatically discovers `preview*.wav` /
    /// `preview*.ogg` in the chart folder.
    #[bms_token("#PREVIEW {}")]
    Preview(String),
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
