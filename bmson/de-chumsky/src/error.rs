//! Error types for the bmson deserialization pipeline.

/// Errors that can occur during BMSON parsing and deserialization.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BmsonDeError {
    /// Fatal JSON parse error (chumsky produced no output).
    ///
    /// Contains the human-readable diagnostic messages from the parser.
    #[error("JSON parse error(s):\n{0}")]
    JsonParse(String),

    /// The bmson version string is missing or unrecognised.
    #[error("{0}")]
    UnknownVersion(String),

    /// Deserialization of a version-specific type from [`serde_json::Value`]
    /// failed (e.g. missing required field, type mismatch).
    #[error("Failed to deserialize {version} bmson: {message}")]
    Deserialize {
        /// Human-readable version identifier (e.g. `"v2.0.0"`, `"v1.0.0"`).
        version: &'static str,
        /// Underlying error description.
        message: String,
    },

    /// Conversion from the legacy v0.2.1 format to the unified v2 format
    /// failed (e.g. invalid `init_bpm`).
    #[error("V0 conversion error: {0}")]
    V0Conversion(String),
}

impl From<bmson_def::BmsonError> for BmsonDeError {
    fn from(e: bmson_def::BmsonError) -> Self {
        match e {
            bmson_def::BmsonError::UnknownVersion(v) => Self::UnknownVersion(v),
            bmson_def::BmsonError::V0Conversion(v) => Self::V0Conversion(v),
            _ => Self::UnknownVersion(e.to_string()),
        }
    }
}

impl From<bmson_def::v0::TryFromV0Error> for BmsonDeError {
    fn from(e: bmson_def::v0::TryFromV0Error) -> Self {
        Self::V0Conversion(e.message)
    }
}
