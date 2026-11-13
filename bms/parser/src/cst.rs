//! Build `Bms` from a rowan CST or a lossless document.

use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;

use bmsrs_bms_source::{Document, TextDecodeError};
use bmsrs_bms_syntax::{
    BranchMismatch, BranchSelection, ParseOptions, SourceFile, SyntaxLineKind, TextRange, TextSize,
};
use bmsrs_bms_values::{BmsToken, BmsTokenizeError};
use bmsrs_pipeline::SourceSpan;

use bmsrs_bms_model::Bms;

/// Error-handling strategy for token extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// Collect errors from every line.
    #[default]
    CollectAll,
    /// Return at the first error.
    FailFast,
}

/// Warning categories produced by CST or document parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmsParseWarningKind {
    /// Single-line value parse error.
    Tokenize(BmsTokenizeError),
    /// Document decode error.
    Decode(TextDecodeError),
    /// CST syntax or control-flow diagnostic.
    Syntax(String),
}

/// A parse warning with a line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsParseWarning {
    /// One-based line number.
    pub line: NonZeroUsize,
    /// Text range.
    pub range: TextRange,
    /// Warning category.
    pub kind: BmsParseWarningKind,
}

/// Result of building `Bms` from a CST.
#[derive(Debug, Clone)]
pub struct BmsParse {
    /// Semantic model.
    pub bms: Bms,
    /// Control-flow selection record.
    pub selection: BranchSelection,
    /// Warnings.
    pub warnings: Vec<BmsParseWarning>,
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
    /// `rng(max)` returns a value in `[1, max]`; `#SETRANDOM` / `#SETSWITCH`
    /// do not call it.
    #[must_use]
    pub fn parse(file: &SourceFile, rng: impl FnMut(u64) -> u64) -> BmsParse {
        Self::parse_with(file, &ParseOptions::default(), rng)
    }

    /// Select branches from the CST with parse options and build `Bms`.
    #[must_use]
    pub fn parse_with(
        file: &SourceFile,
        options: &ParseOptions,
        rng: impl FnMut(u64) -> u64,
    ) -> BmsParse {
        let all_tokens = tokenize_lines_with(file, options);
        let (bms, selection, selected_ranges) = build_selected(file, &all_tokens, rng);
        let warnings = token_warnings(&all_tokens)
            .into_iter()
            .filter(|warning| selected_ranges.contains(&warning.range))
            .collect();
        BmsParse {
            bms,
            selection,
            warnings,
        }
    }

    /// Build `Bms` from a lossless document and collect warnings from every
    /// line.
    ///
    /// # Errors
    ///
    /// Returns [`BmsParseWarning`] when document decoding fails or when `FailFast`
    /// encounters the first warning.
    pub fn parse_document(
        document: &Document,
        rng: impl FnMut(u64) -> u64,
        strategy: ErrorStrategy,
    ) -> Result<BmsParse, BmsParseWarning> {
        Self::parse_document_with(document, &ParseOptions::default(), rng, strategy)
    }

    /// Build `Bms` from a lossless document with parse options.
    ///
    /// # Errors
    ///
    /// Returns [`BmsParseWarning`] when document decoding fails or when `FailFast`
    /// encounters the first warning.
    pub fn parse_document_with(
        document: &Document,
        options: &ParseOptions,
        rng: impl FnMut(u64) -> u64,
        strategy: ErrorStrategy,
    ) -> Result<BmsParse, BmsParseWarning> {
        let text = document.try_to_text().map_err(|error| BmsParseWarning {
            line: NonZeroUsize::MIN,
            range: TextRange::empty(TextSize::from(0)),
            kind: BmsParseWarningKind::Decode(error),
        })?;
        let parse = SourceFile::parse_with(&text, options);
        let file = parse.source_file();
        let all_tokens = tokenize_lines_with(&file, options);
        let mut warnings = syntax_warnings(&parse, &file);
        warnings.extend(token_warnings(&all_tokens));
        warnings.sort_by_key(|warning| (warning.line, warning.range.start()));

        if strategy == ErrorStrategy::FailFast
            && let Some(first) = warnings.first()
        {
            return Err(first.clone());
        }

        let (bms, selection, _selected_ranges) = build_selected(&file, &all_tokens, rng);
        Ok(BmsParse {
            bms,
            selection,
            warnings,
        })
    }
}

/// Build the selected branches from the full token list.
fn build_selected(
    file: &SourceFile,
    all_tokens: &[LineToken],
    rng: impl FnMut(u64) -> u64,
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

    let (lines, selection) = file.select_lines(rng);
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

    (Bms::from_flat_tokens(tokens), selection, selected_ranges)
}

/// Collect CST syntax diagnostics.
fn syntax_warnings(
    parse: &bmsrs_bms_syntax::SyntaxParse,
    file: &SourceFile,
) -> Vec<BmsParseWarning> {
    let lines: Vec<_> = file.lines().enumerate().collect();
    parse
        .errors()
        .iter()
        .map(|error| {
            let offset = error.range().start();
            let line = lines
                .iter()
                .find_map(|(index, line)| {
                    let range = line.range();
                    (range.start() <= offset && offset < range.end()).then(|| {
                        NonZeroUsize::new(index.saturating_add(1)).unwrap_or(NonZeroUsize::MIN)
                    })
                })
                .unwrap_or(NonZeroUsize::MIN);
            BmsParseWarning {
                line,
                range: error.range(),
                kind: BmsParseWarningKind::Syntax(error.message().to_owned()),
            }
        })
        .collect()
}

/// Collect single-line token errors.
fn token_warnings(tokens: &[LineToken]) -> Vec<BmsParseWarning> {
    tokens
        .iter()
        .filter_map(|line| {
            line.token.as_ref().err().map(|error| BmsParseWarning {
                line: line.line,
                range: line.range,
                kind: BmsParseWarningKind::Tokenize(error.clone()),
            })
        })
        .collect()
}

/// Extract typed tokens from every Header/Message line in the CST.
#[must_use]
pub fn tokenize_lines(file: &SourceFile) -> Vec<LineToken> {
    tokenize_lines_with(file, &ParseOptions::default())
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
                token: tokenize_line_with(&content, &options.header_prefixes, line.range()),
            })
        })
        .collect()
}

/// Parse one line of content into a token.
fn tokenize_line_with(
    content: &str,
    prefixes: &[char],
    line_range: TextRange,
) -> Result<BmsToken, BmsTokenizeError> {
    let Some(mut token) = BmsToken::parse_line(content, prefixes)? else {
        return Err(BmsTokenizeError::OutOfRange {
            context: "line",
            value: content.to_owned(),
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
    /// # Errors
    ///
    /// Returns [`BranchMismatch`] naming the first divergent block when the
    /// recording does not match the file's control-flow structure.
    pub fn parse_from_selection_with(
        file: &SourceFile,
        selection: &BranchSelection,
        options: &ParseOptions,
    ) -> Result<BmsParse, BranchMismatch> {
        let all_tokens = tokenize_lines_with(file, options);
        let (lines, replayed) = file.select_lines_with(selection)?;
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
        let warnings = token_warnings(&all_tokens)
            .into_iter()
            .filter(|warning| selected_ranges.contains(&warning.range))
            .collect();
        Ok(BmsParse {
            bms: Bms::from_flat_tokens(tokens),
            selection: replayed,
            warnings,
        })
    }
}
