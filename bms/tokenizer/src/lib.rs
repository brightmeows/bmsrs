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
//! # String storage
//!
//! String data uses the type parameter `C` (defaults to `&str` for zero-copy,
//! or `String` for owned). Typed values (numeric conversions, parsed enums)
//! are always owned.
//! [`ErrorStrategy::FailFast`] stops at the first error — useful for
//! interactive validation where immediate feedback is preferred.

use std::fmt;
use std::num::NonZeroUsize;

mod channel;
mod error;
mod header;
mod index;
mod message;
mod preprocess;

pub use bms_tokenizer_derive::BmsTokenAttr;
pub use channel::BmsChannel;
pub use error::{BmsTokenizeError, BmsTryFromError, IntoTokensError, ParseBmsValueError};
pub use header::parse_header_line;
pub use header::{
    ArgbParams, AtBgaParams, BgaParams, DifficultyLevel, ExBmpParams, ExWavParams, LnMode, LnType,
    ParseDifficultyError, PlayerMode, PoorBgaMode, Rank, StpParams, SwBgaParams,
};
pub use header::{
    BmsHeader, BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderFallback, BmsHeaderGameplay,
    BmsHeaderMetadata, BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming,
};
pub use index::{
    BmpIndex, BmsBase, BmsIndex, BmsIndexError, BpmIndex, ChangeOptionIndex, ChannelIndex,
    ExRankIndex, LnObjIndex, ObjectIndex, ScrollIndex, SeekIndex, SpeedIndex, StopIndex, TextIndex,
    WavIndex,
};
pub use message::BmsMessage;
pub use message::parse_message_line;
pub use preprocess::preprocess;

/// Unified trait for BMS header values.
///
/// Combines parsing (from an input string) and formatting (back to a BMS value
/// string) into a single contract.  Types that implement [`std::str::FromStr`] +
/// [`std::fmt::Display`] get a blanket implementation — no manual work needed for simple
/// numeric or identifier types.
///
/// # Lifetimes
///
/// The `'a` lifetime is the input string's lifetime — implementations may
/// borrow from it without allocating (e.g., `ExBmpParams<'a>`).  Owned-only
/// types can safely implement the trait with any `'a`.
///
/// # Type parameters
///
/// `C` is the string container type used downstream (e.g., `&str`, `String`,
/// `Cow<'_, str>`).  The parameter exists so that consumers can choose between
/// zero-copy and owned storage.
///
/// # Formatting
///
/// This trait uses [`std::fmt::Display`] as a supertrait instead of providing its own
/// format method.  Callers use `.to_string()` to obtain the BMS
/// representation; this keeps the trait compatible with the standard library's
/// formatting infrastructure.
pub trait BmsValue<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a = &'a str>:
    fmt::Display + Sized
{
    /// Parse `s` into `Self`.
    ///
    /// Return `None` to signal that parsing failed — the caller may allow the
    /// input to fall through to `BmsHeaderFallback` instead of treating the
    /// failure as a hard error.
    #[must_use]
    fn parse(s: &'a str) -> Option<Self>;
}

// Covers primitives (f64, u8, i32), BmsIndex, PoorBgaMode, DifficultyLevel,
// and any other type that already implements FromStr + Display.

impl<'a, C, T> BmsValue<'a, C> for T
where
    C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a,
    T: std::str::FromStr + fmt::Display,
{
    #[inline]
    fn parse(s: &'a str) -> Option<Self> {
        s.parse().ok()
    }
}

// From / TryFrom conversions

impl<C> TryFrom<BmsToken<C>> for BmsHeader<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(token: BmsToken<C>) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Header(h) => Ok(h),
            BmsToken::Message(_) => Err(BmsTryFromError::NotAHeader),
        }
    }
}

impl<C> TryFrom<(NonZeroUsize, Result<Self, BmsTokenizeError<C>>)> for BmsToken<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(
        pair: (NonZeroUsize, Result<Self, BmsTokenizeError<C>>),
    ) -> Result<Self, Self::Error> {
        let (line, result) = pair;
        result.map_err(|error| BmsTryFromError::TokenizationError { line, error })
    }
}

/// A single token produced by tokenizing a BMS file.
#[derive(Debug, Clone, PartialEq, derive_more::From)]
pub enum BmsToken<C> {
    /// A header command (metadata, gameplay, timing, resources, etc.).
    Header(BmsHeader<C>),
    /// A channel data line (`#xxxYY:values`).
    Message(BmsMessage<C>),
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
///     .tokenize::<_, &str>("#TITLE My Song\n#00101:11");
/// ```
#[derive(Debug, Clone)]
pub struct BmsTokenizer {
    /// Controls how parse errors are handled.
    error_strategy: ErrorStrategy,
    /// Allowed prefix characters for header commands.
    header_prefixes: Vec<char>,
}

impl Default for BmsTokenizer {
    fn default() -> Self {
        Self {
            error_strategy: ErrorStrategy::default(),
            header_prefixes: vec!['#', '%'],
        }
    }
}

impl BmsTokenizer {
    /// Create a new `BmsTokenizer` with default configuration
    /// ([`ErrorStrategy::CollectAll`], prefixes `#` and `%`).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the error strategy.
    #[must_use]
    pub const fn error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }

    /// Set the allowed header prefix characters.
    ///
    /// Only lines starting with one of these characters will be treated as
    /// potential header commands.  Default: `['#', '%']`.
    #[must_use]
    pub fn header_prefixes(mut self, prefixes: &[char]) -> Self {
        self.header_prefixes = prefixes.to_vec();
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
    /// # Type parameters
    ///
    /// - `Out` — the output collection type (e.g., `Vec`, `Box<[_]>`).
    /// - `C` — the string container type.  Defaults to `&'a str` for
    ///   zero-copy tokenization.
    ///
    /// # Error strategy
    ///
    /// - [`ErrorStrategy::CollectAll`] (default): all lines are processed.
    ///   Errors are embedded in per-element [`Result::Err`].
    /// - [`ErrorStrategy::FailFast`]: stops at the first error. The
    ///   collection contains results up to (and including) the error line.
    #[must_use]
    pub fn tokenize<'a, Out, C>(&self, input: &'a str) -> Out
    where
        Out: FromIterator<(NonZeroUsize, Result<BmsToken<C>, BmsTokenizeError<C>>)>,
        C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a,
    {
        let mut results = Vec::new();
        let mut line_number: usize = 0;

        for raw_line in input.lines() {
            for segment in raw_line.split('\r') {
                line_number += 1;

                let trimmed = segment.trim();
                if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with(';') {
                    continue;
                }

                // line_number is always >= 1 here (incremented from 0
                // before first use); the fallback is unreachable.
                let nz_line = NonZeroUsize::new(line_number).unwrap_or(NonZeroUsize::MAX);

                let result: Result<BmsToken<C>, BmsTokenizeError<C>> =
                    match parse_message_line::<C>(trimmed) {
                        Ok(Some(msg)) => Ok(BmsToken::Message(msg)),
                        Ok(None) => match parse_header_line::<C>(trimmed, &self.header_prefixes) {
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
