//! 从 rowan CST 或无损文档构建 `Bms`。

use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;

use bmsrs_bms_source::{Document, TextDecodeError};
use bmsrs_bms_syntax::{BranchSelection, LineKind, ParseOptions, SourceFile, TextRange, TextSize};
use bmsrs_bms_values::{BmsToken, BmsTokenizeError, parse_header_line, parse_message_line};

use crate::Bms;

/// token 抽取的错误处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// 收集全部行的错误。
    #[default]
    CollectAll,
    /// 遇到首条错误时返回。
    FailFast,
}

/// CST/文档解析产生的警告类别。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CstWarningKind {
    /// 单行值解析错误。
    Tokenize(BmsTokenizeError),
    /// 文档解码错误。
    Decode(TextDecodeError),
    /// CTS 语法/控制流诊断。
    Syntax(String),
}

/// 一条带行号的解析警告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstWarning {
    /// 行号（1 起）。
    pub line: NonZeroUsize,
    /// 文本范围。
    pub range: TextRange,
    /// 警告类别。
    pub kind: CstWarningKind,
}

/// 从 CST 构建 `Bms` 的结果。
#[derive(Debug, Clone)]
pub struct CstParse {
    /// 语义模型。
    pub bms: Bms,
    /// 控制流选择记录。
    pub selection: BranchSelection,
    /// 警告。
    pub warnings: Vec<CstWarning>,
}

/// 一行的 typed token 结果。
#[derive(Debug, Clone, PartialEq)]
pub struct LineToken {
    /// 行号（1 起）。
    pub line: NonZeroUsize,
    /// 整行文本范围。
    pub range: TextRange,
    /// typed token 或单行值错误。
    pub token: Result<BmsToken, BmsTokenizeError>,
}

impl Bms {
    /// 从 CST 选择分支并构建 `Bms`。
    ///
    /// `rng(max)` 返回 `[1, max]` 内的值；`#SETRANDOM` / `#SETSWITCH`
    /// 不调用它。
    #[must_use]
    pub fn from_cst(file: &SourceFile, rng: impl FnMut(u64) -> u64) -> CstParse {
        Self::from_cst_with(file, &ParseOptions::default(), rng)
    }

    /// 使用解析选项从 CST 选择分支并构建 `Bms`。
    #[must_use]
    pub fn from_cst_with(
        file: &SourceFile,
        options: &ParseOptions,
        rng: impl FnMut(u64) -> u64,
    ) -> CstParse {
        let all_tokens = tokenize_lines_with(file, options);
        let (bms, selection, selected_ranges) = build_selected(file, &all_tokens, rng);
        let warnings = token_warnings(&all_tokens)
            .into_iter()
            .filter(|warning| selected_ranges.contains(&warning.range))
            .collect();
        CstParse {
            bms,
            selection,
            warnings,
        }
    }

    /// 从无损文档构建 `Bms`，并收集所有行的警告。
    ///
    /// # Errors
    ///
    /// 文档解码失败，或 `FailFast` 遇到首条警告时返回 [`CstWarning`]。
    pub fn from_document(
        document: &Document,
        rng: impl FnMut(u64) -> u64,
        strategy: ErrorStrategy,
    ) -> Result<CstParse, CstWarning> {
        Self::from_document_with(document, &ParseOptions::default(), rng, strategy)
    }

    /// 使用解析选项从无损文档构建 `Bms`。
    ///
    /// # Errors
    ///
    /// 文档解码失败，或 `FailFast` 遇到首条警告时返回 [`CstWarning`]。
    pub fn from_document_with(
        document: &Document,
        options: &ParseOptions,
        rng: impl FnMut(u64) -> u64,
        strategy: ErrorStrategy,
    ) -> Result<CstParse, CstWarning> {
        let text = document.try_to_text().map_err(|error| CstWarning {
            line: NonZeroUsize::MIN,
            range: TextRange::empty(TextSize::from(0)),
            kind: CstWarningKind::Decode(error),
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
        Ok(CstParse {
            bms,
            selection,
            warnings,
        })
    }
}

/// 用整份 token 列表构建选中分支。
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

/// 收集 CST 语法诊断。
fn syntax_warnings(parse: &bmsrs_bms_syntax::Parse, file: &SourceFile) -> Vec<CstWarning> {
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
            CstWarning {
                line,
                range: error.range(),
                kind: CstWarningKind::Syntax(error.message().to_owned()),
            }
        })
        .collect()
}

/// 收集单行 token 错误。
fn token_warnings(tokens: &[LineToken]) -> Vec<CstWarning> {
    tokens
        .iter()
        .filter_map(|line| {
            line.token.as_ref().err().map(|error| CstWarning {
                line: line.line,
                range: line.range,
                kind: CstWarningKind::Tokenize(error.clone()),
            })
        })
        .collect()
}

/// 对 CST 的所有 Header/Message 行做 typed token 抽取。
#[must_use]
pub fn tokenize_lines(file: &SourceFile) -> Vec<LineToken> {
    tokenize_lines_with(file, &ParseOptions::default())
}

/// 使用解析选项对 CST 的所有 Header/Message 行做 typed token 抽取。
#[must_use]
pub fn tokenize_lines_with(file: &SourceFile, options: &ParseOptions) -> Vec<LineToken> {
    file.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if !matches!(line.kind(), LineKind::Header | LineKind::Message) {
                return None;
            }
            let content = line.content_text()?;
            Some(LineToken {
                line: NonZeroUsize::new(index.saturating_add(1)).unwrap_or(NonZeroUsize::MIN),
                range: line.range(),
                token: tokenize_line_with(&content, &options.header_prefixes),
            })
        })
        .collect()
}

/// 把一行内容解析为 token。
fn tokenize_line_with(content: &str, prefixes: &[char]) -> Result<BmsToken, BmsTokenizeError> {
    if let Some(message) = parse_message_line(content)? {
        return Ok(BmsToken::Message(message));
    }
    parse_header_line(content, prefixes)?.map_or_else(
        || {
            Err(BmsTokenizeError::OutOfRange {
                context: "line",
                value: content.to_owned(),
                expected: "header or message",
            })
        },
        |header| Ok(BmsToken::Header(header)),
    )
}
