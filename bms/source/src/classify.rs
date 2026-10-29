//! Inline comment state machine and line classification.
//!
//! Scanning runs on the raw bytes, not on decoded text. In every supported
//! ASCII-compatible encoding, comment markers and line terminators (`/`, `*`,
//! `;`, `"`, `#`, `:`, `CR`, `LF`) fall in the ASCII range and never appear in
//! the trailing byte of a multi-byte character (trailing bytes start at `0x40`
//! in `Shift_JIS` / `GBK` / `Big5` and at `0xA1` in `EUC-KR` / `EUC-JP`).
//! UTF-16 is scanned as 2-byte code units; a code unit below `0x80` is
//! equivalent to one ASCII byte.

use std::ops::Range;

use crate::{LineError, LineKind, SourceEncoding};

/// String and block-comment state preserved across lines.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScanState {
    /// Whether a `/* ... */` block comment is open.
    in_block: bool,
    /// Whether a `"..."` string is open.
    in_string: bool,
}

impl ScanState {
    /// Scan one line of text, update the cross-line state, and return the
    /// content span.
    pub fn scan(&mut self, text: &str) -> Scanned {
        scan_line(text.as_bytes(), None, self)
    }
}

/// Result of scanning one line.
pub struct Scanned {
    /// Content span range, relative to the line start and excluding the
    /// terminator.
    pub content: Option<Range<usize>>,
}

/// Scan one line, update the cross-line state, and return the content span.
pub fn scan_line(bytes: &[u8], utf16: Option<bool>, state: &mut ScanState) -> Scanned {
    let mut units = Units::new(bytes, utf16).peekable();

    let mut content_start: Option<usize> = None;
    let mut content_end: Option<usize> = None;
    // Freeze the content span at the first comment after content begins;
    // everything after it is suffix trivia.
    let mut content_closed = false;
    // `;` opens a whole-line comment only at line start (leading whitespace is
    // allowed).
    let mut seen_non_whitespace = state.in_block || state.in_string;
    let mut previous_whitespace = true;

    while let Some(unit) = units.next() {
        if state.in_block {
            if unit.ascii == Some(b'*')
                && matches!(units.peek(), Some(next) if next.ascii == Some(b'/'))
            {
                let _consumed: Option<Unit> = units.next();
                state.in_block = false;
            }
            previous_whitespace = false;
            continue;
        }

        if state.in_string {
            if unit.ascii == Some(b'\\') {
                if let Some(escaped) = units.next()
                    && !content_closed
                {
                    record(
                        &mut content_start,
                        &mut content_end,
                        unit.start,
                        escaped.end,
                    );
                }
            } else {
                if unit.ascii == Some(b'"') {
                    state.in_string = false;
                }
                if !content_closed {
                    record(&mut content_start, &mut content_end, unit.start, unit.end);
                }
            }
            previous_whitespace = false;
            continue;
        }

        match unit.ascii {
            Some(b'/')
                if previous_whitespace
                    && matches!(units.peek(), Some(next) if next.ascii == Some(b'/')) =>
            {
                // Line comment: the rest of this line is trivia.
                break;
            }
            Some(b'/') if matches!(units.peek(), Some(next) if next.ascii == Some(b'*')) => {
                let _consumed: Option<Unit> = units.next();
                state.in_block = true;
                seen_non_whitespace = true;
                if content_start.is_some() {
                    content_closed = true;
                }
                previous_whitespace = false;
            }
            Some(b';') if !seen_non_whitespace => {
                // `;` is a comment only at line start; elsewhere it is an
                // ordinary character.
                break;
            }
            Some(b'"') => {
                state.in_string = true;
                seen_non_whitespace = true;
                if !content_closed {
                    record(&mut content_start, &mut content_end, unit.start, unit.end);
                }
                previous_whitespace = false;
            }
            Some(b' ' | b'\t') => {
                previous_whitespace = true;
            }
            _ => {
                seen_non_whitespace = true;
                if !content_closed {
                    record(&mut content_start, &mut content_end, unit.start, unit.end);
                }
                previous_whitespace = false;
            }
        }
    }

    Scanned {
        content: content_start
            .zip(content_end)
            .map(|(start, end)| start..end),
    }
}

/// Extend the content range.
fn record(start: &mut Option<usize>, end: &mut Option<usize>, unit_start: usize, unit_end: usize) {
    start.get_or_insert(unit_start);
    *end = Some(unit_end);
}

/// Classify a line from the scan result and the raw text.
pub fn classify_line(body: &[u8], scanned: &Scanned, encoding: SourceEncoding) -> LineKind {
    let (text, had_errors) = encoding.decode(body);
    if had_errors {
        return LineKind::Error(LineError::Decode {
            encoding: encoding.name(),
        });
    }

    let Some(content) = &scanned.content else {
        return if text.trim().is_empty() {
            LineKind::Blank
        } else {
            LineKind::Comment
        };
    };

    if body.get(content.clone()).is_none() {
        return LineKind::Error(LineError::Decode {
            encoding: encoding.name(),
        });
    }

    LineKind::Text
}

/// Classify UTF-8 text (used for new lines created by edits).
pub fn classify_str(text: &str, scanned: &Scanned) -> LineKind {
    let Some(content) = &scanned.content else {
        return if text.trim().is_empty() {
            LineKind::Blank
        } else {
            LineKind::Comment
        };
    };
    if text.get(content.clone()).is_none() {
        return LineKind::Text;
    }

    LineKind::Text
}

/// Iterator over raw bytes or UTF-16 code units.
struct Units<'a> {
    /// Line bytes.
    bytes: &'a [u8],
    /// UTF-16 endianness; `None` means single-byte units.
    utf16: Option<bool>,
    /// Current offset.
    offset: usize,
}

impl<'a> Units<'a> {
    /// Create the iterator.
    const fn new(bytes: &'a [u8], utf16: Option<bool>) -> Self {
        Self {
            bytes,
            utf16,
            offset: 0,
        }
    }
}

impl Iterator for Units<'_> {
    type Item = Unit;

    fn next(&mut self) -> Option<Unit> {
        let start = self.offset;
        if let Some(little_endian) = self.utf16 {
            let first = *self.bytes.get(start)?;
            let low = self.bytes.get(start + 1).copied();
            let unit = match low {
                Some(byte) if little_endian => u16::from_le_bytes([first, byte]),
                Some(byte) => u16::from_be_bytes([first, byte]),
                None => u16::from(first),
            };
            let len = if low.is_none() {
                1
            } else if is_high_surrogate(unit) && self.bytes.get(start + 3).is_some() {
                4
            } else {
                2
            };
            self.offset = start + len;
            Some(Unit {
                ascii: ascii_of(unit),
                start,
                end: start + len,
            })
        } else {
            let byte = *self.bytes.get(start)?;
            self.offset = start + 1;
            Some(Unit {
                ascii: (byte < 0x80).then_some(byte),
                start,
                end: start + 1,
            })
        }
    }
}

/// A scanned unit.
struct Unit {
    /// ASCII value when below `0x80`.
    ascii: Option<u8>,
    /// Start byte offset.
    start: usize,
    /// End byte offset.
    end: usize,
}

/// Map a UTF-16 code unit to an ASCII value.
fn ascii_of(unit: u16) -> Option<u8> {
    u8::try_from(unit).ok().filter(|byte: &u8| byte.is_ascii())
}

/// Whether the unit is a UTF-16 high surrogate.
const fn is_high_surrogate(unit: u16) -> bool {
    unit >= 0xD800 && unit <= 0xDBFF
}
