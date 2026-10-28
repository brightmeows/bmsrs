//! 按编码把原始字节切成行。

use std::ops::Range;

use crate::{LineEnding, SourceEncoding};

/// 一行的原始字节区间。
pub struct LineSpan {
    /// 整行区间（含终止符）。
    pub range: Range<usize>,
    /// 去掉终止符后的区间。
    pub body: Range<usize>,
    /// 行尾终止符。
    pub terminator: LineEnding,
}

/// 按编码切分原始字节。
pub fn split_lines(bytes: &[u8], encoding: SourceEncoding) -> Vec<LineSpan> {
    encoding.utf16_little_endian().map_or_else(
        || split_bytes(bytes),
        |little_endian| split_utf16(bytes, little_endian),
    )
}

/// ASCII 兼容编码的行切分。
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

/// UTF-16 编码的行切分：按 2 字节代码单元扫描。
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

/// 读取 2 字节代码单元。
fn read_unit(bytes: &[u8], index: usize, little_endian: bool) -> u16 {
    let first = bytes.get(index).copied().unwrap_or(0);
    let second = bytes.get(index + 1).copied().unwrap_or(0);
    if little_endian {
        u16::from_le_bytes([first, second])
    } else {
        u16::from_be_bytes([first, second])
    }
}
