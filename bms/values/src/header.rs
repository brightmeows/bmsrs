//! BMS header command parsing, grouped by semantic domain.

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
pub use res_def_audio::{BmsHeaderResDefAudio, ExWavParams, WavCmdKind, WavCmdParams};
pub use res_def_visual::{
    ArgbParams, AtBgaParams, BgaParams, BmsHeaderResDefVisual, ExBmpParams, SwBgaParams,
};
pub use timing::{BmsHeaderTiming, StpParams};

use crate::BmsTokenAttr;
use crate::BmsTokenizeError;

/// Header commands in a BMS file, grouped by semantic domain.
///
/// Dispatch follows the variant declaration order below. A variant marked
/// `#[bms_fallback]` does not participate in dispatch; it captures anything
/// that does not match a concrete variant.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr, derive_more::From, derive_more::TryUnwrap)]
pub enum BmsHeader {
    /// Audio resource definitions (`#WAV`, `#EXWAV`, `#WAVCMD`, and so on).
    ResDefAudio(BmsHeaderResDefAudio),
    /// Timing definitions (`#BPM`, `#STOP`, `#SCROLL`, `#SPEED`, and so on).
    Timing(BmsHeaderTiming),
    /// Visual resource definitions (`#BMP`, `#BGA`, `#ARGB`, and so on).
    ResDefVisual(BmsHeaderResDefVisual),
    /// Control-flow commands (`#RANDOM`, `#SWITCH`, `#IF`, and so on).
    ControlFlow(BmsHeaderControlFlow),
    /// Gameplay behavior (`#PLAYER`, `#RANK`, `#TOTAL`, `#LNTYPE`, and so on).
    Gameplay(BmsHeaderGameplay),
    /// Display and difficulty markers (`#STAGEFILE`, `#DIFFICULTY`, and so on).
    Display(BmsHeaderDisplay),
    /// Song and chart identity (`#TITLE`, `#ARTIST`, `#GENRE`, and so on).
    Metadata(BmsHeaderMetadata),
    /// Unrecognized or engine-specific header commands.
    #[bms_fallback]
    Fallback(BmsHeaderFallback),
}

/// Fallback type for unrecognized header commands.
///
/// Captures the raw command name and value so downstream consumers (parsers,
/// tools) can handle engine extensions the tokenizer does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsHeaderFallback {
    /// Raw command name as it appears in the file (for example, `"MYEXT"`).
    pub command: String,
    /// Value after the space separator.
    pub value: String,
}

/// Parse a single header line into a `BmsHeader`.
///
/// `prefixes` controls which leading characters are recognized as header
/// markers (default: `#` and `%`). Lines that do not start with a configured
/// prefix are skipped.
///
/// Returns `Ok(None)` when the line is not a header (blank line, comment, or
/// channel data).
///
/// # Errors
///
/// Returns `Err(BmsTokenizeError)` when a header command is recognized but its
/// value cannot be parsed into the expected type.
/// Explicitly resolve the parent trait for C (avoiding `E0283` from the
/// `BmsStr` blanket implementation). Tests use the non-generic
/// `parse_header_line_default` wrapper.
#[expect(
    clippy::string_slice,
    reason = "BMS header lines are ASCII-only; byte indexing at whitespace boundaries is safe"
)]
pub fn parse_header_line(
    line: &str,
    prefixes: &[char],
) -> Result<Option<BmsHeader>, BmsTokenizeError> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    // Lines starting with `##` are comments.
    if trimmed.starts_with("##") {
        return Ok(None);
    }

    // Determine the prefix character from the configured list.
    // `trimmed` is non-empty (checked above).
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

    // Split the command name and value at the first whitespace.
    let split_pos = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let command = &rest[..split_pos];
    let value = rest[split_pos..].trim();

    if command.is_empty() {
        return Ok(None);
    }

    // If the raw command contains a colon, it is a channel message, not a
    // header.
    if command.contains(':') {
        return Ok(None);
    }

    // `%` commands: only `%URL` and `%EMAIL` are valid BMS headers; everything
    // else is an engine-specific extension. Unknown `%` commands must not fall
    // back (otherwise `%TITLE` could impersonate `#TITLE`).
    if prefix == '%'
        && !command.eq_ignore_ascii_case("URL")
        && !command.eq_ignore_ascii_case("EMAIL")
    {
        return Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
            command: command.to_owned(),
            value: value.to_owned(),
        })));
    }

    // Dispatch to the sub-enum `try_match_header` functions.
    // `command` carries the input lifetime so index-slice errors can store the
    // correct part of the command.
    if let Some(header) = BmsHeader::try_match_header(command, value)? {
        return Ok(Some(header));
    }

    // No match: use the fallback.
    Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
        command: command.to_owned(),
        value: value.to_owned(),
    })))
}
