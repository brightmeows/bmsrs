//! Build `Bms` from a rowan CST or a lossless document.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::num::NonZeroUsize;

use bmsrs_bms_source::{Document, TextDecodeError};
use bmsrs_bms_syntax::{
    BranchMismatch, BranchSelection, ControlFlowRng, ParseOptions, SourceFile, SyntaxLineKind,
    TextRange, TextSize,
};
use bmsrs_bms_values::{BmsToken, BmsTokenizeError};
use bmsrs_pipeline::{DiagnosticCode, ErrorStrategy, Severity, SourceSpan};

use bmsrs_bms_model::Bms;

/// What a [`BmsDiagnostic`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmsDiagnosticKind {
    /// Single-line value parse error.
    Tokenize(BmsTokenizeError),
    /// Document decode error.
    Decode(TextDecodeError),
    /// CST syntax or control-flow diagnostic, with its stable code.
    Cst {
        /// Diagnostic message.
        message: String,
        /// Stable code assigned by the syntax layer.
        code: DiagnosticCode,
    },
}

/// One diagnostic from CST or document parsing: a recoverable warning, or the
/// error that stopped a `FailFast` parse.
///
/// Severity travels with the diagnostic rather than being implied by the
/// channel it arrives on: a `CollectAll` parse keeps error-severity
/// diagnostics in its warning list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsDiagnostic {
    /// One-based line number.
    pub line: NonZeroUsize,
    /// Text range.
    pub range: TextRange,
    /// Severity assigned by the layer that produced the diagnostic.
    pub severity: Severity,
    /// What the diagnostic reports.
    pub kind: BmsDiagnosticKind,
}

impl BmsDiagnostic {
    /// A whole-document decode failure: line 1 and an empty range, because the
    /// text never got far enough to have a position.
    #[must_use]
    pub fn decode(error: TextDecodeError) -> Self {
        Self {
            line: NonZeroUsize::MIN,
            range: TextRange::empty(TextSize::from(0)),
            severity: Severity::Error,
            kind: BmsDiagnosticKind::Decode(error),
        }
    }

    /// Return whether this diagnostic has error severity.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }
}

impl fmt::Display for BmsDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Information => "information",
        };
        match &self.kind {
            BmsDiagnosticKind::Cst { message, code } => {
                write!(
                    formatter,
                    "{code} {severity} at {:?}: {message}",
                    self.range
                )
            }
            BmsDiagnosticKind::Tokenize(error) => {
                write!(formatter, "tokenization error at {:?}: {error}", self.range)
            }
            BmsDiagnosticKind::Decode(error) => {
                write!(formatter, "decode error at {:?}: {error}", self.range)
            }
        }
    }
}

impl std::error::Error for BmsDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            BmsDiagnosticKind::Cst { .. } => None,
            BmsDiagnosticKind::Tokenize(error) => Some(error),
            BmsDiagnosticKind::Decode(error) => Some(error),
        }
    }
}

impl From<BmsDiagnostic> for bmsrs_pipeline::Diagnostic {
    fn from(diagnostic: BmsDiagnostic) -> Self {
        let (message, code) = match diagnostic.kind {
            BmsDiagnosticKind::Cst { message, code } => (message, code),
            BmsDiagnosticKind::Tokenize(error) => (error.to_string(), DiagnosticCode::Tokenize),
            BmsDiagnosticKind::Decode(error) => (error.to_string(), DiagnosticCode::Decode),
        };
        Self::new(diagnostic.severity, message)
            .with_span(SourceSpan::new(
                diagnostic.range.start().into(),
                diagnostic.range.end().into(),
            ))
            .with_code(code)
    }
}

/// Result of building `Bms` from a CST.
#[derive(Debug, Clone)]
pub struct BmsParse {
    /// Semantic model.
    pub bms: Bms,
    /// Control-flow selection record.
    pub selection: BranchSelection,
    /// Diagnostics collected while building the model.
    pub warnings: Vec<BmsDiagnostic>,
}

/// Typed token result for one line.
#[derive(Debug, Clone, PartialEq)]
pub struct LineToken {
    /// One-based line number.
    pub line: NonZeroUsize,
    /// Text range of the whole line.
    pub range: TextRange,
    /// Typed token or single-line value error.
    pub token: Result<BmsToken, BmsTokenizeError>,
}

/// BMS semantic parser.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BmsParser;

impl BmsParser {
    /// Select branches from the CST and build `Bms`.
    ///
    /// `rng.draw(max)` returns a value in `[1, max]`; `#SETRANDOM` / `#SETSWITCH`
    /// do not call it.
    ///
    /// Warnings cover selected lines only; errors in unselected branches do
    /// not surface. Use [`BmsParser::parse_document`] to collect warnings
    /// from every line.
    #[must_use]
    pub fn parse<R: ControlFlowRng + ?Sized>(file: &SourceFile, rng: &mut R) -> BmsParse {
        Self::parse_with(file, &ParseOptions::default(), rng)
    }

    /// Select branches from the CST with parse options and build `Bms`.
    ///
    /// `options.error_strategy` is not consulted: a CST carries no syntax
    /// diagnostics to fail on. Use [`Self::parse_document_with`] when
    /// `FailFast` must stop at the first error.
    #[must_use]
    pub fn parse_with<R: ControlFlowRng + ?Sized>(
        file: &SourceFile,
        options: &ParseOptions,
        rng: &mut R,
    ) -> BmsParse {
        let all_tokens = Self::tokenize_lines_with(file, options);
        let (bms, selection, selected_ranges) = build_selected(
            file,
            &all_tokens,
            rng,
            options.control_flow_compat,
            options.format,
        );
        let warnings = token_diagnostics(&all_tokens)
            .into_iter()
            .filter(|warning| selected_ranges.contains(&warning.range))
            .collect();
        BmsParse {
            bms,
            selection,
            warnings,
        }
    }

    /// Build `Bms` from a lossless document and collect diagnostics from
    /// every line.
    ///
    /// # Errors
    ///
    /// Returns the diagnostic when document decoding fails or, under
    /// `FailFast`, for the first diagnostic of error severity. Warning-severity
    /// diagnostics stay in the list.
    pub fn parse_document<R: ControlFlowRng + ?Sized>(
        document: &Document,
        rng: &mut R,
    ) -> Result<BmsParse, BmsDiagnostic> {
        Self::parse_document_with(document, &ParseOptions::default(), rng)
    }

    /// Build `Bms` from a lossless document with parse options.
    ///
    /// # Errors
    ///
    /// Returns the diagnostic when document decoding fails or, under
    /// `FailFast`, for the first diagnostic of error severity. Warning-severity
    /// diagnostics stay in the list.
    pub fn parse_document_with<R: ControlFlowRng + ?Sized>(
        document: &Document,
        options: &ParseOptions,
        rng: &mut R,
    ) -> Result<BmsParse, BmsDiagnostic> {
        let text = document.try_to_text().map_err(BmsDiagnostic::decode)?;
        let parse = SourceFile::parse_with(&text, options);
        let file = parse.source_file();
        let all_tokens = Self::tokenize_lines_with(&file, options);
        let mut warnings = syntax_diagnostics(&parse, &file);
        warnings.extend(token_diagnostics(&all_tokens));
        warnings.sort_by_key(|warning| (warning.line, warning.range.start()));

        if options.error_strategy == ErrorStrategy::FailFast
            && let Some(first_error) = warnings.iter().find(|warning| warning.is_error())
        {
            return Err(first_error.clone());
        }

        let (bms, selection, _selected_ranges) = build_selected(
            &file,
            &all_tokens,
            rng,
            options.control_flow_compat,
            options.format,
        );
        Ok(BmsParse {
            bms,
            selection,
            warnings,
        })
    }
}

/// Build the selected branches from the full token list.
fn build_selected<R: ControlFlowRng + ?Sized>(
    file: &SourceFile,
    all_tokens: &[LineToken],
    rng: &mut R,
    compat: bmsrs_bms_syntax::ControlFlowCompat,
    format: bmsrs_pipeline::SourceFormat,
) -> (Bms, BranchSelection, HashSet<TextRange>) {
    let mut token_map: HashMap<TextRange, BmsToken> = all_tokens
        .iter()
        .filter_map(|line| {
            line.token
                .as_ref()
                .ok()
                .map(|token| (line.range, token.clone()))
        })
        .collect();

    let (lines, selection) = file.select_lines(rng, compat);
    let mut tokens = Vec::new();
    let mut selected_ranges = HashSet::new();
    for line in &lines {
        if line.is_directive() {
            continue;
        }
        selected_ranges.insert(line.range());
        if let Some(token) = token_map.remove(&line.range()) {
            tokens.push(token);
        }
    }

    (
        Bms::from_flat_tokens_with(tokens, format),
        selection,
        selected_ranges,
    )
}

/// Collect CST diagnostics, copying the severity the syntax layer assigned.
fn syntax_diagnostics(
    parse: &bmsrs_bms_syntax::SyntaxParse,
    file: &SourceFile,
) -> Vec<BmsDiagnostic> {
    parse
        .errors()
        .iter()
        .map(|error| BmsDiagnostic {
            line: file
                .line_number_at(error.range().start())
                .unwrap_or(NonZeroUsize::MIN),
            range: error.range(),
            severity: error.severity(),
            kind: BmsDiagnosticKind::Cst {
                message: error.message().to_owned(),
                code: error.code(),
            },
        })
        .collect()
}

/// Collect single-line token errors.
fn token_diagnostics(tokens: &[LineToken]) -> Vec<BmsDiagnostic> {
    tokens
        .iter()
        .filter_map(|line| {
            line.token.as_ref().err().map(|error| BmsDiagnostic {
                line: line.line,
                range: line.range,
                severity: Severity::Error,
                kind: BmsDiagnosticKind::Tokenize(error.clone()),
            })
        })
        .collect()
}

impl BmsParser {
    /// Extract typed tokens from every Header/Message line in the CST.
    #[must_use]
    pub fn tokenize_lines(file: &SourceFile) -> Vec<LineToken> {
        Self::tokenize_lines_with(file, &ParseOptions::default())
    }

    /// Extract typed tokens from every Header/Message line in the CST with parse
    /// options.
    #[must_use]
    pub fn tokenize_lines_with(file: &SourceFile, options: &ParseOptions) -> Vec<LineToken> {
        file.lines()
            .enumerate()
            .filter_map(|(index, line)| {
                if !matches!(
                    line.kind(),
                    SyntaxLineKind::Header | SyntaxLineKind::Message
                ) {
                    return None;
                }
                let content = line.content_text()?;
                Some(LineToken {
                    line: NonZeroUsize::new(index.saturating_add(1)).unwrap_or(NonZeroUsize::MIN),
                    range: line.range(),
                    token: tokenize_line_with(
                        &content,
                        &options.header_prefixes,
                        line.range(),
                        options.format,
                    ),
                })
            })
            .collect()
    }
}

/// Parse one line of content into a token.
/// Strip a `.bmd` inline comment from channel data.
///
/// `KaleidBeat` extension: `.bmd` ignores spaces and `;` comments inside the
/// channel data area; standard BMS treats `;` as an ordinary character there.
#[expect(
    clippy::string_slice,
    reason = "BMS lines are ASCII; the colon and semicolon are byte boundaries"
)]
fn bmd_content(content: &str, format: bmsrs_pipeline::SourceFormat) -> std::borrow::Cow<'_, str> {
    if format != bmsrs_pipeline::SourceFormat::Bmd {
        return std::borrow::Cow::Borrowed(content);
    }
    let Some(colon) = content.find(':') else {
        return std::borrow::Cow::Borrowed(content);
    };
    content[colon..]
        .find(';')
        .map_or(std::borrow::Cow::Borrowed(content), |offset| {
            std::borrow::Cow::Owned(content[..colon + offset].trim_end().to_owned())
        })
}

/// Parse one line of content into a token, applying the source format.
fn tokenize_line_with(
    content: &str,
    prefixes: &[char],
    line_range: TextRange,
    format: bmsrs_pipeline::SourceFormat,
) -> Result<BmsToken, BmsTokenizeError> {
    let normalized = bmd_content(content, format);
    let Some(mut token) = BmsToken::parse_line(normalized.as_ref(), prefixes)? else {
        return Err(BmsTokenizeError::OutOfRange {
            context: "line",
            value: normalized.to_string(),
            expected: "header or message",
        });
    };
    if let BmsToken::Message(message) = &mut token {
        message.span = Some(SourceSpan::new(
            u32::from(line_range.start()),
            u32::from(line_range.end()),
        ));
    }
    Ok(token)
}

impl BmsParser {
    /// Build `Bms` from a recorded [`BranchSelection`].
    ///
    /// The replay is strict: when the file's control-flow blocks diverge from
    /// the recording (count, kind, or declared bound), [`BranchMismatch`]
    /// names the first divergent block. `#SETRANDOM` / `#SETSWITCH` values are
    /// taken from the current file. Warnings cover selected lines only,
    /// mirroring [`BmsParser::parse_with`].
    ///
    /// # Errors
    ///
    /// Returns [`BranchMismatch`] naming the first divergent block when the
    /// recording does not match the file's control-flow structure.
    pub fn parse_from_selection(
        file: &SourceFile,
        selection: &BranchSelection,
    ) -> Result<BmsParse, BranchMismatch> {
        Self::parse_from_selection_with(file, selection, &ParseOptions::default())
    }

    /// Build `Bms` from a recorded [`BranchSelection`] with parse options.
    ///
    /// `options.error_strategy` is not consulted here, for the same reason as
    /// [`Self::parse_with`]; the facade's replay entry point applies it after
    /// collecting the syntax diagnostics itself.
    ///
    /// # Errors
    ///
    /// Returns [`BranchMismatch`] naming the first divergent block when the
    /// recording does not match the file's control-flow structure.
    pub fn parse_from_selection_with(
        file: &SourceFile,
        selection: &BranchSelection,
        options: &ParseOptions,
    ) -> Result<BmsParse, BranchMismatch> {
        let all_tokens = Self::tokenize_lines_with(file, options);
        let (lines, replayed) = file.replay_lines(selection)?;
        let mut token_map: HashMap<TextRange, BmsToken> = all_tokens
            .iter()
            .filter_map(|line| {
                line.token
                    .as_ref()
                    .ok()
                    .map(|token| (line.range, token.clone()))
            })
            .collect();
        let mut tokens = Vec::new();
        let mut selected_ranges = HashSet::new();
        for line in &lines {
            if line.is_directive() {
                continue;
            }
            selected_ranges.insert(line.range());
            if let Some(token) = token_map.remove(&line.range()) {
                tokens.push(token);
            }
        }
        let warnings = token_diagnostics(&all_tokens)
            .into_iter()
            .filter(|warning| selected_ranges.contains(&warning.range))
            .collect();
        Ok(BmsParse {
            bms: Bms::from_flat_tokens_with(tokens, options.format),
            selection: replayed,
            warnings,
        })
    }
}
