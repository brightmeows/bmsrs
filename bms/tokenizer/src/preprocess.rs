//! BMS comment-stripping pre-processor.
//!
//! This module provides a lexer-level pass that removes BMS comment syntax
//! before tokenization.  It handles the three comment forms specified in
//! the BMS control-flow document:
//!
//! - `//` — single-line comment (strips from marker to end-of-line).
//! - `;` — single-line comment, **line-start only** (after optional whitespace).
//! - `/* ... */` — block comment (non-nesting, may span multiple lines).
//!
//! Comment markers inside double-quoted strings (`"..."`) are preserved,
//! matching the `IIDXv` / HDX string-escape behavior.

/// State for the character-by-character comment scanner.
enum State {
    /// Outside any comment or string.
    Normal,
    /// Inside a `//` single-line comment.
    LineComment,
    /// Inside a `/*` block comment.
    BlockComment,
    /// After seeing `*` inside a block comment (potential `*/`).
    BlockMaybeEnd,
    /// Inside a `"..."` quoted string — comment markers are literal.
    InString(u8),
}

/// Strip BMS comments from source text, returning an owned [`String`].
///
/// # What is removed
///
/// | Syntax | Scope | Example |
/// |--------|-------|---------|
/// | `//` to EOL | Anywhere on a line | `#TITLE foo // comment` => `#TITLE foo ` |
/// | `;` to EOL | Line-start only (after trim) | `; debug` => _(whole line removed)_ |
/// | `/* ... */` | Multi-line, non-nesting | `/* block */#TITLE x` => `#TITLE x` |
///
/// # String escaping
///
/// Comment markers inside double-quoted strings (`"..."`) are treated as
/// ordinary characters, matching the `IIDXv` / HDX string-escape convention.
///
/// Backslash-prefixed characters inside strings are treated as escapes
/// (e.g. `\"` produces a literal quote, `\\` produces a literal backslash).
///
/// # Usage
///
/// Call this **before** tokenization, then tokenize with `C = String`:
///
/// ```ignore
/// let cleaned = bms_tokenizer::preprocess(raw_bms);
/// let tokens: Vec<(_, _)> = BmsTokenizer::new()
///     .tokenize::<Vec<_>, String>(&cleaned);
/// ```
#[must_use]
#[expect(clippy::indexing_slicing, reason = "guard checks ensure bounds")]
pub fn preprocess(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut state = State::Normal;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        match &mut state {
            State::Normal => match b {
                b'"' => {
                    out.push('"');
                    state = State::InString(0);
                    i += 1;
                }
                b';' if at_line_start(bytes, i) => {
                    state = State::LineComment;
                    i += 1;
                }
                b'/' if i + 1 < bytes.len() => match bytes[i + 1] {
                    b'/' => {
                        state = State::LineComment;
                        i += 2;
                    }
                    b'*' => {
                        state = State::BlockComment;
                        i += 2;
                    }
                    _ => {
                        out.push('/');
                        i += 1;
                    }
                },
                _ => {
                    out.push(b as char);
                    i += 1;
                }
            },

            State::LineComment => {
                if b == b'\n' || b == b'\r' {
                    out.push(b as char);
                    state = State::Normal;
                }
                i += 1;
            }

            State::BlockComment => {
                if b == b'*' {
                    state = State::BlockMaybeEnd;
                }
                i += 1;
            }

            State::BlockMaybeEnd => match b {
                b'/' => {
                    state = State::Normal;
                    i += 1;
                }
                b'*' => {
                    i += 1;
                }
                _ => {
                    state = State::BlockComment;
                    i += 1;
                }
            },

            State::InString(escape) => {
                if *escape == 0 && b == b'\\' {
                    out.push(b as char);
                    i += 1;
                    if let Some(&next) = bytes.get(i) {
                        out.push(next as char);
                        i += 1;
                    }
                } else if *escape == 0 && b == b'"' {
                    out.push('"');
                    state = State::Normal;
                    i += 1;
                } else {
                    out.push(b as char);
                    i += 1;
                }
            }
        }
    }

    out
}

/// Check if position `i` in `bytes` is at the start of a line (or after
/// leading whitespace only), for `;` comment detection.
fn at_line_start(bytes: &[u8], mut i: usize) -> bool {
    loop {
        i = match i.checked_sub(1) {
            None => return true,
            Some(prev) => match bytes.get(prev) {
                Some(b' ' | b'\t') => prev,
                Some(b'\n' | b'\r') => return true,
                _ => return false,
            },
        };
    }
}
