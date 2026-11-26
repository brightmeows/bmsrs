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
/// Parsing and processing for the BMS format.
pub mod bms {
    // Layer crates live under `raw`; the curated path keeps only the entry
    // points and facade-owned types.
    use crate::raw::{model, parser, processor, source, syntax, values, writer};

    pub use bmsrs_pipeline::{ErrorStrategy, Outcome};
    pub use model::Bms;
    pub use parser::{BmsDiagnostic, BmsDiagnosticKind};
    pub use processor::layout::{Bme, BmsLayout};
    pub use processor::{AssetLookup, ProcessError, ProcessWarning, Processed};

    /// Every BMS mode family: the [`BmsLayout`] trait
    /// and its standard implementations.
    pub mod layout {
        pub use crate::raw::processor::layout::*;
    }
    pub use source::DocumentOptions;
    pub use source::TextDecodeError;
    pub use syntax::TextRange;
    pub use syntax::{BranchMismatch, BranchSelection, ParseOptions};
    pub use values::BmsTokenizeError;

    pub use syntax::{
        ControlFlowCompat, ControlFlowRng, FirstBranch, JavaRandom, LR2_SEED_MAX, Lr2Mt19937,
    };

    /// Error type for the whole-pipeline entry points.
    ///
    /// The diagnostic variant is the error that stopped a `FailFast` parse or
    /// a document that failed to decode; the other two have no position in the
    /// text.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum ParseError {
        /// A decode, tokenization, or CST diagnostic of error severity.
        #[error(transparent)]
        Diagnostic(#[from] parser::BmsDiagnostic),
        /// The file's control-flow blocks diverged from the recording.
        #[error(transparent)]
        Branch(#[from] syntax::BranchMismatch),
        /// File read error.
        #[error("failed to read file: {0}")]
        Io(#[from] std::io::Error),
    }

    /// Parse BMS text with defaults: first branch and `CollectAll`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when parsing fails.
    pub fn parse(
        text: &str,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, BmsDiagnostic>, ParseError> {
        parse_with(
            text,
            &syntax::ParseOptions::default(),
            &mut syntax::FirstBranch,
        )
    }

    /// Parse BMS text with explicit options and a control-flow generator.
    ///
    /// The generator stays with the caller, so its remaining stream is
    /// available afterwards; LR2's `#RANDOM` lane option continues from the
    /// same stream. `rng.draw(max)` supplies one value per block in `[1, max]`;
    /// every `FnMut(u64) -> u64` closure implements
    /// [`ControlFlowRng`], so `&mut |_| 1` and
    /// `&mut JavaRandom::new(seed)` are both valid.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Diagnostic`] when `FailFast` meets the first
    /// diagnostic of error severity.
    pub fn parse_with(
        text: &str,
        options: &syntax::ParseOptions,
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, BmsDiagnostic>, ParseError> {
        let document = source::Document::from_text(text);
        from_document(&document, options, rng)
            .map(|(bms, warnings)| bmsrs_pipeline::Outcome::new(bms, warnings))
    }

    /// Parse BMS text by replaying a recorded [`BranchSelection`].
    ///
    /// The replay is strict: when the file's control-flow blocks diverge from
    /// the recording (count, kind, or declared bound), the call fails with
    /// [`ParseError::Branch`] naming the first divergent block.
    /// `#SETRANDOM` / `#SETSWITCH` values are taken from the current file.
    ///
    /// Build the recording with `parse` /
    /// `parse_with` and read the decisions through
    /// [`BranchSelection::decisions`](syntax::BranchSelection::decisions);
    /// or drive the recording from [`JavaRandom`] to reproduce a beatoraja /
    /// `LR2oraja` branch layout from its seed.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when decoding fails or the recording diverges
    /// from the file.
    pub fn parse_from_selection(
        text: &str,
        selection: &syntax::BranchSelection,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, BmsDiagnostic>, ParseError> {
        let document = source::Document::from_text(text);
        let src = document
            .try_to_text()
            .map_err(BmsDiagnostic::decode)
            .map_err(ParseError::from)?;
        let parse = syntax::SourceFile::parse_with(&src, &syntax::ParseOptions::default());
        let file = parse.source_file();
        let mut warnings: Vec<BmsDiagnostic> = parse
            .errors()
            .iter()
            .map(|error| BmsDiagnostic {
                line: file
                    .line_number_at(error.range().start())
                    .unwrap_or(std::num::NonZeroUsize::MIN),
                range: error.range(),
                severity: error.severity(),
                kind: BmsDiagnosticKind::Cst {
                    message: error.message().to_owned(),
                    code: error.code(),
                },
            })
            .collect();
        let parsed =
            parser::BmsParser::parse_from_selection(&file, selection).map_err(ParseError::from)?;
        warnings.extend(parsed.warnings);
        Ok(bmsrs_pipeline::Outcome::new(parsed.bms, warnings))
    }

    /// Parse BMS text, returning the recorded branch decisions alongside.
    ///
    /// Feed the recording to `parse_from_selection`
    /// to reproduce this exact branch layout later, or persist the decisions
    /// for replay tooling.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Diagnostic`] when `FailFast` meets the first
    /// diagnostic of error severity.
    pub fn parse_with_recording(
        text: &str,
        options: &syntax::ParseOptions,
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<
        (
            bmsrs_pipeline::Outcome<model::Bms, BmsDiagnostic>,
            syntax::BranchSelection,
        ),
        ParseError,
    > {
        let document = source::Document::from_text(text);
        let parsed = parser::BmsParser::parse_document_with(&document, options, rng)
            .map_err(ParseError::from)?;
        Ok((
            bmsrs_pipeline::Outcome::new(parsed.bms, parsed.warnings),
            parsed.selection,
        ))
    }

    /// Parse BMS bytes with defaults: first branch and `CollectAll`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when parsing or decoding fails.
    pub fn parse_bytes(
        bytes: Vec<u8>,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, BmsDiagnostic>, ParseError> {
        parse_bytes_with(
            bytes,
            &source::DocumentOptions::default(),
            &syntax::ParseOptions::default(),
            &mut syntax::FirstBranch,
        )
    }

    /// Parse BMS bytes with explicit decoding and syntax parse options.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Diagnostic`] when `FailFast` meets the first
    /// diagnostic of error severity.
    pub fn parse_bytes_with(
        bytes: Vec<u8>,
        doc_options: &source::DocumentOptions,
        parse_options: &syntax::ParseOptions,
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, BmsDiagnostic>, ParseError> {
        let document = source::Document::from_bytes_with(bytes, doc_options);
        from_document(&document, parse_options, rng)
            .map(|(bms, warnings)| bmsrs_pipeline::Outcome::new(bms, warnings))
    }

    /// Read the BMS file at `path` and parse it with the default pipeline.
    ///
    /// This is [`parse_bytes`] with file I/O: the file is read with
    /// [`std::fs::read`] and handed to the same pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when reading fails or when parsing or
    /// decoding fails.
    pub fn parse_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, BmsDiagnostic>, ParseError> {
        let file_path = path.as_ref();
        let options = syntax::ParseOptions {
            format: bmsrs_pipeline::SourceFormat::from_path(file_path),
            ..syntax::ParseOptions::default()
        };
        let bytes = std::fs::read(file_path)?;
        parse_bytes_with(
            bytes,
            &source::DocumentOptions::default(),
            &options,
            &mut syntax::FirstBranch,
        )
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
    pub fn process_with(
        bms: &model::Bms,
        layout: impl processor::layout::BmsLayout,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::BmsProcessor::process_with(bms, layout)
    }

    /// Convert `Bms` to a chart with an explicit layout and asset policy.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with_assets(
        bms: &model::Bms,
        layout: impl processor::layout::BmsLayout,
        assets: processor::AssetLookup<'_>,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::BmsProcessor::process_with_assets(bms, layout, assets)
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
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<(model::Bms, Vec<BmsDiagnostic>), ParseError> {
        let parsed = parser::BmsParser::parse_document_with(document, options, rng)
            .map_err(ParseError::from)?;
        Ok((parsed.bms, parsed.warnings))
    }
}

/// Type definitions and converters for the BMSON format.
///
/// Parse a BMSON chart; v0/v1 inputs are upgraded to the root v2 schema:
///
/// ```
/// let json = r#"{
///     "version": "2.0.0",
///     "song_info": {"title": "T", "artist": "A", "genre": "G"},
///     "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
///     "chart_data": {"init_bpm": 120}
/// }"#;
/// let bmson = bmsrs::bmson::parse_str(json).expect("valid BMSON");
/// assert_eq!(bmson.song_info.title, "T");
/// ```
#[cfg(feature = "bmson")]
pub mod bmson {
    // Bmson layer types resolve through `raw`; the curated path keeps only the
    // entry points and facade-owned types.
    use crate::raw::bmson_model as model;
    use crate::raw::bmson_processor as processor;

    pub use bmsrs_pipeline::Outcome;
    pub use model::Bmson;
    pub use processor::layout::{Beat, BmsonLayout};
    pub use processor::{BmsonProcessWarning, BmsonProcessed, ProcessError};

    /// Every BMSON mode family: the [`BmsonLayout`] trait and its standard
    /// implementations, including
    /// [`GenericLayout`](layout::GenericLayout).
    pub mod layout {
        pub use crate::raw::bmson_processor::layout::*;
    }

    /// Parse a BMSON JSON string, upgrading v0/v1 to the root v2 schema.
    ///
    /// # Errors
    ///
    /// Returns [`BmsonParseError`] when the JSON or version is invalid.
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
        bmsrs_pipeline::Outcome<processor::BmsonProcessed, processor::BmsonProcessWarning>,
        processor::ProcessError,
    > {
        processor::BmsonProcessor::process_default(bmson)
    }

    /// Convert `Bmson` with an explicit layout family.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with(
        bmson: &model::Bmson,
        layout: impl processor::layout::BmsonLayout,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::BmsonProcessed, processor::BmsonProcessWarning>,
        processor::ProcessError,
    > {
        processor::BmsonProcessor::process_with(bmson, layout)
    }

    /// BMSON parse error.
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
    #[cfg(feature = "bmson")]
    pub mod bmson_model {
        pub use bmsrs_bmson_model::*;
    }

    /// BMSON processor.
    #[cfg(feature = "bmson")]
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
    pub use crate::bms::{Bme, Bms, BmsLayout};
    pub use crate::bms::{
        BmsDiagnostic, ParseError, parse, parse_bytes, parse_with, process, write,
    };
    #[cfg(feature = "bmson")]
    pub use crate::bmson::Bmson;
    #[cfg(feature = "bmson")]
    pub use crate::bmson::BmsonParseError;
    pub use crate::chart::{
        Chart, ChartData, Event, EventKind, Lane, Note, NoteKind, NoteRole, TimingTrack,
    };
    pub use crate::pipeline::{Diagnostic, DiagnosticCode, Outcome, Severity, SourceSpan};
    pub use crate::player::Player;
}

pub use bmsrs_pipeline::{Diagnostic, DiagnosticCode, Outcome, Severity, SourceSpan};
