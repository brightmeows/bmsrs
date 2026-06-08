//! Tokenizer error types — the single error type for the BMS tokenizer.

use std::num::ParseFloatError;
use std::num::ParseIntError;

use thiserror::Error;

/// Errors that can occur during BMS tokenization.
///
/// Variants are categorised into four groups:
/// - **Structural** — the line itself has invalid syntax.
/// - **Numeric** — a numeric field failed to parse (std errors, owned).
/// - **Invalid value** — the input is not a recognised value for its domain.
/// - **Out of range** — the input is recognised but falls outside the domain.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BmsTokenizeError<'a> {
    /// The measure number in a channel line is not a valid 3-digit value.
    #[error("invalid measure number: \"{0}\"")]
    InvalidMeasure(&'a str),
    /// The channel number in a channel line is not a valid 2-digit value.
    #[error("invalid channel number: \"{0}\"")]
    InvalidChannel(&'a str),

    /// An integer field could not be parsed.
    #[error("invalid integer: {0}")]
    InvalidInteger(String),
    /// A float field could not be parsed.
    #[error("invalid float: {0}")]
    InvalidFloat(String),

    /// The input is not a recognised value for its domain.
    #[error("invalid value \"{value}\" for {context}")]
    InvalidValue {
        /// The header command name (e.g., `"#PLAYER"`).
        context: &'static str,
        /// The raw input that failed to parse.
        value: &'a str,
    },

    /// The input is valid in format but outside the allowed range.
    #[error("value \"{value}\" out of range for {context} (expected {expected})")]
    OutOfRange {
        /// The header command name (e.g., `"#DIFFICULTY"`).
        context: &'static str,
        /// The raw input value.
        value: &'a str,
        /// Description of the expected range (e.g., `"1-5"`).
        expected: &'static str,
    },
}

impl From<ParseIntError> for BmsTokenizeError<'_> {
    fn from(e: ParseIntError) -> Self {
        BmsTokenizeError::InvalidInteger(e.to_string())
    }
}

impl From<ParseFloatError> for BmsTokenizeError<'_> {
    fn from(e: ParseFloatError) -> Self {
        BmsTokenizeError::InvalidFloat(e.to_string())
    }
}
