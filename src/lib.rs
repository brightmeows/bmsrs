//! Re-export facade for the bmsrs workspace.
//!
//! This crate aggregates the public APIs of the sub-crates. The [`bms`] and
//! [`bmson`] modules keep only the whole-pipeline entry points and
//! facade-owned types; [`raw`] mirrors the workspace directory structure with
//! the full layer exports.

/// Parsing and processing for the BMS format.
pub mod bms {
    // Layer crates live under `raw`; the curated path keeps only the entry
    // points and facade-owned types.
    use crate::raw::{model, parser, processor, source, syntax, values, writer};

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
            error: values::BmsTokenizeError,
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
            error: values::BmsTokenizeError,
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

    /// Processed BMS chart and its format-specific sidecars.
    #[derive(Debug)]
    pub struct BmsProcessed {
        /// Format-agnostic chart.
        pub chart: bmsrs_chart::Chart<(), processor::custom_event::BmsCustomEvent>,
        /// Raw BMS gameplay parameters.
        pub gameplay: processor::BmsGameplay,
        /// Raw BMS audio parameters.
        pub audio: processor::BmsAudio,
    }

    /// Parse BMS text with defaults: first branch and `CollectAll`.
    ///
    /// # Errors
    ///
    /// Returns [`FromTextError`] when parsing fails.
    pub fn parse(
        text: &str,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, FromTextWarning>, FromTextError> {
        parse_with(
            text,
            &syntax::ParseOptions::default(),
            |_| 1,
            ErrorStrategy::CollectAll,
        )
    }

    /// Parse BMS text with explicit options.
    ///
    /// # Errors
    ///
    /// Returns [`FromTextError`] when `FailFast` encounters an error.
    pub fn parse_with(
        text: &str,
        options: &syntax::ParseOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, FromTextWarning>, FromTextError> {
        from_text_with(text, options, rng, error_strategy)
            .map(|(bms, warnings)| bmsrs_pipeline::Outcome::new(bms, warnings))
    }

    /// Parse BMS bytes with defaults: first branch and `CollectAll`.
    ///
    /// # Errors
    ///
    /// Returns [`FromTextError`] when parsing or decoding fails.
    pub fn parse_bytes(
        bytes: Vec<u8>,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, FromTextWarning>, FromTextError> {
        from_bytes(bytes, |_| 1, ErrorStrategy::CollectAll)
            .map(|(bms, warnings)| bmsrs_pipeline::Outcome::new(bms, warnings))
    }

    /// Parse BMS bytes with explicit decoding and parse options.
    ///
    /// # Errors
    ///
    /// Returns [`FromTextError`] when parsing or decoding fails.
    pub fn parse_bytes_with(
        bytes: Vec<u8>,
        doc_options: &source::DocumentOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, FromTextWarning>, FromTextError> {
        from_bytes_with(bytes, doc_options, rng, error_strategy)
            .map(|(bms, warnings)| bmsrs_pipeline::Outcome::new(bms, warnings))
    }

    /// Convert `Bms` to a format-agnostic chart with the default layout.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process(
        bms: &model::Bms,
    ) -> Result<
        bmsrs_pipeline::Outcome<BmsProcessed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        let processed = processor::BmsProcessor::process_default(bms)?;
        let processor::Processed {
            chart,
            warnings,
            gameplay,
            audio,
        } = processed;
        Ok(bmsrs_pipeline::Outcome::new(
            BmsProcessed {
                chart,
                gameplay,
                audio,
            },
            warnings,
        ))
    }

    /// Convert `Bms` to a chart with an explicit layout family.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with<L: processor::layout::BmsLayout>(
        bms: &model::Bms,
    ) -> Result<
        bmsrs_pipeline::Outcome<BmsProcessed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        let processed = processor::BmsProcessor::process::<L>(bms)?;
        let processor::Processed {
            chart,
            warnings,
            gameplay,
            audio,
        } = processed;
        Ok(bmsrs_pipeline::Outcome::new(
            BmsProcessed {
                chart,
                gameplay,
                audio,
            },
            warnings,
        ))
    }

    /// Write the full `Bms` model as canonical BMS text.
    #[must_use]
    pub fn write(bms: &model::Bms) -> String {
        writer::BmsWriter::write(bms)
    }

    /// Build [`Bms`](model::Bms) from raw BMS text.
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
    ) -> Result<(model::Bms, Vec<FromTextWarning>), FromTextError> {
        from_text_with(text, &syntax::ParseOptions::default(), rng, error_strategy)
    }

    /// Build [`Bms`](model::Bms) from raw BMS text with parse options.
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
    ) -> Result<(model::Bms, Vec<FromTextWarning>), FromTextError> {
        let document = source::Document::from_text(text);
        from_document(&document, options, rng, error_strategy)
    }

    /// Build [`Bms`](model::Bms) from raw BMS bytes, detecting the encoding
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
    ) -> Result<(model::Bms, Vec<FromTextWarning>), FromTextError> {
        from_bytes_with(
            bytes,
            &source::DocumentOptions::default(),
            rng,
            error_strategy,
        )
    }

    /// Build [`Bms`](model::Bms) from raw BMS bytes with document decoding
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
    ) -> Result<(model::Bms, Vec<FromTextWarning>), FromTextError> {
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
    ) -> Result<(model::Bms, Vec<FromTextWarning>), FromTextError> {
        let parsed = parser::BmsParser::parse_document_with(document, options, rng, error_strategy)
            .map_err(from_parse_error)?;
        Ok((
            parsed.bms,
            parsed
                .warnings
                .into_iter()
                .map(from_parse_warning)
                .collect(),
        ))
    }

    /// Convert a parser-layer warning into a facade warning.
    fn from_parse_warning(warning: parser::BmsParseWarning) -> FromTextWarning {
        match warning.kind {
            parser::BmsParseWarningKind::Syntax(message) => FromTextWarning::Syntax {
                range: warning.range,
                message,
            },
            parser::BmsParseWarningKind::Tokenize(error) => FromTextWarning::Tokenize {
                range: warning.range,
                error,
            },
            parser::BmsParseWarningKind::Decode(error) => FromTextWarning::Decode {
                range: warning.range,
                error,
            },
        }
    }

    /// Convert a parser-layer warning into a facade error.
    fn from_parse_error(warning: parser::BmsParseWarning) -> FromTextError {
        match warning.kind {
            parser::BmsParseWarningKind::Syntax(message) => FromTextError::Syntax {
                range: warning.range,
                message,
            },
            parser::BmsParseWarningKind::Tokenize(error) => FromTextError::Tokenize {
                range: warning.range,
                error,
            },
            parser::BmsParseWarningKind::Decode(error) => FromTextError::Decode {
                range: warning.range,
                error,
            },
        }
    }
}

/// Type definitions and converters for the BMSON format.
pub mod bmson {
    // The bmson processor types resolve through `raw`; the curated path keeps
    // only the entry points and facade-owned types.
    use crate::raw::bmson_processor as processor;

    /// Processed BMSON chart.
    #[derive(Debug)]
    pub struct BmsonProcessed {
        /// Format-agnostic chart.
        pub chart: bmsrs_chart::Chart<processor::BmsonNoteExt>,
    }

    /// Parse a BMSON JSON string, upgrading v0/v1 to the root v2 schema.
    ///
    /// # Errors
    ///
    /// Returns [`BmsonParseError`] when the JSON or version is invalid.
    #[cfg(feature = "bmson")]
    pub fn parse_str(json: &str) -> Result<bmsrs_bmson_model::Bmson, BmsonParseError> {
        match bmsrs_bmson_model::DetectedVersion::detect(json)? {
            bmsrs_bmson_model::DetectedVersion::V2 => Ok(serde_json::from_str(json)?),
            bmsrs_bmson_model::DetectedVersion::V1 => {
                let v1: bmsrs_bmson_model::v1::Bmson = serde_json::from_str(json)?;
                Ok(bmsrs_bmson_model::Bmson::from(v1))
            }
            bmsrs_bmson_model::DetectedVersion::V0 => {
                let v0: bmsrs_bmson_model::v0::Bmson = serde_json::from_str(json)?;
                v0.try_into()
                    .map_err(|error: bmsrs_bmson_model::v0::TryFromV0Error| {
                        BmsonParseError::Version(bmsrs_bmson_model::BmsonError::V0Conversion(
                            error.to_string(),
                        ))
                    })
            }
        }
    }

    /// Parse BMSON JSON bytes, upgrading v0/v1 to the root v2 schema.
    ///
    /// # Errors
    ///
    /// Returns [`BmsonParseError`] when the JSON, UTF-8, or version is invalid.
    #[cfg(feature = "bmson")]
    pub fn parse_slice(bytes: &[u8]) -> Result<bmsrs_bmson_model::Bmson, BmsonParseError> {
        let json = std::str::from_utf8(bytes)?;
        parse_str(json)
    }

    /// Convert `Bmson` to a format-agnostic chart with the default layout.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process(
        bmson: &bmsrs_bmson_model::Bmson,
    ) -> Result<
        bmsrs_pipeline::Outcome<BmsonProcessed, processor::BmsonProcessWarning>,
        processor::ProcessError,
    > {
        let processed = processor::BmsonProcessor::process_default(bmson)?;
        let processor::Processed { chart, warnings } = processed;
        Ok(bmsrs_pipeline::Outcome::new(
            BmsonProcessed { chart },
            warnings,
        ))
    }

    /// Convert `Bmson` with an explicit layout family.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with<L: processor::layout::BmsonLayout>(
        bmson: &bmsrs_bmson_model::Bmson,
    ) -> Result<
        bmsrs_pipeline::Outcome<BmsonProcessed, processor::BmsonProcessWarning>,
        processor::ProcessError,
    > {
        let processed = processor::BmsonProcessor::process::<L>(bmson)?;
        let processor::Processed { chart, warnings } = processed;
        Ok(bmsrs_pipeline::Outcome::new(
            BmsonProcessed { chart },
            warnings,
        ))
    }

    /// Convert `Bmson` with an explicit generic key count.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_nkeys(
        bmson: &bmsrs_bmson_model::Bmson,
        keys: u8,
    ) -> Result<
        bmsrs_pipeline::Outcome<BmsonProcessed, processor::BmsonProcessWarning>,
        processor::ProcessError,
    > {
        let processed = processor::BmsonProcessor::process_nkeys(bmson, keys)?;
        let processor::Processed { chart, warnings } = processed;
        Ok(bmsrs_pipeline::Outcome::new(
            BmsonProcessed { chart },
            warnings,
        ))
    }

    /// BMSON parse error.
    #[cfg(feature = "bmson")]
    #[derive(Debug, thiserror::Error)]
    pub enum BmsonParseError {
        /// JSON parse error.
        #[error("JSON parse error: {0}")]
        Json(#[from] serde_json::Error),
        /// Input is not valid UTF-8.
        #[error("BMSON input is not UTF-8: {0}")]
        Utf8(#[from] std::str::Utf8Error),
        /// Version detection or conversion error.
        #[error("BMSON version error: {0}")]
        Version(#[from] bmsrs_bmson_model::BmsonError),
    }
}

/// Format-agnostic chart data model.
pub mod chart {
    pub use bmsrs_chart::*;
}

/// Shared pipeline result and diagnostic types.
pub mod pipeline {
    pub use bmsrs_pipeline::*;
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

/// Low-level crates for advanced and editor integrations.
pub mod raw {
    /// BMS source-text layer.
    pub mod source {
        pub use bmsrs_bms_source::*;
    }

    /// BMS syntax layer.
    pub mod syntax {
        pub use bmsrs_bms_syntax::*;
    }

    /// BMS single-line values.
    pub mod values {
        pub use bmsrs_bms_values::*;
    }

    /// BMS whole-file model.
    pub mod model {
        pub use bmsrs_bms_model::*;
    }

    /// BMS semantic parser.
    pub mod parser {
        pub use bmsrs_bms_parser::*;
    }

    /// BMS analysis layer: diagnostics, symbol index, and editor data.
    pub mod analysis {
        pub use bmsrs_bms_analysis::*;
    }

    /// BMS processor.
    pub mod processor {
        pub use bmsrs_bms_processor::*;
    }

    /// BMS writer.
    pub mod writer {
        pub use bmsrs_bms_writer::*;
    }

    /// BMSON model.
    pub mod bmson_model {
        pub use bmsrs_bmson_model::*;
    }

    /// BMSON processor.
    pub mod bmson_processor {
        pub use bmsrs_bmson_processor::*;
    }
}

/// Common imports for application code.
pub mod prelude {
    pub use crate::bms::{
        BmsProcessed, FromTextError, FromTextWarning, parse, parse_bytes, process, write,
    };
    #[cfg(feature = "bmson")]
    pub use crate::bmson::BmsonParseError;
    pub use crate::bmson::BmsonProcessed;
    pub use crate::chart::{
        Chart, ChartData, Event, EventKind, Lane, Note, NoteKind, NoteRole, TimingTrack,
    };
    pub use crate::pipeline::{Diagnostic, Outcome, Severity, SourceSpan};
    pub use crate::player::Player;
    pub use crate::raw::bmson_model::Bmson;
    pub use crate::raw::model::Bms;
}

pub use bmsrs_pipeline::{Diagnostic, Outcome, Severity, SourceSpan};
