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
    if let Some(little_endian) = encoding.utf16_little_endian() {
        return split_utf16(bytes, little_endian);
    }
    if let Some(big_endian) = encoding.utf32_big_endian() {
        return split_utf32(bytes, big_endian);
    }
    split_bytes(bytes)
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

/// Line splitting for UTF-32: scan 4-byte code units, so the split is exact
/// for well-formed files (no misaligned terminators).
fn split_utf32(bytes: &[u8], big_endian: bool) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while index + 3 < bytes.len() {
        let unit = read_unit32(bytes, index, big_endian);
        match unit {
            0x0000_000A => {
                spans.push(LineSpan {
                    range: start..index + 4,
                    body: start..index,
                    terminator: LineEnding::Lf,
                });
                start = index + 4;
                index += 4;
            }
            0x0000_000D => {
                if index + 7 < bytes.len()
                    && read_unit32(bytes, index + 4, big_endian) == 0x0000_000A
                {
                    spans.push(LineSpan {
                        range: start..index + 8,
                        body: start..index,
                        terminator: LineEnding::CrLf,
                    });
                    start = index + 8;
                    index += 8;
                } else {
                    spans.push(LineSpan {
                        range: start..index + 4,
                        body: start..index,
                        terminator: LineEnding::Cr,
                    });
                    start = index + 4;
                    index += 4;
                }
            }
            _ => index += 4,
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

/// Read a 4-byte code unit.
fn read_unit32(bytes: &[u8], index: usize, big_endian: bool) -> u32 {
    let mut buffer = [0u8; 4];
    for (offset, slot) in buffer.iter_mut().enumerate() {
        *slot = bytes.get(index + offset).copied().unwrap_or(0);
    }
    if big_endian {
        u32::from_be_bytes(buffer)
    } else {
        u32::from_le_bytes(buffer)
    }
}
