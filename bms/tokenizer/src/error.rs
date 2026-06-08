//! Tokenizer error types — the single error type for the BMS tokenizer.

use std::fmt;
use std::num::ParseFloatError;
use std::num::ParseIntError;

/// Errors that can occur during BMS tokenization.
///
/// Variants are categorised into four groups:
/// - **Structural** — the line itself has invalid syntax.
/// - **Numeric** — a numeric field failed to parse (std errors, owned).
/// - **Invalid value** — the input is not a recognised value for its domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmsTokenizeError<'a> {
    /// The measure number in a channel line is not a valid 3-digit value.
    InvalidMeasure(&'a str),
    /// The channel number in a channel line is not a valid 2-digit value.
    InvalidChannel(&'a str),

    /// An integer field could not be parsed.
    InvalidInteger(String),
    /// A float field could not be parsed.
    InvalidFloat(String),

    /// The input is not a recognised value for its domain.
    InvalidValue {
        /// The header command name (e.g., `"#PLAYER"`).
        context: &'static str,
        /// The raw input that failed to parse.
        value: &'a str,
        /// Optional contextual hint (e.g., `Some(" (expected 1-5)")`).
        ///
        /// This is a static string determined at compile time by the code
        /// generator, not a dynamically computed message.
        detail: Option<&'static str>,
    },
}

impl fmt::Display for BmsTokenizeError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BmsTokenizeError::InvalidMeasure(v) => write!(f, "invalid measure number: \"{v}\""),
            BmsTokenizeError::InvalidChannel(v) => write!(f, "invalid channel number: \"{v}\""),
            BmsTokenizeError::InvalidInteger(v) => write!(f, "invalid integer: {v}"),
            BmsTokenizeError::InvalidFloat(v) => write!(f, "invalid float: {v}"),
            BmsTokenizeError::InvalidValue {
                context,
                value,
                detail,
            } => {
                write!(f, "invalid value \"{value}\" for {context}")?;
                if let Some(d) = detail {
                    write!(f, "{d}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for BmsTokenizeError<'_> {}

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
