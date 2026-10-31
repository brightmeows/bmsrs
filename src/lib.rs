//! Re-export facade for the bmsrs workspace.
//!
//! This crate aggregates and re-exports the public APIs of the sub-crates; the
//! module hierarchy mirrors the workspace directory structure. The whole-pipeline
//! entry points [`bms::from_text`] / [`bms::from_bytes`] also live here.

/// Parsing and processing for the BMS format.
pub mod bms {
    pub use parser::ErrorStrategy;

    /// Error type for the whole-pipeline entry points.
    #[derive(Debug, thiserror::Error)]
    pub enum FromTextError {
        /// CST syntax or control-flow structure error.
        #[error("syntax error at {range:?}: {message}")]
        Syntax {
            /// Text range.
            range: syntax::TextRange,
            /// Error message.
            message: String,
        },
        /// Single-line value parse error.
        #[error("tokenization error at {range:?}: {error}")]
        Tokenize {
            /// Text range.
            range: syntax::TextRange,
            /// Tokenizer error.
            error: parser::BmsTokenizeError,
        },
        /// Document decode error.
        #[error("decode error at {range:?}: {error}")]
        Decode {
            /// Text range.
            range: syntax::TextRange,
            /// Decode error.
            error: source::TextDecodeError,
        },
    }

    /// Non-fatal warnings from the whole-pipeline entry points.
    #[derive(Debug, Clone, PartialEq, thiserror::Error)]
    #[non_exhaustive]
    pub enum FromTextWarning {
        /// CST syntax or control-flow structure warning.
        #[error("syntax warning at {range:?}: {message}")]
        Syntax {
            /// Text range.
            range: syntax::TextRange,
            /// Warning message.
            message: String,
        },
        /// Single-line value parsing failed and the line was skipped.
        #[error("tokenization warning at {range:?}: {error}")]
        Tokenize {
            /// Text range.
            range: syntax::TextRange,
            /// Tokenizer error.
            error: parser::BmsTokenizeError,
        },
        /// Document decoding failed.
        #[error("decode warning at {range:?}: {error}")]
        Decode {
            /// Text range.
            range: syntax::TextRange,
            /// Decode error.
            error: source::TextDecodeError,
        },
    }

    /// Build [`Bms`](parser::Bms) from raw BMS text.
    ///
    /// Pipeline: CST parsing, control-flow branch selection, single-line value
    /// parsing, then `Bms`.
    ///
    /// # Errors
    ///
    /// `FailFast` returns [`FromTextError`] at the first CST or tokenizer error.
    pub fn from_text(
        text: &str,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        from_text_with(text, &syntax::ParseOptions::default(), rng, error_strategy)
    }

    /// Build [`Bms`](parser::Bms) from raw BMS text with parse options.
    ///
    /// # Errors
    ///
    /// `FailFast` returns [`FromTextError`] at the first CST, tokenizer, or
    /// decode error.
    pub fn from_text_with(
        text: &str,
        options: &syntax::ParseOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        let document = source::Document::from_text(text);
        from_document(&document, options, rng, error_strategy)
    }

    /// Build [`Bms`](parser::Bms) from raw BMS bytes, detecting the encoding
    /// automatically.
    ///
    /// Unlike [`from_text`], this preserves `bmsrs-bms-source`'s encoding
    /// detection and decoding path, suitable for non-UTF-8 charts such as
    /// `Shift_JIS`.
    ///
    /// # Errors
    ///
    /// `FailFast` returns [`FromTextError`] at the first CST, tokenizer, or
    /// decode error.
    pub fn from_bytes(
        bytes: Vec<u8>,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        from_bytes_with(
            bytes,
            &source::DocumentOptions::default(),
            rng,
            error_strategy,
        )
    }

    /// Build [`Bms`](parser::Bms) from raw BMS bytes with document decoding
    /// options.
    ///
    /// # Errors
    ///
    /// `FailFast` returns [`FromTextError`] at the first CST, tokenizer, or
    /// decode error.
    pub fn from_bytes_with(
        bytes: Vec<u8>,
        document_options: &source::DocumentOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        let document = source::Document::from_bytes_with(bytes, document_options);
        from_document(
            &document,
            &syntax::ParseOptions::default(),
            rng,
            error_strategy,
        )
    }

    /// Run the full semantic pipeline from an already-built document.
    fn from_document(
        document: &source::Document,
        options: &syntax::ParseOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        let parsed = parser::Bms::from_document_with(document, options, rng, error_strategy)
            .map_err(from_cst_error)?;
        Ok((
            parsed.bms,
            parsed.warnings.into_iter().map(from_cst_warning).collect(),
        ))
    }

    /// Convert a parser-layer warning into a facade warning.
    fn from_cst_warning(warning: parser::CstWarning) -> FromTextWarning {
        match warning.kind {
            parser::CstWarningKind::Syntax(message) => FromTextWarning::Syntax {
                range: warning.range,
                message,
            },
            parser::CstWarningKind::Tokenize(error) => FromTextWarning::Tokenize {
                range: warning.range,
                error,
            },
            parser::CstWarningKind::Decode(error) => FromTextWarning::Decode {
                range: warning.range,
                error,
            },
        }
    }

    /// Convert a parser-layer warning into a facade error.
    fn from_cst_error(warning: parser::CstWarning) -> FromTextError {
        match warning.kind {
            parser::CstWarningKind::Syntax(message) => FromTextError::Syntax {
                range: warning.range,
                message,
            },
            parser::CstWarningKind::Tokenize(error) => FromTextError::Tokenize {
                range: warning.range,
                error,
            },
            parser::CstWarningKind::Decode(error) => FromTextError::Decode {
                range: warning.range,
                error,
            },
        }
    }

    /// Source-text layer: bytes, encoding, line index, and byte-faithful editing.
    pub mod source {
        pub use bmsrs_bms_source::*;
    }

    /// Syntax layer: rowan CST, lexing, parsing, and control-flow queries.
    pub mod syntax {
        pub use bmsrs_bms_syntax::*;
    }

    /// BMS semantic layer: typed headers/messages and the `Bms` model.
    ///
    /// Canonical path for `Bms` and `Messages`. The canonical path for value types
    /// (`WavIndex` and so on) is [`values`]; the re-export here is only a
    /// convenience alias for parser users.
    pub mod parser {
        pub use bmsrs_bms_parser::*;
    }

    /// Single-line syntax and typed values: headers/messages, indices, parsing.
    ///
    /// Canonical path for value types.
    pub mod values {
        pub use bmsrs_bms_values::*;
    }

    /// Analysis layer: diagnostics, symbol index, and editor data.
    pub mod analysis {
        pub use bmsrs_bms_analysis::*;
    }

    /// BMS processor: converts `Bms` into a format-agnostic `Chart`.
    pub mod processor {
        pub use bmsrs_bms_processor::*;
    }

    /// BMS generator: rewrites the full `Bms` as BMS text.
    pub mod writer {
        pub use bmsrs_bms_writer::*;
    }
}

/// Type definitions and converters for the BMSON format.
pub mod bmson {
    /// BMSON type system (v0/v1/v2).
    pub mod def {
        pub use bmsrs_bmson_def::{v0, v1, *};
    }

    /// BMSON processor: converts bmson data into a format-agnostic `Chart`.
    pub mod processor {
        pub use bmsrs_bmson_processor::*;
    }
}

/// Format-agnostic chart data model.
pub mod chart {
    pub use bmsrs_chart::*;
}

/// Pure simulation and playback layer.
pub mod player {
    pub use bmsrs_player::*;
}

/// Cross-layer unified diagnostic exit.
///
/// Each layer keeps its own warning types; consumers that need a single
/// diagnostic stream convert here, without forcing the processor to depend on
/// analysis.
pub mod diagnostic {
    use bmsrs_bms_analysis::{AnalysisDiagnostic, Severity};
    use bmsrs_bms_syntax::TextRange;

    use crate::bms::FromTextWarning;

    /// Unified diagnostic.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Diagnostic {
        /// Severity.
        pub severity: Severity,
        /// Text range; semantic warnings may have none.
        pub range: Option<TextRange>,
        /// Message.
        pub message: String,
        /// Stable error code.
        pub code: &'static str,
    }

    impl Diagnostic {
        /// Construct a diagnostic.
        #[must_use]
        pub fn new(
            severity: Severity,
            range: Option<TextRange>,
            message: impl Into<String>,
            code: &'static str,
        ) -> Self {
            Self {
                severity,
                range,
                message: message.into(),
                code,
            }
        }
    }

    impl From<FromTextWarning> for Diagnostic {
        fn from(warning: FromTextWarning) -> Self {
            match warning {
                FromTextWarning::Syntax { range, message } => {
                    Self::new(Severity::Warning, Some(range), message, "syntax")
                }
                FromTextWarning::Tokenize { range, error } => Self::new(
                    Severity::Warning,
                    Some(range),
                    error.to_string(),
                    "tokenize",
                ),
                FromTextWarning::Decode { range, error } => {
                    Self::new(Severity::Warning, Some(range), error.to_string(), "decode")
                }
            }
        }
    }

    impl From<bmsrs_bms_processor::ProcessWarning> for Diagnostic {
        fn from(warning: bmsrs_bms_processor::ProcessWarning) -> Self {
            Self::new(Severity::Warning, None, warning.to_string(), "bms-process")
        }
    }

    impl From<bmsrs_bmson_processor::BmsonProcessWarning> for Diagnostic {
        fn from(warning: bmsrs_bmson_processor::BmsonProcessWarning) -> Self {
            Self::new(
                Severity::Warning,
                None,
                warning.to_string(),
                "bmson-process",
            )
        }
    }

    impl From<AnalysisDiagnostic> for Diagnostic {
        fn from(diagnostic: AnalysisDiagnostic) -> Self {
            Self::new(
                diagnostic.severity(),
                Some(diagnostic.range()),
                diagnostic.message(),
                diagnostic.code().unwrap_or("analysis"),
            )
        }
    }
}
