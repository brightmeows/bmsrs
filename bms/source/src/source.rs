//! Split raw bytes into lines according to the encoding.

use std::ops::Range;

use crate::{LineEnding, SourceEncoding};

/// Byte range of a single line.
pub struct LineSpan {
    /// Full line range, including the terminator.
    pub range: Range<usize>,
    /// Range without the terminator.
    pub body: Range<usize>,
    /// Line terminator.
    pub terminator: LineEnding,
}

/// Split raw bytes according to the encoding.
pub fn split_lines(bytes: &[u8], encoding: SourceEncoding) -> Vec<LineSpan> {
    encoding.utf16_little_endian().map_or_else(
        || split_bytes(bytes),
        |little_endian| split_utf16(bytes, little_endian),
    )
}

/// Line splitting for ASCII-compatible encodings.
fn split_bytes(bytes: &[u8]) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'\n' => {
                spans.push(LineSpan {
                    range: start..index + 1,
                    body: start..index,
                    terminator: LineEnding::Lf,
                });
                start = index + 1;
                index += 1;
            }
            b'\r' => {
                if bytes.get(index + 1) == Some(&b'\n') {
                    spans.push(LineSpan {
                        range: start..index + 2,
                        body: start..index,
                        terminator: LineEnding::CrLf,
                    });
                    start = index + 2;
                    index += 2;
                } else {
                    spans.push(LineSpan {
                        range: start..index + 1,
                        body: start..index,
                        terminator: LineEnding::Cr,
                    });
                    start = index + 1;
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }

    if start < bytes.len() {
        spans.push(LineSpan {
            range: start..bytes.len(),
            body: start..bytes.len(),
            terminator: LineEnding::None,
        });
    }

    spans
}

/// Line splitting for UTF-16: scan 2-byte code units.
fn split_utf16(bytes: &[u8], little_endian: bool) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while index + 1 < bytes.len() {
        match read_unit(bytes, index, little_endian) {
            0x000A => {
                spans.push(LineSpan {
                    range: start..index + 2,
                    body: start..index,
                    terminator: LineEnding::Lf,
                });
                start = index + 2;
                index += 2;
            }
            0x000D => {
                if index + 3 < bytes.len() && read_unit(bytes, index + 2, little_endian) == 0x000A {
                    spans.push(LineSpan {
                        range: start..index + 4,
                        body: start..index,
                        terminator: LineEnding::CrLf,
                    });
                    start = index + 4;
                    index += 4;
                } else {
                    spans.push(LineSpan {
                        range: start..index + 2,
                        body: start..index,
                        terminator: LineEnding::Cr,
                    });
                    start = index + 2;
                    index += 2;
                }
            }
            _ => index += 2,
        }
    }

    if start < bytes.len() {
        spans.push(LineSpan {
            range: start..bytes.len(),
            body: start..bytes.len(),
            terminator: LineEnding::None,
        });
    }

    spans
}

/// Read a 2-byte code unit.
fn read_unit(bytes: &[u8], index: usize, little_endian: bool) -> u16 {
    let first = bytes.get(index).copied().unwrap_or(0);
    let second = bytes.get(index + 1).copied().unwrap_or(0);
    if little_endian {
        u16::from_le_bytes([first, second])
    } else {
        u16::from_be_bytes([first, second])
    }
}
