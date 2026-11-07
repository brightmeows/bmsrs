//! Re-export facade for the bmsrs workspace.
//!
//! This crate aggregates the public APIs of the sub-crates. The [`bms`] and
//! [`bmson`] modules keep only the whole-pipeline entry points and
//! facade-owned types; [`raw`] mirrors the workspace directory structure with
//! the full layer exports.

//! # Quick start
//!
//! Parse a BMS chart, process it into the unified [`chart::Chart`], and
//! simulate playback with the [`player::Player`]:
//!
//! ```
//! use std::time::Duration;
//!
//! use bmsrs::prelude::*;
//!
//! let source = "#TITLE Demo\n#BPM 120\n#WAV01 kick.wav\n#00111:0100\n";
//! let parsed = bmsrs::bms::parse(source).expect("valid BMS");
//! let processed = bmsrs::bms::process(&parsed.value).expect("valid chart");
//!
//! let mut player = Player::new(processed.value.chart);
//! player.advance(Duration::from_secs(1));
//! assert_eq!(player.current_tick(), 480);
//! ```
//!
//! Parse a BMSON chart; v0/v1 inputs are upgraded to the root v2 schema:
//!
//! ```
//! let json = r#"{
//!     "version": "2.0.0",
//!     "song_info": {"title": "T", "artist": "A", "genre": "G"},
//!     "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
//!     "chart_data": {"init_bpm": 120}
//! }"#;
//! let bmson = bmsrs::bmson::parse_str(json).expect("valid BMSON");
//! assert_eq!(bmson.song_info.title, "T");
//! ```

/// Parsing and processing for the BMS format.
pub mod bms {
    // Layer crates live under `raw`; the curated path keeps only the entry
    // points and facade-owned types.
    use crate::raw::{model, parser, processor, source, syntax, values, writer};

    pub use parser::ErrorStrategy;

    /// Error type for the whole-pipeline entry points.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
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
        /// File read error.
        #[error("failed to read file: {0}")]
        Io(#[from] std::io::Error),
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

    /// Default control-flow branch selector for [`parse_with`] and
    /// [`parse_bytes_with`]: always evaluates to the first branch, regardless
    /// of the seed.
    #[must_use]
    pub const fn first_branch(_seed: u64) -> u64 {
        1
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
            first_branch,
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
        let document = source::Document::from_text(text);
        from_document(&document, options, rng, error_strategy)
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
        parse_bytes_with(
            bytes,
            &source::DocumentOptions::default(),
            &syntax::ParseOptions::default(),
            first_branch,
            ErrorStrategy::CollectAll,
        )
    }

    /// Parse BMS bytes with explicit decoding and syntax parse options.
    ///
    /// # Errors
    ///
    /// Returns [`FromTextError`] when `FailFast` encounters an error.
    pub fn parse_bytes_with(
        bytes: Vec<u8>,
        doc_options: &source::DocumentOptions,
        parse_options: &syntax::ParseOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, FromTextWarning>, FromTextError> {
        let document = source::Document::from_bytes_with(bytes, doc_options);
        from_document(&document, parse_options, rng, error_strategy)
            .map(|(bms, warnings)| bmsrs_pipeline::Outcome::new(bms, warnings))
    }

    /// Read the BMS file at `path` and parse it with the default pipeline.
    ///
    /// This is [`parse_bytes`] with file I/O: the file is read with
    /// [`std::fs::read`] and handed to the same pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`FromTextError`] when reading fails or when parsing or
    /// decoding fails.
    pub fn parse_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, FromTextWarning>, FromTextError> {
        let bytes = std::fs::read(path)?;
        parse_bytes(bytes)
    }

    /// Convert `Bms` to a format-agnostic chart with the default layout.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process(
        bms: &model::Bms,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::BmsProcessor::process_default(bms)
    }

    /// Convert `Bms` to a chart with an explicit layout family.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with<L: processor::layout::BmsLayout>(
        bms: &model::Bms,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::BmsProcessor::process::<L>(bms)
    }

    /// Write the full `Bms` model as canonical BMS text.
    #[must_use]
    pub fn write(bms: &model::Bms) -> String {
        writer::BmsWriter::write(bms)
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
    // The unified diagnostic exit lives in `bmsrs-pipeline`; each layer keeps
    // its own warning type and converts at this boundary.
    impl From<FromTextWarning> for bmsrs_pipeline::Diagnostic {
        fn from(warning: FromTextWarning) -> Self {
            match warning {
                FromTextWarning::Syntax { range, message } => Self::new(
                    bmsrs_pipeline::Severity::Warning,
                    Some(text_span(range)),
                    message,
                    Some(bmsrs_pipeline::DiagnosticCode::Syntax),
                ),
                FromTextWarning::Tokenize { range, error } => Self::new(
                    bmsrs_pipeline::Severity::Warning,
                    Some(text_span(range)),
                    error.to_string(),
                    Some(bmsrs_pipeline::DiagnosticCode::Tokenize),
                ),
                FromTextWarning::Decode { range, error } => Self::new(
                    bmsrs_pipeline::Severity::Warning,
                    Some(text_span(range)),
                    error.to_string(),
                    Some(bmsrs_pipeline::DiagnosticCode::Decode),
                ),
            }
        }
    }

    /// Convert a rowan text range into a format-neutral source span.
    fn text_span(range: syntax::TextRange) -> bmsrs_pipeline::SourceSpan {
        bmsrs_pipeline::SourceSpan::new(range.start().into(), range.end().into())
    }
}

/// Type definitions and converters for the BMSON format.
pub mod bmson {
    // Bmson layer types resolve through `raw`; the curated path keeps only the
    // entry points and facade-owned types.
    use crate::raw::bmson_model as model;
    use crate::raw::bmson_processor as processor;

    /// Parse a BMSON JSON string, upgrading v0/v1 to the root v2 schema.
    ///
    /// # Errors
    ///
    /// Returns [`BmsonParseError`] when the JSON or version is invalid.
    #[cfg(feature = "bmson")]
    pub fn parse_str(json: &str) -> Result<model::Bmson, BmsonParseError> {
        match model::DetectedVersion::detect(json)? {
            model::DetectedVersion::V2 => Ok(serde_json::from_str(json)?),
            model::DetectedVersion::V1 => {
                let v1: model::v1::Bmson = serde_json::from_str(json)?;
                Ok(model::Bmson::from(v1))
            }
            model::DetectedVersion::V0 => {
                let v0: model::v0::Bmson = serde_json::from_str(json)?;
                v0.try_into().map_err(|error: model::v0::TryFromV0Error| {
                    BmsonParseError::Version(model::BmsonError::V0Conversion(error))
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
    pub fn parse_slice(bytes: &[u8]) -> Result<model::Bmson, BmsonParseError> {
        let json = std::str::from_utf8(bytes)?;
        parse_str(json)
    }

    /// Read the BMSON file at `path` and parse it, upgrading v0/v1 to the
    /// root v2 schema.
    ///
    /// This is [`parse_slice`] with file I/O: the file is read with
    /// [`std::fs::read`] and handed to the same pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`BmsonParseError`] when reading fails or when the JSON,
    /// UTF-8, or version is invalid.
    #[cfg(feature = "bmson")]
    pub fn parse_file(path: impl AsRef<std::path::Path>) -> Result<model::Bmson, BmsonParseError> {
        let bytes = std::fs::read(path)?;
        parse_slice(&bytes)
    }

    /// Convert `Bmson` to a format-agnostic chart with the default layout.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process(
        bmson: &model::Bmson,
    ) -> Result<
        bmsrs_pipeline::Outcome<
            crate::chart::Chart<processor::BmsonNoteExt>,
            processor::BmsonProcessWarning,
        >,
        processor::ProcessError,
    > {
        processor::BmsonProcessor::process_default(bmson)
    }

    /// Convert `Bmson` with an explicit layout family.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with<L: processor::layout::BmsonLayout>(
        bmson: &model::Bmson,
    ) -> Result<
        bmsrs_pipeline::Outcome<
            crate::chart::Chart<processor::BmsonNoteExt>,
            processor::BmsonProcessWarning,
        >,
        processor::ProcessError,
    > {
        processor::BmsonProcessor::process::<L>(bmson)
    }

    /// Convert `Bmson` with an explicit generic key count.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_nkeys(
        bmson: &model::Bmson,
        keys: u8,
    ) -> Result<
        bmsrs_pipeline::Outcome<
            crate::chart::Chart<processor::BmsonNoteExt>,
            processor::BmsonProcessWarning,
        >,
        processor::ProcessError,
    > {
        processor::BmsonProcessor::process_nkeys(bmson, keys)
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
        Version(#[from] model::BmsonError),
        /// File read error.
        #[error("failed to read file: {0}")]
        Io(#[from] std::io::Error),
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
///
/// Covers the default-layout surface end to end: parsing, processing through
/// the default family, simulation, and the core chart types. Non-default mode
/// families go through the full [`processor`](crate::raw::processor) paths.
pub mod prelude {
    pub use crate::bms::process_with;
    pub use crate::bms::{FromTextError, FromTextWarning, parse, parse_bytes, process, write};
    #[cfg(feature = "bmson")]
    pub use crate::bmson::BmsonParseError;
    pub use crate::bmson::process_nkeys;
    pub use crate::chart::{
        Chart, ChartData, Event, EventKind, Lane, Note, NoteKind, NoteRole, TimingTrack,
    };
    pub use crate::pipeline::{Diagnostic, Outcome, Severity, SourceSpan};
    pub use crate::player::Player;
    pub use crate::raw::bmson_model::Bmson;
    pub use crate::raw::model::Bms;
    pub use crate::raw::processor::layout::{Bme, BmsLayout};
}

pub use bmsrs_pipeline::{Diagnostic, Outcome, Severity, SourceSpan};
