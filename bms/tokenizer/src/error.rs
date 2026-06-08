//! Tokenizer error types — the single error type for the BMS tokenizer.

use std::num::ParseFloatError;
use std::num::ParseIntError;

use thiserror::Error;

/// Errors that can occur during BMS tokenization.
///
/// All header parse errors are converted into this type via `From` impls,
/// so header variants can store typed values directly without a `Result`
/// wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BmsTokenizeError {
    /// The measure number in a channel line is not a valid 3-digit value.
    #[error("invalid measure number: \"{0}\"")]
    InvalidMeasure(String),
    /// The channel number in a channel line is not a valid 2-digit value.
    #[error("invalid channel number: \"{0}\"")]
    InvalidChannel(String),

    // -- Domain-specific header parse errors --
    /// `#PLAYER` value is not valid.
    #[error("invalid #PLAYER value: \"{0}\"")]
    InvalidPlayerMode(String),
    /// `#DIFFICULTY` value is not valid.
    #[error("invalid #DIFFICULTY value: \"{0}\"")]
    InvalidDifficulty(String),
    /// `#LNTYPE` value is not valid.
    #[error("invalid #LNTYPE value: \"{0}\"")]
    InvalidLnType(String),
    /// `#LNMODE` value is not valid.
    #[error("invalid #LNMODE value: \"{0}\"")]
    InvalidLnMode(String),
    /// A channel ID string is not a valid 1–2 char `0-9A-Za-z` sequence.
    #[error("invalid channel id: \"{0}\"")]
    InvalidChannelId(String),

    // -- Generic numeric parse errors --
    /// An integer field could not be parsed.
    #[error("invalid integer: {0}")]
    InvalidInteger(String),
    /// A float field could not be parsed.
    #[error("invalid float: {0}")]
    InvalidFloat(String),
}

// -- Std error conversions (lose original input string, keep description) --

impl From<ParseIntError> for BmsTokenizeError {
    fn from(e: ParseIntError) -> Self {
        BmsTokenizeError::InvalidInteger(e.to_string())
    }
}

impl From<ParseFloatError> for BmsTokenizeError {
    fn from(e: ParseFloatError) -> Self {
        BmsTokenizeError::InvalidFloat(e.to_string())
    }
}
