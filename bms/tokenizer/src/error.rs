use thiserror::Error;

/// Errors that can occur during BMS tokenization.
#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
pub enum TokenizerError {
    /// The measure number in a channel line is not a valid 3-digit value.
    #[error("invalid measure number: \"{0}\"")]
    InvalidMeasure(String),
    /// The channel number in a channel line is not a valid 2-digit value.
    #[error("invalid channel number: \"{0}\"")]
    InvalidChannel(String),
}
