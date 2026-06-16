//! Display and difficulty headers: `#STAGEFILE`, `#BANNER`, `#BACKBMP`,
//! `#CHARFILE`, `#PLAYLEVEL`, `#DIFFICULTY`, `#PREVIEW`.
//!
//! This module also defines the domain types used by [`BmsHeaderDisplay`]:
//! [`DifficultyLevel`] and [`PoorBgaMode`].

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

use crate::BmsStr;
use crate::BmsTokenAttr;
use crate::IntoTokensError;
use crate::{BmsHeader, BmsTryFromError};

/// The difficulty category specified by `#DIFFICULTY` (values 1–5).
///
/// Used to sort and filter charts in song-selection screens.  Common
/// mapping:
///
/// | Value | Typical label |
/// |-------|---------------|
/// | `1` | BEGINNER / EASY / LIGHT |
/// | `2` | NORMAL / STANDARD |
/// | `3` | HYPER / HARD |
/// | `4` | ANOTHER / EX |
/// | `5` | INSANE / BLACK ANOTHER |
///
/// Omitting `#DIFFICULTY` is allowed but means the chart cannot be
/// filtered by difficulty category.
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
///
/// Controls how the miss / poor image (channel `#xxx06`) is shown:
///
/// | Value | Behaviour |
/// |-------|-----------|
/// | `0` | **Default** — on miss, the entire BGA switches to `#xxx06` for a brief moment, then returns to the normal image sequence. |
/// | `1` | **Overlay** — the `#xxx06` image is composited on top of the current BGA (like beatmaniaIIDX's miss character animation). |
/// | `2` | **Hidden** — miss images are never shown; the normal BGA continues uninterrupted. |
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
///
/// These commands control *what the player sees* outside of actual
/// gameplay notes — loading screens, banners, difficulty labels, etc.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderDisplay<'a, C = &'a str> {
    /// `#STAGEFILE` — splash-screen image shown during loading (typically 640×480).
    ///
    /// Optional.  When omitted, players show their default loading screen.
    #[bms_token("#STAGEFILE {}")]
    StageFile(C),
    /// `#BANNER` — banner image for song-selection and result screens (300×80).
    ///
    /// Optional.  Supports relative paths (descendant only).  Path length
    /// is limited to 260 bytes.
    #[bms_token("#BANNER {}")]
    Banner(C),
    /// `#BACKBMP` — background image for the play screen (typically 640×480).
    ///
    /// Original spec: the image fills the play-area background.  In some
    /// LR2 skins, it is repurposed as a title card.  Size and behaviour
    /// are skin-dependent.
    #[bms_token("#BACKBMP {}")]
    BackBmp(C),
    /// `#CHARFILE` — pop'n music-style character file (pomu2 extension).
    ///
    /// A `.chp` file that defines an animated character shown during play.
    /// Only supported by pomu2 and PMChr-V.
    #[bms_token("#CHARFILE {}")]
    CharFile(C),
    /// `#PLAYLEVEL` — difficulty number shown in the song-selection list.
    ///
    /// Display format varies by player (stars, bar graph, integer).
    /// Usually an integer but some players accept strings (e.g.
    /// `#PLAYLEVEL 安心`).  Default when omitted: `3` (BM98 convention).
    ///
    /// Value `0` has special meaning in BM98 and several other players:
    /// it displays as a question mark (`?`) instead of a numeric value,
    /// often used for charts whose difficulty varies via `#RANDOM`/`#SWITCH`.
    #[bms_token("#PLAYLEVEL {}")]
    PlayLevel(f64),
    /// `#DIFFICULTY` — difficulty *category* (1–5) for chart filtering.
    #[bms_token("#DIFFICULTY {}")]
    Difficulty(DifficultyLevel),
    /// `#PREVIEW` — audio file played on the song-selection screen
    /// (beatoraja extension).
    ///
    /// When omitted, beatoraja auto-discovers `preview*.wav` /
    /// `preview*.ogg` in the chart folder.
    #[bms_token("#PREVIEW {}")]
    Preview(C),
    /// Phantom data to satisfy E0392 (unused lifetime parameter).
    #[doc(hidden)]
    _Phantom(std::marker::PhantomData<&'a C>),
}

// From / TryFrom conversions

impl<'a, C: BmsStr<'a>> From<BmsHeaderDisplay<'a, C>> for BmsHeader<'a, C> {
    #[inline]
    fn from(display: BmsHeaderDisplay<'a, C>) -> Self {
        BmsHeader::Display(display)
    }
}

impl<'a, C: BmsStr<'a>> TryFrom<BmsHeader<'a, C>> for BmsHeaderDisplay<'a, C> {
    type Error = BmsTryFromError<'a>;

    #[inline]
    fn try_from(header: BmsHeader<'a, C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Display(d) => Ok(d),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
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
