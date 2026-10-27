//! 行内注释状态机与行分类。
//!
//! 扫描在原始字节上进行，不在解码后的文本上。原因是所有受支持的
//! ASCII 兼容编码里，注释标记与行终止符（`/`、`*`、`;`、`"`、`#`、`:`、
//! `CR`、`LF`）都落在 ASCII 区，且不会出现在多字节字符的尾字节中
//! （`Shift_JIS` / `GBK` / `Big5` 的尾字节从 `0x40` 起，`EUC-KR` /
//! `EUC-JP` 从 `0xA1` 起）。UTF-16 按 2 字节代码单元扫描，代码单元
//! 小于 `0x80` 时等价于一个 ASCII 字节。

use std::ops::Range;

use crate::{LineError, LineKind, SourceEncoding};

/// 跨行保留的字符串 / 块注释状态。
#[derive(Debug, Clone, Copy, Default)]
pub struct ScanState {
    /// 是否处于 `/* ... */` 块注释中。
    pub in_block: bool,
    /// 是否处于 `"..."` 字符串中。
    pub in_string: bool,
}

/// 单行扫描结果。
pub struct Scanned {
    /// 内容段区间（相对行起点，不含终止符）。
    pub content: Option<Range<usize>>,
}

/// 扫描一行，更新跨行状态，返回内容段。
pub fn scan_line(bytes: &[u8], utf16: Option<bool>, state: &mut ScanState) -> Scanned {
    let mut units = Units::new(bytes, utf16).peekable();

    let mut content_start: Option<usize> = None;
    let mut content_end: Option<usize> = None;
    // 内容段在遇到内容之后的第一个注释时冻结，之后一律算后缀 trivia。
    let mut content_closed = false;
    // `;` 只在行首（允许前置空白）开启整行注释。
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
                // 行注释：本行剩余部分全部是 trivia。
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
                // `;` 只在行首构成注释；行中保留为普通字符。
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

/// 扩展内容区间。
fn record(start: &mut Option<usize>, end: &mut Option<usize>, unit_start: usize, unit_end: usize) {
    start.get_or_insert(unit_start);
    *end = Some(unit_end);
}

/// 根据扫描结果与原文分类一行。
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

/// 对 UTF-8 文本做分类（用于编辑后的新行）。
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

/// 原始字节或 UTF-16 代码单元的迭代器。
struct Units<'a> {
    /// 行字节。
    bytes: &'a [u8],
    /// UTF-16 端序；`None` 表示按单字节处理。
    utf16: Option<bool>,
    /// 当前偏移。
    offset: usize,
}

impl<'a> Units<'a> {
    /// 构造迭代器。
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

/// 扫描单元。
struct Unit {
    /// 小于 `0x80` 时的 ASCII 值。
    ascii: Option<u8>,
    /// 起始字节偏移。
    start: usize,
    /// 结束字节偏移。
    end: usize,
}

/// 将 UTF-16 代码单元映射为 ASCII 值。
fn ascii_of(unit: u16) -> Option<u8> {
    u8::try_from(unit).ok().filter(|byte: &u8| byte.is_ascii())
}

/// 是否是 UTF-16 高代理项。
const fn is_high_surrogate(unit: u16) -> bool {
    unit >= 0xD800 && unit <= 0xDBFF
}
