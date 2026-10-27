//! 行导向的 BMS 词法：切行、识别 trivia 与行内容。

use std::ops::Range;

use bms_source::ScanState;

use crate::SyntaxKind;

/// 单个词法 token。
#[derive(Debug, Clone, Copy)]
pub struct LexToken<'a> {
    /// token 种类。
    pub kind: SyntaxKind,
    /// token 文本。
    pub text: &'a str,
    /// 在文档中的起始字节偏移。
    pub start: usize,
}

impl LexToken<'_> {
    /// 返回 token 的结束偏移。
    pub(crate) const fn end(&self) -> usize {
        self.start.saturating_add(self.text.len())
    }
}

/// 行的语法分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// 空白行。
    Blank,
    /// 纯注释行。
    Comment,
    /// 头部命令行。
    Header,
    /// 通道数据行。
    Message,
    /// 普通文本行。
    Text,
    /// 解析失败的行。
    Error,
}

/// 一行及其 token。
#[derive(Debug, Clone)]
pub struct LineData<'a> {
    /// 语法分类。
    pub kind: LineKind,
    /// token 列表。
    pub tokens: Vec<LexToken<'a>>,
    /// 行起始字节偏移（含前缀 trivia）。
    pub start: usize,
    /// 行结束字节偏移（含行终止符）。
    pub end: usize,
}

impl LineData<'_> {
    /// 返回头部命令名（若存在）。
    pub(crate) fn command_name(&self) -> Option<&str> {
        self.tokens
            .iter()
            .find(|token| token.kind == SyntaxKind::CommandName)
            .map(|token| token.text)
    }
}

/// 原始行片段。
#[derive(Debug, Clone, Copy)]
struct RawLine<'a> {
    /// 不含行终止符的内容。
    body: &'a str,
    /// 行终止符。
    newline: &'a str,
    /// 行起始偏移。
    start: usize,
}

/// 把文本切成行，保留行终止符。
fn split_lines(text: &str) -> Vec<RawLine<'_>> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes.get(index) {
            Some(b'\n') => {
                lines.push(RawLine {
                    body: text.get(start..index).unwrap_or_default(),
                    newline: text.get(index..index + 1).unwrap_or_default(),
                    start,
                });
                index += 1;
                start = index;
            }
            Some(b'\r') => {
                let end = if bytes.get(index + 1) == Some(&b'\n') {
                    index + 2
                } else {
                    index + 1
                };
                lines.push(RawLine {
                    body: text.get(start..index).unwrap_or_default(),
                    newline: text.get(index..end).unwrap_or_default(),
                    start,
                });
                index = end;
                start = end;
            }
            _ => index += 1,
        }
    }

    if start < text.len() {
        lines.push(RawLine {
            body: text.get(start..).unwrap_or_default(),
            newline: "",
            start,
        });
    }

    lines
}

/// 逐行词法分析。
pub fn lex(text: &str) -> Vec<LineData<'_>> {
    let mut state = ScanState::default();
    split_lines(text)
        .into_iter()
        .map(|line| lex_line(line, &mut state))
        .collect()
}

/// 词法分析一行。
fn lex_line<'a>(line: RawLine<'a>, state: &mut ScanState) -> LineData<'a> {
    let scanned = state.scan(line.body);
    let mut tokens = Vec::new();

    let kind = if let Some(content) = scanned.content {
        push_trivia(line.body, 0..content.start, line.start, &mut tokens);
        let absolute = line.start.saturating_add(content.start);
        let content_text = line.body.get(content.clone()).unwrap_or_default();
        let kind = lex_content(content_text, absolute, &mut tokens);
        push_trivia(
            line.body,
            content.end..line.body.len(),
            line.start,
            &mut tokens,
        );
        kind
    } else {
        push_trivia(line.body, 0..line.body.len(), line.start, &mut tokens);
        if line.body.trim().is_empty() {
            LineKind::Blank
        } else {
            LineKind::Comment
        }
    };

    if !line.newline.is_empty() {
        tokens.push(LexToken {
            kind: SyntaxKind::Newline,
            text: line.newline,
            start: line.start.saturating_add(line.body.len()),
        });
    }

    LineData {
        kind,
        tokens,
        start: line.start,
        end: line
            .start
            .saturating_add(line.body.len())
            .saturating_add(line.newline.len()),
    }
}

/// 把内容段解析为 token。
fn lex_content<'a>(text: &'a str, absolute: usize, tokens: &mut Vec<LexToken<'a>>) -> LineKind {
    if text.starts_with("##") {
        push_token(tokens, SyntaxKind::Comment, text, absolute);
        return LineKind::Comment;
    }

    let Some(prefix) = text.chars().next() else {
        return LineKind::Blank;
    };
    if prefix != '#' && prefix != '%' {
        push_token(tokens, SyntaxKind::Text, text, absolute);
        return LineKind::Text;
    }

    let prefix_kind = if prefix == '#' {
        SyntaxKind::Hash
    } else {
        SyntaxKind::Percent
    };

    if let Some(body_start) = message_body_start(text) {
        push_token(
            tokens,
            prefix_kind,
            text.get(..1).unwrap_or_default(),
            absolute,
        );
        push_token(
            tokens,
            SyntaxKind::Measure,
            text.get(1..4).unwrap_or_default(),
            absolute.saturating_add(1),
        );
        push_token(
            tokens,
            SyntaxKind::Channel,
            text.get(4..6).unwrap_or_default(),
            absolute.saturating_add(4),
        );
        push_token(
            tokens,
            SyntaxKind::Colon,
            text.get(6..7).unwrap_or_default(),
            absolute.saturating_add(6),
        );
        push_body(
            text.get(body_start..).unwrap_or_default(),
            absolute.saturating_add(body_start),
            tokens,
        );
        return LineKind::Message;
    }

    lex_header(text, absolute, prefix_kind, tokens)
}

/// 判断是否是 `#xxxYY:...` 通道行，返回 body 起始偏移。
fn message_body_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.len() < 7 || bytes.first() != Some(&b'#') || bytes.get(6) != Some(&b':') {
        return None;
    }
    if !bytes.get(1..4)?.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if !bytes.get(4..6)?.iter().all(u8::is_ascii_alphanumeric) {
        return None;
    }
    Some(7)
}

/// 词法分析头部行。
fn lex_header<'a>(
    text: &'a str,
    absolute: usize,
    prefix_kind: SyntaxKind,
    tokens: &mut Vec<LexToken<'a>>,
) -> LineKind {
    push_token(
        tokens,
        prefix_kind,
        text.get(..1).unwrap_or_default(),
        absolute,
    );
    let rest = text.get(1..).unwrap_or_default();
    let rest_start = absolute.saturating_add(1);

    let command_end = rest
        .char_indices()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index))
        .unwrap_or(rest.len());
    let command = rest.get(..command_end).unwrap_or_default();
    if command.is_empty() {
        push_token(tokens, SyntaxKind::ErrorToken, rest, rest_start);
        return LineKind::Error;
    }
    push_token(tokens, SyntaxKind::CommandName, command, rest_start);

    let after_command = rest.get(command_end..).unwrap_or_default();
    let after_start = rest_start.saturating_add(command_end);
    let value_start = after_command
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_whitespace()).then_some(index));
    match value_start {
        Some(value_offset) => {
            push_token(
                tokens,
                SyntaxKind::Whitespace,
                after_command.get(..value_offset).unwrap_or_default(),
                after_start,
            );
            push_token(
                tokens,
                SyntaxKind::Value,
                after_command.get(value_offset..).unwrap_or_default(),
                after_start.saturating_add(value_offset),
            );
        }
        None => {
            push_token(tokens, SyntaxKind::Whitespace, after_command, after_start);
        }
    }

    LineKind::Header
}

/// 把通道体拆成两字符 chunk。
fn push_body<'a>(body: &'a str, absolute: usize, tokens: &mut Vec<LexToken<'a>>) {
    let mut boundaries: Vec<usize> = body.char_indices().map(|(index, _)| index).collect();
    boundaries.push(body.len());

    let mut index = 0usize;
    while index + 1 < boundaries.len() {
        let start = boundaries.get(index).copied().unwrap_or_default();
        let end = boundaries.get(index + 2).copied().unwrap_or(body.len());
        let kind = if index + 2 < boundaries.len() {
            SyntaxKind::BodyChunk
        } else {
            SyntaxKind::ErrorToken
        };
        push_token(
            tokens,
            kind,
            body.get(start..end).unwrap_or_default(),
            absolute.saturating_add(start),
        );
        index += 2;
    }
}

/// 处理前缀/后缀 trivia。
fn push_trivia<'a>(
    body: &'a str,
    range: Range<usize>,
    line_start: usize,
    tokens: &mut Vec<LexToken<'a>>,
) {
    let Some(text) = body.get(range.clone()) else {
        return;
    };
    if text.is_empty() {
        return;
    }

    let mut index = 0usize;
    while index < text.len() {
        let rest = text.get(index..).unwrap_or_default();
        let absolute = line_start.saturating_add(range.start).saturating_add(index);

        if let Some(ch) = rest.chars().next()
            && ch.is_whitespace()
        {
            let end = rest
                .char_indices()
                .find_map(|(offset, current)| (!current.is_whitespace()).then_some(offset))
                .unwrap_or(rest.len());
            push_token(
                tokens,
                SyntaxKind::Whitespace,
                rest.get(..end).unwrap_or_default(),
                absolute,
            );
            index += end;
            continue;
        }

        let end = if rest.starts_with("//") || rest.starts_with(';') {
            rest.len()
        } else if rest.starts_with("/*") {
            rest.find("*/").map_or(rest.len(), |offset| offset + 2)
        } else {
            rest.char_indices()
                .find_map(|(offset, current)| current.is_whitespace().then_some(offset))
                .unwrap_or(rest.len())
        };
        push_token(
            tokens,
            SyntaxKind::Comment,
            rest.get(..end).unwrap_or_default(),
            absolute,
        );
        index += end;
    }
}

/// 追加一个 token。
fn push_token<'a>(tokens: &mut Vec<LexToken<'a>>, kind: SyntaxKind, text: &'a str, start: usize) {
    if text.is_empty() {
        return;
    }
    tokens.push(LexToken { kind, text, start });
}
