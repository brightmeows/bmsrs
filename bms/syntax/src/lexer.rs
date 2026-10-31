//! Line-oriented BMS lexing: splitting lines, recognizing trivia, and lexing
//! line content.

use std::ops::Range;

use bmsrs_bms_source::ScanState;

use crate::{ParseOptions, SyntaxKind};

/// A single lexical token.
#[derive(Debug, Clone, Copy)]
pub struct LexToken<'a> {
    /// Token kind.
    pub kind: SyntaxKind,
    /// Token text.
    pub text: &'a str,
    /// Start byte offset in the document.
    pub start: usize,
}

impl LexToken<'_> {
    /// Return the token's end offset.
    pub(crate) const fn end(&self) -> usize {
        self.start.saturating_add(self.text.len())
    }
}

/// Syntactic classification of a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// Blank line.
    Blank,
    /// Comment-only line.
    Comment,
    /// Header command line.
    Header,
    /// Channel data line.
    Message,
    /// Plain text line.
    Text,
    /// A line that failed to parse.
    Error,
}

/// A line and its tokens.
#[derive(Debug, Clone)]
pub struct LineData<'a> {
    /// Syntactic classification.
    pub kind: LineKind,
    /// Token list.
    pub tokens: Vec<LexToken<'a>>,
    /// Line start byte offset (including prefix trivia).
    pub start: usize,
    /// Line end byte offset (including the line terminator).
    pub end: usize,
}

impl LineData<'_> {
    /// Return the header command name, if present.
    pub(crate) fn command_name(&self) -> Option<&str> {
        self.tokens
            .iter()
            .find(|token| token.kind == SyntaxKind::CommandName)
            .map(|token| token.text)
    }
}

/// A raw line fragment.
#[derive(Debug, Clone, Copy)]
struct RawLine<'a> {
    /// Content without the line terminator.
    body: &'a str,
    /// Line terminator.
    newline: &'a str,
    /// Line start offset.
    start: usize,
}

/// Split text into lines, preserving terminators.
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

/// Lex line by line.
pub fn lex<'a>(text: &'a str, options: &ParseOptions) -> Vec<LineData<'a>> {
    let mut state = ScanState::default();
    split_lines(text)
        .into_iter()
        .map(|line| lex_line(line, &mut state, options))
        .collect()
}

/// Lex one line.
fn lex_line<'a>(line: RawLine<'a>, state: &mut ScanState, options: &ParseOptions) -> LineData<'a> {
    let scanned = state.scan(line.body);
    let mut tokens = Vec::new();

    let kind = if let Some(content) = scanned.content {
        push_trivia(line.body, 0..content.start, line.start, &mut tokens);
        let absolute = line.start.saturating_add(content.start);
        let content_text = line.body.get(content.clone()).unwrap_or_default();
        let kind = lex_content(content_text, absolute, options, &mut tokens);
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

/// Parse the content span into tokens.
fn lex_content<'a>(
    text: &'a str,
    absolute: usize,
    options: &ParseOptions,
    tokens: &mut Vec<LexToken<'a>>,
) -> LineKind {
    if text.starts_with("##") {
        push_token(tokens, SyntaxKind::Comment, text, absolute);
        return LineKind::Comment;
    }

    let Some(prefix) = text.chars().next() else {
        return LineKind::Blank;
    };
    if !options.header_prefixes.contains(&prefix) {
        push_token(tokens, SyntaxKind::Text, text, absolute);
        return LineKind::Text;
    }

    let (prefix_kind, prefix_len) = match prefix {
        '#' => (SyntaxKind::Hash, 1),
        '%' => (SyntaxKind::Percent, 1),
        other => (SyntaxKind::Prefix, other.len_utf8()),
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

    lex_header(text, absolute, prefix_kind, prefix_len, tokens)
}

/// Return the body start offset if this is a `#xxxYY:...` channel line.
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

/// Lex a header line.
fn lex_header<'a>(
    text: &'a str,
    absolute: usize,
    prefix_kind: SyntaxKind,
    prefix_len: usize,
    tokens: &mut Vec<LexToken<'a>>,
) -> LineKind {
    push_token(
        tokens,
        prefix_kind,
        text.get(..prefix_len).unwrap_or_default(),
        absolute,
    );
    let rest = text.get(prefix_len..).unwrap_or_default();
    let rest_start = absolute.saturating_add(prefix_len);

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

/// Split the channel body into two-character chunks.
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

/// Handle prefix and suffix trivia.
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

/// Append a token.
fn push_token<'a>(tokens: &mut Vec<LexToken<'a>>, kind: SyntaxKind, text: &'a str, start: usize) {
    if text.is_empty() {
        return;
    }
    tokens.push(LexToken { kind, text, start });
}
