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
//! let parsed = bmsrs::bms::parse(Input::text(source, SourceFormat::Bms)).expect("valid BMS");
//! let processed = bmsrs::bms::process(&parsed.value).expect("valid chart");
//!
//! let mut player = Player::new(processed.value.chart);
//! player.advance(Duration::from_secs(1));
//! assert_eq!(player.current_tick(), 480);
//! ```
//!
mod input;

/// Parsing and processing for the BMS format.
pub mod bms {
    // Layer crates live under `raw`; the curated path keeps only the entry
    // points and facade-owned types.
    use crate::raw::bms::{model, parser, processor, source, syntax, values, writer};

    pub use bmsrs_pipeline::Outcome;
    pub use model::Bms;
    pub use parser::{Diagnostic, DiagnosticKind};
    pub use processor::layout::{Bme, Layout};
    pub use processor::{AssetLookup, ProcessError, ProcessWarning, Processed};

    /// Every BMS mode family: the [`Layout`] trait
    /// and its standard implementations.
    pub mod layout {
        pub use crate::raw::bms::processor::layout::*;
    }
    pub use crate::input::Input;
    pub use bmsrs_pipeline::SourceFormat;
    pub use source::TextDecodeError;
    pub use syntax::TextRange;
    pub use syntax::{BranchMismatch, BranchSelection, SyntaxOptions};
    pub use values::BmsTokenizeError;

    pub use syntax::{
        ControlFlowCompat, ControlFlowRng, FirstBranch, JavaRandom, LR2_SEED_MAX, Lr2Mt19937,
    };

    /// Error type for the whole-pipeline entry points.
    ///
    /// Every variant means no chart value was produced. Recoverable parse
    /// diagnostics travel in [`Outcome`] instead.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum ParseError {
        /// The input bytes are not text in the detected or declared encoding.
        #[error(transparent)]
        Decode(#[from] source::TextDecodeError),
        /// The file's control-flow blocks diverged from the recording.
        #[error(transparent)]
        Branch(#[from] syntax::BranchMismatch),
        /// The input declares a format the BMS pipeline does not parse.
        ///
        /// A `.bmson` path reaches this check because [`Input::file`] derives
        /// the format from the extension; the BMS parser would otherwise
        /// decode the JSON as an empty chart without a single warning.
        #[error("the BMS entry points do not parse {format:?} input")]
        Format {
            /// The rejected source format.
            format: SourceFormat,
        },
        /// File read error.
        #[error("failed to read file: {0}")]
        Io(#[from] std::io::Error),
    }

    /// Parse BMS input with defaults: first branch, collecting every
    /// diagnostic in the outcome.
    ///
    /// `input` is an [`Input`]; a `&Path` converts to a file input, so
    /// `parse(path)` reads the file and derives the format from its extension.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when reading, decoding, or parsing fails.
    pub fn parse<'a>(
        input: impl Into<Input<'a>>,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, Diagnostic>, ParseError> {
        let source = input.into();
        let parse_options = with_input_format(source.format(), &syntax::SyntaxOptions::default())?;
        let document = source.into_document()?;
        from_document(&document, &parse_options, &mut syntax::FirstBranch)
            .map(|(bms, diagnostics)| bmsrs_pipeline::Outcome::new(bms, diagnostics))
    }

    /// Parse BMS input with explicit options and a control-flow generator.
    ///
    /// The input decides the source format and overrides `options.format`: a
    /// file derives it from the extension, text and bytes carry it explicitly.
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
    /// Returns [`ParseError::Decode`] when the input cannot be decoded,
    /// [`ParseError::Format`] for a non-BMS format, and [`ParseError::Io`] when
    /// a file cannot be read.
    pub fn parse_with<'a>(
        input: impl Into<Input<'a>>,
        options: &syntax::SyntaxOptions,
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, Diagnostic>, ParseError> {
        let source = input.into();
        let parse_options = with_input_format(source.format(), options)?;
        let document = source.into_document()?;
        from_document(&document, &parse_options, rng)
            .map(|(bms, diagnostics)| bmsrs_pipeline::Outcome::new(bms, diagnostics))
    }

    /// Parse BMS input, returning the recorded branch decisions alongside.
    ///
    /// Feed the recording to [`parse_from_selection`] with the same options
    /// to reproduce this exact branch layout later, or persist the decisions
    /// for replay tooling.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Decode`] when the input cannot be decoded,
    /// [`ParseError::Format`] for a non-BMS format, and [`ParseError::Io`] when
    /// a file cannot be read.
    pub fn parse_with_recording<'a>(
        input: impl Into<Input<'a>>,
        options: &syntax::SyntaxOptions,
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<
        (
            bmsrs_pipeline::Outcome<model::Bms, Diagnostic>,
            syntax::BranchSelection,
        ),
        ParseError,
    > {
        let source = input.into();
        let parse_options = with_input_format(source.format(), options)?;
        let document = source.into_document()?;
        let (bms, diagnostics, selection) =
            parse_document_selected(&document, &parse_options, rng)?;
        Ok((bmsrs_pipeline::Outcome::new(bms, diagnostics), selection))
    }

    /// Parse BMS input by replaying a recorded [`BranchSelection`], under the
    /// options the recording was taken with.
    ///
    /// The replay is strict: when the input's control-flow blocks diverge from
    /// the recording (count, kind, or declared bound), the call fails with
    /// [`ParseError::Branch`] naming the first divergent block.
    /// `#SETRANDOM` / `#SETSWITCH` values are taken from the current input.
    ///
    /// Build the recording with [`parse_with_recording`]; or drive the
    /// generator from [`JavaRandom`] to reproduce a beatoraja / `LR2oraja`
    /// branch layout from its seed.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Branch`] when the recording diverges,
    /// [`ParseError::Decode`] when the input cannot be decoded,
    /// [`ParseError::Format`] for a non-BMS format, and [`ParseError::Io`] when
    /// the input cannot be read.
    pub fn parse_from_selection<'a>(
        input: impl Into<Input<'a>>,
        selection: &syntax::BranchSelection,
        options: &syntax::SyntaxOptions,
    ) -> Result<bmsrs_pipeline::Outcome<model::Bms, Diagnostic>, ParseError> {
        let source = input.into();
        let parse_options = with_input_format(source.format(), options)?;
        let document = source.into_document()?;
        let text = document.try_to_text().map_err(ParseError::Decode)?;
        let parse = syntax::SourceFile::parse_with(&text, &parse_options);
        let file = parse.source_file();
        let mut diagnostics = syntax_diagnostics(&parse, &file);
        let parsed = parser::BmsParser::parse_from_selection_with(&file, selection, &parse_options)
            .map_err(ParseError::from)?;
        diagnostics.extend(parsed.warnings);
        diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.range.start()));
        Ok(bmsrs_pipeline::Outcome::new(parsed.bms, diagnostics))
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
        processor::Processor::process_default(bms)
    }

    /// Convert `Bms` to a chart with an explicit layout family.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with(
        bms: &model::Bms,
        layout: impl processor::layout::Layout,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::Processor::process_with(bms, layout)
    }

    /// Convert `Bms` to a chart with an explicit layout and asset policy.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with_assets(
        bms: &model::Bms,
        layout: impl processor::layout::Layout,
        assets: processor::AssetLookup<'_>,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::Processor::process_with_assets(bms, layout, assets)
    }

    /// Write the full `Bms` model as canonical BMS text.
    #[must_use]
    pub fn write(bms: &model::Bms) -> String {
        writer::Writer::write(bms)
    }

    /// Run the full semantic pipeline from an already-built document.
    fn from_document(
        document: &source::Document,
        options: &syntax::SyntaxOptions,
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<(model::Bms, Vec<Diagnostic>), ParseError> {
        let (bms, diagnostics, _selection) = parse_document_selected(document, options, rng)?;
        Ok((bms, diagnostics))
    }

    /// Parse a document and return its selected branch record.
    fn parse_document_selected(
        document: &source::Document,
        options: &syntax::SyntaxOptions,
        rng: &mut impl syntax::ControlFlowRng,
    ) -> Result<(model::Bms, Vec<Diagnostic>, syntax::BranchSelection), ParseError> {
        let text = document.try_to_text().map_err(ParseError::Decode)?;
        let parse = syntax::SourceFile::parse_with(&text, options);
        let file = parse.source_file();
        let all_tokens = parser::BmsParser::tokenize_lines_with(&file, options);
        let mut diagnostics = syntax_diagnostics(&parse, &file);
        diagnostics.extend(token_diagnostics(&all_tokens));
        let parsed = parser::BmsParser::parse_with(&file, options, rng);
        diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.range.start()));
        Ok((parsed.bms, diagnostics, parsed.selection))
    }

    /// Map CST diagnostics to facade diagnostics.
    fn syntax_diagnostics(
        parse: &syntax::SyntaxParse,
        file: &syntax::SourceFile,
    ) -> Vec<Diagnostic> {
        parse
            .errors()
            .iter()
            .map(|error| Diagnostic {
                line: file
                    .line_number_at(error.range().start())
                    .unwrap_or(std::num::NonZeroUsize::MIN),
                range: error.range(),
                severity: error.severity(),
                kind: DiagnosticKind::Cst {
                    message: error.message().to_owned(),
                    code: error.code(),
                },
            })
            .collect()
    }

    /// Map single-line token errors to facade diagnostics.
    fn token_diagnostics(tokens: &[parser::LineToken]) -> Vec<Diagnostic> {
        tokens
            .iter()
            .filter_map(|line| {
                line.token.as_ref().err().map(|error| Diagnostic {
                    line: line.line,
                    range: line.range,
                    severity: bmsrs_pipeline::Severity::Error,
                    kind: DiagnosticKind::Tokenize(error.clone()),
                })
            })
            .collect()
    }

    /// Apply the input's source format on top of the caller's options.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Format`] for [`SourceFormat::Bmson`]: the BMS
    /// pipeline parses the BMS text family only.
    fn with_input_format(
        format: SourceFormat,
        options: &syntax::SyntaxOptions,
    ) -> Result<syntax::SyntaxOptions, ParseError> {
        if format == SourceFormat::Bmson {
            return Err(ParseError::Format { format });
        }
        let mut parse_options = options.clone();
        parse_options.format = format;
        Ok(parse_options)
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
    use crate::raw::bmson::{model, processor};

    pub use crate::input::Input;
    pub use bmsrs_pipeline::{Outcome, SourceFormat};
    pub use model::Bmson;
    pub use processor::layout::{Beat, GenericLayout, Layout};
    pub use processor::{ProcessError, ProcessWarning, Processed};

    /// Every BMSON mode family: the [`Layout`] trait and its standard
    /// implementations, including
    /// [`GenericLayout`].
    pub mod layout {
        pub use crate::raw::bmson::processor::layout::*;
    }

    /// Parse BMSON input, upgrading v0/v1 to the root v2 schema.
    ///
    /// Every [`Input`] source is accepted: text and bytes must declare
    /// [`SourceFormat::Bmson`], and a path must end in `.bmson` or `.json`.
    /// Any other format fails with [`ParseError::Format`] before reading.
    /// The bytes are decoded as UTF-8 through the lossless
    /// [`Document`](bmsrs_bms_source::Document), which strips a UTF-8 BOM.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::Format`] for a non-BMSON format,
    /// [`ParseError::Io`] when a file cannot be read, [`ParseError::Decode`]
    /// when the bytes are not UTF-8, and the JSON or version variants as
    /// for [`parse_str`].
    pub fn parse<'a>(input: impl Into<Input<'a>>) -> Result<model::Bmson, ParseError> {
        let source = input.into();
        let format = source.format();
        if format != SourceFormat::Bmson {
            return Err(ParseError::Format { format });
        }
        // JSON is UTF-8 by specification; pinning the encoding keeps the
        // strictness of the former from_utf8 step instead of letting the
        // detector claim another encoding for invalid bytes.
        let document = source
            .with_encoding(bmsrs_bms_source::SourceEncoding::UTF8)
            .into_document()?;
        parse_str(&document.try_to_text()?)
    }

    /// Parse a BMSON JSON string, upgrading v0/v1 to the root v2 schema.
    ///
    /// The text shortcut of [`parse`].
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when the JSON or version is invalid.
    pub fn parse_str(json: &str) -> Result<model::Bmson, ParseError> {
        match model::DetectedVersion::detect(json)? {
            model::DetectedVersion::V2 => Ok(serde_json::from_str(json)?),
            model::DetectedVersion::V1 => {
                let v1: model::v1::Bmson = serde_json::from_str(json)?;
                v1.try_into().map_err(|error: model::UpgradeError| {
                    ParseError::Version(model::BmsonError::Upgrade(error))
                })
            }
            model::DetectedVersion::V0 => {
                let v0: model::v0::Bmson = serde_json::from_str(json)?;
                v0.try_into().map_err(|error: model::UpgradeError| {
                    ParseError::Version(model::BmsonError::Upgrade(error))
                })
            }
        }
    }

    /// Parse BMSON JSON bytes, upgrading v0/v1 to the root v2 schema.
    ///
    /// [`parse`] with the format declared in code.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when the bytes are not UTF-8 or when the JSON
    /// or version is invalid.
    pub fn parse_slice(bytes: &[u8]) -> Result<model::Bmson, ParseError> {
        parse(Input::bytes(bytes, SourceFormat::Bmson))
    }

    /// Read the BMSON file at `path` and parse it, upgrading v0/v1 to the
    /// root v2 schema.
    ///
    /// [`parse`] with [`Input::file`]; a `.bmson` or `.json` extension
    /// selects the format and any other extension fails with
    /// [`ParseError::Format`].
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when reading fails or when the bytes, JSON,
    /// or version is invalid.
    pub fn parse_file(path: impl AsRef<std::path::Path>) -> Result<model::Bmson, ParseError> {
        parse(Input::file(path.as_ref()))
    }

    /// Convert `Bmson` to a format-agnostic chart with the default layout.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process(
        bmson: &model::Bmson,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::Processor::process_default(bmson)
    }

    /// Convert `Bmson` with an explicit layout family.
    ///
    /// # Errors
    ///
    /// Returns [`processor::ProcessError`] when conversion fails.
    pub fn process_with(
        bmson: &model::Bmson,
        layout: impl processor::layout::Layout,
    ) -> Result<
        bmsrs_pipeline::Outcome<processor::Processed, processor::ProcessWarning>,
        processor::ProcessError,
    > {
        processor::Processor::process_with(bmson, layout)
    }

    /// BMSON parse error.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum ParseError {
        /// The input declares a format other than BMSON.
        #[error("the BMSON entry points do not parse {format:?} input")]
        Format {
            /// The rejected source format.
            format: SourceFormat,
        },
        /// JSON parse error.
        #[error("JSON parse error: {0}")]
        Json(#[from] serde_json::Error),
        /// The input is not UTF-8 text.
        #[error("BMSON input cannot be decoded: {0}")]
        Decode(#[from] bmsrs_bms_source::TextDecodeError),
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

/// Common diagnostic vocabulary for reporting across pipeline stages.
///
/// Typed diagnostics stay with their layers (`bms::Diagnostic`,
/// `bms::ProcessWarning`, `bmson::ProcessWarning`, and the analysis
/// diagnostic). Convert them to [`Diagnostic`](crate::diagnostics::Diagnostic)
/// when collecting one report:
///
/// ```
/// use bmsrs::bms::{Input, parse};
/// use bmsrs::diagnostics::Diagnostic;
/// use bmsrs::pipeline::SourceFormat;
///
/// let source = "#TITLE A\n#BPM abc\n";
/// let parsed = parse(Input::text(source, SourceFormat::Bms)).expect("valid BMS");
/// let report: Vec<Diagnostic> = parsed.diagnostics.iter().map(Diagnostic::from).collect();
/// assert_eq!(report.len(), 1);
/// ```
pub mod diagnostics {
    pub use bmsrs_pipeline::{Diagnostic, DiagnosticCode, HasSeverity, Severity, SourceSpan};
}

/// Pure simulation and playback layer.
pub mod player {
    pub use bmsrs_player::*;
}

/// Low-level crates for advanced and editor integrations.
///
/// The modules mirror the workspace directory tree: `raw::bms::*` maps to
/// `bms/` and `raw::bmson::*` to `bmson/`.
pub mod raw {
    /// The BMS pipeline crates, one module per crate under `bms/`.
    pub mod bms {
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
    }

    /// The BMSON pipeline crates, one module per crate under `bmson/`.
    #[cfg(feature = "bmson")]
    pub mod bmson {
        /// BMSON model.
        pub mod model {
            pub use bmsrs_bmson_model::*;
        }

        /// BMSON processor.
        pub mod processor {
            pub use bmsrs_bmson_processor::*;
        }
    }
}

/// Common imports for application code.
///
/// Types and traits only: what the curated [`bms`] and
/// [`bmson`] modules export, plus the chart, pipeline, and
/// player types those signatures name. Functions stay behind their module
/// (`bmsrs::bms::parse`, `bmsrs::bmson::parse_str`), so a glob import never
/// puts a bare `parse` or `write` in scope. `tests/facade_api.rs` names every
/// item, so dropping one fails to compile. When two modules export the same
/// name, the prelude keeps the `bms` one: `Diagnostic` is the parser wrapper
/// returned by [`bms::parse`], and the shared [`pipeline::Diagnostic`] stays
/// behind `pipeline::`; the bmson `Processed`, `ProcessWarning`, `ParseError`,
/// and `Layout` stay behind [`bmson`].
pub mod prelude {
    pub use crate::bms::layout::Bme;
    pub use crate::bms::{
        AssetLookup, Bms, BmsTokenizeError, BranchMismatch, BranchSelection, ControlFlowCompat,
        ControlFlowRng, Diagnostic, DiagnosticKind, FirstBranch, Input, JavaRandom, Layout,
        Lr2Mt19937, Outcome, ParseError, ProcessError, ProcessWarning, Processed, SourceFormat,
        SyntaxOptions, TextDecodeError, TextRange,
    };
    #[cfg(feature = "bmson")]
    pub use crate::bmson::layout::Beat;
    #[cfg(feature = "bmson")]
    pub use crate::bmson::{Bmson, GenericLayout};
    pub use crate::chart::{
        Chart, ChartData, Event, EventKind, Lane, Note, NoteKind, NoteRole, TimingTrack,
    };
    pub use crate::pipeline::{DiagnosticCode, Severity, SourceSpan};
    pub use crate::player::Player;
}
