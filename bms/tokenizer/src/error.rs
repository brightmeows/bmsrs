//! Tokenizer error types — the single error type for the BMS tokenizer.

use std::fmt;
use std::num::NonZeroUsize;
use std::num::ParseFloatError;
use std::num::ParseIntError;

use thiserror::Error;

/// Errors that can occur during BMS tokenization.
///
/// Every variant carries the original input `value` (as the string container
/// `C`) so callers can inspect or display the raw text that caused the failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmsTokenizeError<C> {
    /// The measure number in a channel line is not a valid 3-digit value.
    InvalidMeasure {
        /// The raw measure string that failed validation.
        value: C,
    },
    /// The channel number in a channel line is not a valid 2-digit value.
    InvalidChannel {
        /// The raw channel string that failed validation.
        value: C,
    },
    /// An integer field could not be parsed.
    InvalidInteger {
        /// The raw input that could not be parsed as an integer.
        value: C,
    },
    /// A float field could not be parsed.
    InvalidFloat {
        /// The raw input that could not be parsed as a float.
        value: C,
    },
    /// The input is not a recognised value for its context.
    ///
    /// This covers both "out of range" (the value has the right shape but
    /// falls outside valid bounds) and "unrecognised" (the value does not
    /// match any known option for a literal enum).  When no specific valid
    /// set is available, `expected` is the empty string.
    OutOfRange {
        /// The header command name (e.g., `"#DIFFICULTY"`).
        context: &'static str,
        /// The raw input that is out of range or unrecognised.
        value: C,
        /// A description of the valid range (e.g., `"1-5"`, `"1 or 2"`), or
        /// the empty string when no specific hint is available.
        expected: &'static str,
    },
}

impl<C> BmsTokenizeError<C> {
    /// Convert a borrowed `BmsTokenizeError<&str>` into this container type.
    #[must_use]
    #[expect(clippy::needless_pass_by_value, reason = "consumed to move values out")]
    pub(crate) fn from_ref<'a>(err: BmsTokenizeError<&'a str>) -> Self
    where
        C: From<&'a str>,
    {
        match err {
            BmsTokenizeError::InvalidMeasure { value } => Self::InvalidMeasure {
                value: C::from(value),
            },
            BmsTokenizeError::InvalidChannel { value } => Self::InvalidChannel {
                value: C::from(value),
            },
            BmsTokenizeError::InvalidInteger { value } => Self::InvalidInteger {
                value: C::from(value),
            },
            BmsTokenizeError::InvalidFloat { value } => Self::InvalidFloat {
                value: C::from(value),
            },
            BmsTokenizeError::OutOfRange {
                context,
                value,
                expected,
            } => Self::OutOfRange {
                context,
                value: C::from(value),
                expected,
            },
        }
    }
}

impl<C: fmt::Display> fmt::Display for BmsTokenizeError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMeasure { value } => {
                write!(f, "invalid measure number: \"{value}\"")
            }
            Self::InvalidChannel { value } => {
                write!(f, "invalid channel number: \"{value}\"")
            }
            Self::InvalidInteger { value } => write!(f, "invalid integer: \"{value}\""),
            Self::InvalidFloat { value } => write!(f, "invalid float: \"{value}\""),
            Self::OutOfRange {
                context,
                value,
                expected,
            } => write!(
                f,
                "value out of range: \"{value}\" for {context} (expected {expected})"
            ),
        }
    }
}

impl<C: fmt::Debug + fmt::Display> std::error::Error for BmsTokenizeError<C> {}

/// Conversion from a `FromStr::Err` into a [`BmsTokenizeError`].
///
/// The derive macro for `#[derive(BmsTokenAttr)]` calls this trait to
/// transform any parse error into the tokenizer's unified error type.
/// Implementations are provided for [`ParseIntError`], [`ParseFloatError`],
/// [`ParseBmsValueError`], and — for custom `FromStr` impls that already
/// produce a `BmsTokenizeError` — [`BmsTokenizeError`] itself (identity).
pub trait IntoTokensError<C> {
    /// Convert this error into a `BmsTokenizeError`.
    ///
    /// * `context` — the header command name (e.g. `"#PLAYER"`).
    /// * `value` — the raw input string that failed to parse.
    fn into_error(self, context: &'static str, value: C) -> BmsTokenizeError<C>;
}

impl<C> IntoTokensError<C> for BmsTokenizeError<C> {
    fn into_error(self, context: &'static str, value: C) -> BmsTokenizeError<C> {
        match self {
            BmsTokenizeError::OutOfRange {
                context: "",
                expected,
                ..
            } => BmsTokenizeError::OutOfRange {
                context,
                value,
                expected,
            },
            other => other,
        }
    }
}

impl<C> IntoTokensError<C> for ParseIntError {
    fn into_error(self, _context: &'static str, value: C) -> BmsTokenizeError<C> {
        BmsTokenizeError::InvalidInteger { value }
    }
}

impl<C> IntoTokensError<C> for ParseFloatError {
    fn into_error(self, _context: &'static str, value: C) -> BmsTokenizeError<C> {
        BmsTokenizeError::InvalidFloat { value }
    }
}

impl<C> IntoTokensError<C> for ParseBmsValueError {
    fn into_error(self, context: &'static str, value: C) -> BmsTokenizeError<C> {
        BmsTokenizeError::OutOfRange {
            context,
            value,
            expected: self.0,
        }
    }
}

/// The input could not be parsed as the expected BMS value type.
///
/// Returned by [`std::str::FromStr`] impls generated by `#[derive(BmsTokenAttr)]`
/// in literal mode.
///
/// When non-empty, the string provides a human-readable hint about the expected
/// values (e.g., `"expected 1 or 2"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBmsValueError(pub &'static str);

impl fmt::Display for ParseBmsValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "invalid BMS value")
        } else {
            write!(f, "expected {}", self.0)
        }
    }
}

impl std::error::Error for ParseBmsValueError {}

/// Error type for `TryFrom` conversions between token types.
///
/// Used when extracting a specific header variant or message from a
/// [`BmsToken`](crate::BmsToken), [`BmsHeader`](crate::BmsHeader), or
/// `(NonZeroUsize, Result<BmsToken, BmsTokenizeError>)` pair.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BmsTryFromError<C> {
    /// The source `(NonZeroUsize, Result<BmsToken, …>)` contained an `Err`.
    #[error("tokenization error on line {line}: {error}")]
    TokenizationError {
        /// The 1-based line number where the error occurred.
        line: NonZeroUsize,
        /// The underlying tokenization error.
        error: BmsTokenizeError<C>,
    },
    /// The `BmsToken` is a `Message`, not a `Header`.
    #[error("expected a header, but the token is a channel message")]
    NotAHeader,
    /// The `BmsToken` is a `Header`, not a `Message`.
    #[error("expected a message, but the token is a header")]
    NotAMessage,
    /// The `BmsHeader` variant does not match the requested header type.
    #[error("the header is not of the requested type")]
    WrongHeaderType,
}
