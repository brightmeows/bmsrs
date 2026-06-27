//! BMS header command parsing, categorized by semantic domain.

mod control_flow;
mod display;
mod gameplay;
mod metadata;
mod res_def_audio;
mod res_def_visual;
mod timing;

pub use control_flow::BmsHeaderControlFlow;
pub use display::{BmsHeaderDisplay, DifficultyLevel, ParseDifficultyError, PoorBgaMode};
pub use gameplay::{BmsHeaderGameplay, LnMode, LnType, PlayerMode, Rank};
pub use metadata::BmsHeaderMetadata;
pub use res_def_audio::{BmsHeaderResDefAudio, ExWavParams};
pub use res_def_visual::{
    ArgbParams, AtBgaParams, BgaParams, BmsHeaderResDefVisual, ExBmpParams, SwBgaParams,
};
pub use timing::{BmsHeaderTiming, StpParams};

use std::fmt;

use crate::BmsTokenAttr;
use crate::BmsTokenizeError;
use crate::BmsTryFromError;

/// A header command from a BMS file, categorized by semantic domain.
///
/// The dispatch order follows the variant declaration order below.
/// Variants annotated with `#[bms_fallback]` are excluded from dispatch
/// and instead catch anything that didn't match a concrete variant.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr, derive_more::From)]
pub enum BmsHeader<C> {
    /// Audio resource definitions (`#WAV`, `#EXWAV`, `#WAVCMD`, etc.).
    ResDefAudio(BmsHeaderResDefAudio<C>),
    /// Timing definitions (`#BPM`, `#STOP`, `#SCROLL`, `#SPEED`, etc.).
    Timing(BmsHeaderTiming),
    /// Visual resource definitions (`#BMP`, `#BGA`, `#ARGB`, etc.).
    ResDefVisual(BmsHeaderResDefVisual<C>),
    /// Control-flow commands (`#RANDOM`, `#SWITCH`, `#IF`, etc.).
    ControlFlow(BmsHeaderControlFlow),
    /// Gameplay behaviour (`#PLAYER`, `#RANK`, `#TOTAL`, `#LNTYPE`, etc.).
    Gameplay(BmsHeaderGameplay<C>),
    /// Display and difficulty markers (`#STAGEFILE`, `#DIFFICULTY`, etc.).
    Display(BmsHeaderDisplay<C>),
    /// Song/chart identification (`#TITLE`, `#ARTIST`, `#GENRE`, etc.).
    Metadata(BmsHeaderMetadata<C>),
    /// An unrecognised or engine-specific header command.
    #[bms_fallback]
    Fallback(BmsHeaderFallback<C>),
}

/// Catch-all for unrecognised header commands.
///
/// Captures the raw command name and value so that downstream consumers
/// (parsers, tools) can handle engine-specific extensions that the
/// tokenizer doesn't know about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsHeaderFallback<C> {
    /// The raw command name as it appears in the file (e.g., `"MYEXT"`).
    pub command: C,
    /// The value after the space separator.
    pub value: C,
}

// From / TryFrom conversions

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderFallback<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Fallback(f) => Ok(f),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}

/// Parse a single header line into a `BmsHeader`.
///
/// `prefixes` controls which leading characters are recognised as header
/// markers (default: `#` and `%`).  Lines not starting with one of the
/// configured prefixes are skipped.
///
/// Returns `Ok(None)` if the line is not a header (empty, comment, channel data).
///
/// # Errors
///
/// Returns `Err(BmsTokenizeError)` if a header command is recognised but its
/// value cannot be parsed into the expected type.
/// Parse with explicit supertraits for C (avoid `E0283` with `BmsStr` blanket impl).
/// In tests, use the non-generic `parse_header_line_default` wrapper.
#[expect(
    clippy::string_slice,
    reason = "BMS header lines are ASCII-only; byte indexing at whitespace boundaries is safe"
)]
pub fn parse_header_line<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a>(
    line: &'a str,
    prefixes: &[char],
) -> Result<Option<BmsHeader<C>>, BmsTokenizeError<C>> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    // Lines starting with `##` are comments.
    if trimmed.starts_with("##") {
        return Ok(None);
    }

    // Determine prefix character from the configured list.
    // trimmed is non-empty (checked above).
    let Some(first) = trimmed.chars().next() else {
        return Ok(None);
    };
    if !prefixes.contains(&first) {
        return Ok(None);
    }
    let prefix = first;
    let rest = &trimmed[prefix.len_utf8()..];

    if rest.is_empty() {
        return Ok(None);
    }

    // Split command name and value at first whitespace.
    let split_pos = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let command = &rest[..split_pos];
    let value = rest[split_pos..].trim();

    if command.is_empty() {
        return Ok(None);
    }

    // If the raw command contains a colon it is a channel message, not a header.
    if command.contains(':') {
        return Ok(None);
    }

    // `%` commands: only `%URL` and `%EMAIL` are valid BMS headers —
    // everything else is an engine-specific extension.  Unknown `%` commands
    // must NOT fall through (doing so would let `%TITLE` masquerade as
    // `#TITLE`).
    if prefix == '%'
        && !command.eq_ignore_ascii_case("URL")
        && !command.eq_ignore_ascii_case("EMAIL")
    {
        return Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
            command: C::from(command),
            value: C::from(value),
        })));
    }

    // Dispatch to sub-enum `try_match_header` functions.
    // `command` carries the input lifetime so that index-slice errors
    // can store the correct portion of the command.
    if let Some(header) = BmsHeader::try_match_header(command, value)? {
        return Ok(Some(header));
    }

    // Nothing matched → Fallback.
    Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
        command: C::from(command),
        value: C::from(value),
    })))
}
