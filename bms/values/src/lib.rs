//! Single-line syntax for the BMS (Be-Music Script) format.
//!
//! This crate provides the lowest layer of the BMS parsing pipeline: it parses
//! **one line** of text into structured tokens. Whole-file line splitting, the
//! comment state machine, encoding, and line-level editing belong to
//! `bmsrs-bms-source` / `bmsrs-bms-syntax`.
//!
//! # Structure
//!
//! - [`enum@BmsToken`] represents one meaningful line in a BMS file.
//! - [`BmsHeader`] covers all header commands, grouped by semantic domain.
//! - [`BmsMessage`] covers channel data lines (`#xxxYY:values`).
//! - [`parse_header_line`] / [`parse_message_line`] are the single-line entry
//!   points.
//!
//! The submodules are private; all public types are re-exported from the crate
//! root.
//!
//! Typed values (numeric conversions and parsed enums) are always owned.

use std::fmt;
use std::num::NonZeroUsize;

mod body;
mod channel;
mod error;
mod header;
mod index;
mod message;

pub use bmsrs_bms_values_derive::{BmsIndexNewtype, BmsTokenAttr};
pub use body::{BodyCells, Cell};
pub use channel::BmsChannel;
pub use error::{BmsTokenizeError, BmsTryFromError, IntoTokensError, ParseBmsValueError};
pub use header::parse_header_line;
pub use header::{
    ArgbParams, AtBgaParams, BgaParams, DifficultyLevel, ExBmpParams, ExWavParams, LnMode, LnType,
    ParseDifficultyError, PlayerMode, PoorBgaMode, Rank, StpParams, SwBgaParams, WavCmdKind,
    WavCmdParams,
};
pub use header::{
    BmsHeader, BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderFallback, BmsHeaderGameplay,
    BmsHeaderMetadata, BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming,
};
pub use index::{
    BmpIndex, BmsBase, BmsIndex, BmsIndexError, BpmIndex, ChangeOptionIndex, ChannelIndex,
    ExRankIndex, LnObjIndex, ObjectIndex, ScrollIndex, SeekIndex, SpeedIndex, StopIndex, TextIndex,
    WavIndex, base36_digit_value,
};
pub use message::BmsMessage;
pub use message::parse_message_line;

/// Unified trait for BMS header values.
///
/// Combines parsing (from an input string) and formatting (back to a BMS value
/// string) into a single contract. Types implementing [`std::str::FromStr`] and
/// [`std::fmt::Display`] get a blanket implementation, so simple numeric or
/// identifier types need no manual implementation.
///
/// # Formatting
///
/// This trait uses [`std::fmt::Display`] as a supertrait instead of providing
/// its own formatting method. Callers use `.to_string()` to get the BMS
/// representation, which keeps the trait compatible with the standard library's
/// formatting infrastructure.
pub trait BmsValue: fmt::Display + Sized {
    /// Parse `s` into `Self`.
    ///
    /// Returns `None` on parse failure, letting the caller fall back to
    /// `BmsHeaderFallback` instead of treating the failure as a hard error.
    #[must_use]
    fn parse(s: &str) -> Option<Self>;
}

// Covers primitive types (f64, u8, i32), BmsIndex, PoorBgaMode,
// DifficultyLevel, and any type that implements FromStr + Display.

impl<T: std::str::FromStr + fmt::Display> BmsValue for T {
    #[inline]
    fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

// From / TryFrom conversions

impl TryFrom<BmsToken> for BmsHeader {
    type Error = BmsTryFromError;

    #[inline]
    fn try_from(token: BmsToken) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Header(h) => Ok(h),
            BmsToken::Message(_) => Err(BmsTryFromError::NotAHeader),
        }
    }
}

impl TryFrom<(NonZeroUsize, Result<Self, BmsTokenizeError>)> for BmsToken {
    type Error = BmsTryFromError;

    #[inline]
    fn try_from(pair: (NonZeroUsize, Result<Self, BmsTokenizeError>)) -> Result<Self, Self::Error> {
        let (line, result) = pair;
        result.map_err(|error| BmsTryFromError::TokenizationError { line, error })
    }
}

/// A single token produced by tokenizing a BMS file.
#[derive(Debug, Clone, PartialEq, derive_more::From)]
pub enum BmsToken {
    /// Header command (metadata, gameplay, timing, resources, and so on).
    Header(BmsHeader),
    /// Channel data line (`#xxxYY:values`).
    Message(BmsMessage),
}
