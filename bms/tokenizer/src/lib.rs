//! Tokenizer for BMS (Be-Music Script) format.
//!
//! This crate provides the first stage of the BMS parsing pipeline:
//! converting raw BMS text into a structured token stream.
//!
//! # Architecture
//!
//! - [`enum@BmsToken`] represents a single meaningful line in a BMS file.
//! - [`BmsHeader`] covers all header commands, categorized by semantic domain.
//! - [`BmsMessage`] covers channel data lines (`#xxxYY:values`).
//! - [`BmsTokenizer`] is the main entry point with configurable error strategy.
//!
//! Submodules are private; all public types are re-exported from the crate root.
//!
//! # Zero-copy
//!
//! String data (paths, display text) borrows from the input. Typed values
//! (numeric conversions, parsed enums) are owned.

use std::fmt;
use std::num::NonZeroUsize;

mod error;
mod header;
mod id;
mod message;

pub use bms_tokenizer_derive::BmsTokenAttr;
pub use error::{BmsTokenizeError, ParseBmsValueError};
pub use header::{
    ArgbParams, AtBgaParams, BgaParams, DifficultyLevel, ExBmpParams, ExWavParams, LnMode, LnType,
    ParseDifficultyError, PlayerMode, PoorBgaMode, SwBgaParams,
};
pub use header::{
    BmsHeader, BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderFallback, BmsHeaderGameplay,
    BmsHeaderMetadata, BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming,
};
pub use id::{
    AlphaNum, Base36Upper, BmpTag, BmsChannelId, BmsChannelIdError, BpmTag, ChannelTag, ExRankTag,
    Hex, LnObjTag, ScrollTag, SeekTag, SpeedTag, StopTag, WavTag,
};
pub use message::BmsMessage;

/// Unified trait for BMS header values.
///
/// Combines parsing (from an input string) and formatting (back to a BMS value
/// string) into a single contract.  Types that implement [`std::str::FromStr`] +
/// [`std::fmt::Display`] get a blanket implementation — no manual work needed for simple
/// numeric or identifier types.
///
/// # Lifetimes
///
/// The `'a` lifetime allows implementations to borrow from the input string
/// without allocating (e.g., `ExBmpParams<'a>`).  Owned-only types can safely
/// implement the trait with any `'a`.
///
/// # Formatting
///
/// This trait uses [`std::fmt::Display`] as a supertrait instead of providing its own
/// format method.  Callers use `.to_string()` to obtain the BMS
/// representation; this keeps the trait compatible with the standard library's
/// formatting infrastructure.
pub trait BmsValue<'a>: fmt::Display + Sized {
    /// Parse `s` into `Self`.
    ///
    /// Return `None` to signal that parsing failed — the caller may allow the
    /// input to fall through to `BmsHeaderFallback` instead of treating the
    /// failure as a hard error.
    #[must_use]
    fn parse(s: &'a str) -> Option<Self>;
}

// ── Blanket implementation ──────────────────────────────────────────────────
//
// Covers primitives (f64, u8, i32), BmsChannelId, PoorBgaMode, DifficultyLevel,
// and any other type that already implements FromStr + Display.

impl<'a, T> BmsValue<'a> for T
where
    T: std::str::FromStr + fmt::Display,
{
    #[inline]
    fn parse(s: &'a str) -> Option<Self> {
        s.parse().ok()
    }
}

use header::parse_header_line;
use message::parse_message_line;

/// A single token produced by tokenizing a BMS file.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsToken<'a> {
    /// A header command (metadata, gameplay, timing, resources, etc.).
    Header(BmsHeader<'a>),
    /// A channel data line (`#xxxYY:values`).
    Message(BmsMessage<'a>),
}

/// Error strategy for BMS tokenization.
///
/// Controls how the tokenizer handles lines that fail to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// Process every line; each line's result is wrapped individually.
    /// Tokenization continues past errors so the caller can inspect all
    /// failures at once.
    #[default]
    CollectAll,
    /// Stop at the first error and return results up to (and including)
    /// that line. Subsequent lines are not inspected.
    FailFast,
}

/// BMS tokenizer with builder-style configuration.
///
/// # Examples
///
/// ```
/// # use bms_tokenizer::{BmsTokenizer, ErrorStrategy};
/// let tokens: Vec<_> = BmsTokenizer::new()
///     .error_strategy(ErrorStrategy::CollectAll)
///     .tokenize("#TITLE My Song\n#00101:11");
/// ```
#[derive(Debug, Clone, Default)]
pub struct BmsTokenizer {
    /// Controls how parse errors are handled.
    error_strategy: ErrorStrategy,
}

impl BmsTokenizer {
    /// Create a new `BmsTokenizer` with default configuration
    /// ([`ErrorStrategy::CollectAll`]).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the error strategy.
    #[must_use]
    pub fn error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }

    /// Tokenize a BMS string into a collection of line-numbered results.
    ///
    /// Each element is a tuple of `(1-based line number, result)`.
    /// Empty lines and comments are skipped (no output entry).
    ///
    /// Supports LF (`\n`), CRLF (`\r\n`), and standalone CR (`\r`) line
    /// endings.
    ///
    /// The output collection type `C` is generic — use `Vec`, `Box<[_]>`,
    /// or any container that implements [`FromIterator`].
    ///
    /// # Error strategy
    ///
    /// - [`ErrorStrategy::CollectAll`] (default): all lines are processed.
    ///   Errors are embedded in per-element [`Result::Err`].
    /// - [`ErrorStrategy::FailFast`]: stops at the first error. The
    ///   collection contains results up to (and including) the error line.
    #[must_use]
    pub fn tokenize<'a, C>(&self, input: &'a str) -> C
    where
        C: FromIterator<(NonZeroUsize, Result<BmsToken<'a>, BmsTokenizeError<'a>>)>,
    {
        let mut results = Vec::new();
        let mut line_number: usize = 0;

        for raw_line in input.lines() {
            for segment in raw_line.split('\r') {
                line_number += 1;

                let trimmed = segment.trim();
                if trimmed.is_empty() || trimmed.starts_with("//") {
                    continue;
                }

                // SAFETY: line_number is always >= 1 here (incremented from 0
                // before first use); the fallback is unreachable.
                let nz_line = NonZeroUsize::new(line_number).unwrap_or(NonZeroUsize::MAX);

                let result: Result<BmsToken<'_>, BmsTokenizeError<'_>> =
                    match parse_message_line(trimmed) {
                        Ok(Some(msg)) => Ok(BmsToken::Message(msg)),
                        Ok(None) => match parse_header_line(trimmed) {
                            Ok(Some(hdr)) => Ok(BmsToken::Header(hdr)),
                            Ok(None) => continue,
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    };

                if self.error_strategy == ErrorStrategy::FailFast && result.is_err() {
                    results.push((nz_line, result));
                    return results.into_iter().collect();
                }

                results.push((nz_line, result));
            }
        }

        results.into_iter().collect()
    }
}
