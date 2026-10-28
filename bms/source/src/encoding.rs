//! 源文本编码：校验、检测与按行编解码。

use std::borrow::Cow;

use encoding_rs::Encoding;

/// 可用于编辑与写回的源文本编码。
///
/// 构造时校验编码是否满足两个前提：
///
/// - **ASCII 兼容**（UTF-16 除外）：行终止符 `CR` / `LF` 不会出现在多字节
///   字符的尾字节中，因此可以按字节切行。
/// - **无状态**：单行可以独立重新编码，不必回放整篇转义状态。
///
/// `replacement` 编码不可解码，`ISO-2022-JP` 的转义序列跨行，两者都会被
/// 拒绝。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceEncoding(&'static Encoding);

impl SourceEncoding {
    /// UTF-8 的内部常量（构造必然成功）。
    pub(crate) const UTF8: Self = Self(encoding_rs::UTF_8);

    /// 校验并包装一个编码。
    ///
    /// # Errors
    ///
    /// 当编码是 `replacement`、`ISO-2022-JP`，或既非 ASCII 兼容又不是
    /// UTF-16 时返回 [`UnsupportedEncoding`]。
    pub fn new(encoding: &'static Encoding) -> Result<Self, UnsupportedEncoding> {
        if encoding == encoding_rs::REPLACEMENT || encoding == encoding_rs::ISO_2022_JP {
            return Err(UnsupportedEncoding {
                name: encoding.name(),
            });
        }
        let supported = encoding == encoding_rs::UTF_16LE
            || encoding == encoding_rs::UTF_16BE
            || encoding.is_ascii_compatible();
        if supported {
            Ok(Self(encoding))
        } else {
            Err(UnsupportedEncoding {
                name: encoding.name(),
            })
        }
    }

    /// 按 WHATWG 编码标签（例如 `#CHARSET` 的值）构造编码。
    #[must_use]
    pub fn from_label(label: &[u8]) -> Option<Self> {
        let encoding = Encoding::for_label_no_replacement(label)?;
        Self::new(encoding).ok()
    }

    /// 检测字节序列的编码。
    ///
    /// 检测顺序：
    ///
    /// 1. BOM
    /// 2. `#CHARSET` 声明
    /// 3. 严格 UTF-8
    /// 4. 候选表：`Shift_JIS`、`EUC-KR`、`GBK`、`Big5`
    ///
    /// `#CHARSET` 声明与实际字节不符时回退到候选表，并在
    /// [`DetectedEncoding::method`] 中记为
    /// [`DetectionMethod::DeclaredFallback`]。
    #[must_use]
    pub fn detect(bytes: &[u8]) -> DetectedEncoding {
        if let Some((bom_encoding, bom_len)) = Encoding::for_bom(bytes)
            && let Ok(encoding) = Self::new(bom_encoding)
        {
            return DetectedEncoding {
                encoding,
                bom_len,
                declared: None,
                method: DetectionMethod::Bom,
            };
        }

        let declared =
            scan_charset_header(bytes).and_then(|label| Self::from_label(label.as_bytes()));

        if let Some(encoding) = declared {
            if !encoding.decode(bytes).1 {
                return DetectedEncoding {
                    encoding,
                    bom_len: 0,
                    declared: Some(encoding),
                    method: DetectionMethod::Charset,
                };
            }
            return Self::detect_after_declared(bytes, encoding);
        }

        Self::detect_without_declared(bytes)
    }

    /// 声明不符时的回退检测。
    fn detect_after_declared(bytes: &[u8], declared: Self) -> DetectedEncoding {
        if std::str::from_utf8(bytes).is_ok() {
            return DetectedEncoding {
                encoding: Self::UTF8,
                bom_len: 0,
                declared: Some(declared),
                method: DetectionMethod::DeclaredFallback,
            };
        }
        if let Some(encoding) = Self::first_candidate(bytes) {
            return DetectedEncoding {
                encoding,
                bom_len: 0,
                declared: Some(declared),
                method: DetectionMethod::DeclaredFallback,
            };
        }
        DetectedEncoding {
            encoding: Self::UTF8,
            bom_len: 0,
            declared: Some(declared),
            method: DetectionMethod::Lossy,
        }
    }

    /// 无声明时的检测。
    fn detect_without_declared(bytes: &[u8]) -> DetectedEncoding {
        if std::str::from_utf8(bytes).is_ok() {
            return DetectedEncoding {
                encoding: Self::UTF8,
                bom_len: 0,
                declared: None,
                method: DetectionMethod::Utf8,
            };
        }
        if let Some(encoding) = Self::first_candidate(bytes) {
            return DetectedEncoding {
                encoding,
                bom_len: 0,
                declared: None,
                method: DetectionMethod::Heuristic,
            };
        }
        DetectedEncoding {
            encoding: Self::UTF8,
            bom_len: 0,
            declared: None,
            method: DetectionMethod::Lossy,
        }
    }

    /// 依次尝试候选编码，返回第一个能无损解码的。
    fn first_candidate(bytes: &[u8]) -> Option<Self> {
        let candidates = [
            encoding_rs::SHIFT_JIS,
            encoding_rs::EUC_KR,
            encoding_rs::GBK,
            encoding_rs::BIG5,
        ];
        candidates.into_iter().find_map(|candidate| {
            let encoding = Self::new(candidate).ok()?;
            if encoding.decode(bytes).1 {
                None
            } else {
                Some(encoding)
            }
        })
    }

    /// 返回底层编码。
    #[must_use]
    pub const fn encoding(self) -> &'static Encoding {
        self.0
    }

    /// 返回 WHATWG 规范名（例如 `"Shift_JIS"`）。
    #[must_use]
    pub fn name(self) -> &'static str {
        self.0.name()
    }

    /// 返回是否是 UTF-16LE / UTF-16BE。
    #[must_use]
    pub fn is_utf16(self) -> bool {
        self.0 == encoding_rs::UTF_16LE || self.0 == encoding_rs::UTF_16BE
    }

    /// 返回 UTF-16 的端序：`Some(true)` 为小端，`Some(false)` 为大端，
    /// 其他编码返回 `None`。
    #[must_use]
    pub fn utf16_little_endian(self) -> Option<bool> {
        if self.0 == encoding_rs::UTF_16LE {
            Some(true)
        } else if self.0 == encoding_rs::UTF_16BE {
            Some(false)
        } else {
            None
        }
    }

    /// 解码字节，返回 `(文本, 是否出现解码错误)`。
    ///
    /// 调用方负责先剥离 BOM。
    #[must_use]
    pub fn decode(self, bytes: &[u8]) -> (Cow<'_, str>, bool) {
        self.0.decode_without_bom_handling(bytes)
    }

    /// 将文本编码为字节。
    ///
    /// # Errors
    ///
    /// 当文本包含此编码无法表示的字符时返回 [`UnmappableCharacter`]。
    /// 不会像 [`Encoding::encode`] 那样把不可映射字符替换为数字字符引用。
    pub fn encode(self, text: &str) -> Result<Vec<u8>, UnmappableCharacter> {
        let (bytes, _, had_unmappable) = self.0.encode(text);
        if had_unmappable {
            Err(UnmappableCharacter {
                encoding: self.0.name(),
            })
        } else {
            Ok(bytes.into_owned())
        }
    }

    /// 返回文件开头属于此编码的 BOM 长度。
    pub(crate) fn bom_len(self, bytes: &[u8]) -> usize {
        match Encoding::for_bom(bytes) {
            Some((encoding, len)) if encoding == self.0 => len,
            _ => 0,
        }
    }
}

/// 编码检测结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectedEncoding {
    /// 最终采用的编码。
    pub encoding: SourceEncoding,
    /// BOM 的字节长度，`0` 表示无 BOM。
    pub bom_len: usize,
    /// 文件 `#CHARSET` 声明的编码（若有）。
    pub declared: Option<SourceEncoding>,
    /// 检测路径。
    pub method: DetectionMethod,
}

/// 编码的检测路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMethod {
    /// 由 BOM 确定。
    Bom,
    /// 由 `#CHARSET` 声明确定，且解码无错。
    Charset,
    /// 严格 UTF-8。
    Utf8,
    /// 候选表命中。
    Heuristic,
    /// `#CHARSET` 声明解码失败，回退到候选表。
    DeclaredFallback,
    /// 无候选通过，按 UTF-8 损失性解码。
    Lossy,
    /// 调用方显式指定。
    Explicit,
}

/// 编码不在支持范围内。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unsupported encoding: {name}")]
pub struct UnsupportedEncoding {
    /// 被拒绝的编码名。
    pub name: &'static str,
}

/// 文本包含目标编码无法表示的字符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("text cannot be represented in {encoding}")]
pub struct UnmappableCharacter {
    /// 目标编码名。
    pub encoding: &'static str,
}

/// 字节级扫描 `#CHARSET` 或 `%CHARSET` 行首指令。
///
/// 仅在行首位置匹配（`input[0]` 或前一个字节为 `\n` / `\r`），
/// 避免匹配注释或通道数据中的内容。
fn scan_charset_header(input: &[u8]) -> Option<String> {
    let mut i = 0;
    while let Some(&prefix_byte) = input.get(i) {
        let is_line_start = i == 0 || input.get(i - 1).is_some_and(|&b| b == b'\n' || b == b'\r');

        if !is_line_start || (prefix_byte != b'#' && prefix_byte != b'%') {
            i += 1;
            continue;
        }

        let Some(after_prefix) = input.get(i + 1..) else {
            break;
        };
        if !after_prefix
            .get(..7)
            .is_some_and(|s| s.eq_ignore_ascii_case(b"CHARSET"))
        {
            i += 1;
            continue;
        }

        let Some(after_keyword) = after_prefix.get(7..) else {
            i += 1;
            continue;
        };
        // 值为空行或全空白时跳过此行，继续扫描后续 #CHARSET 行。
        let Some(value_start) = after_keyword
            .iter()
            .position(|&byte| !byte.is_ascii_whitespace())
        else {
            i += 1;
            continue;
        };
        let Some(remaining) = after_keyword.get(value_start..) else {
            i += 1;
            continue;
        };
        let value_end = remaining
            .iter()
            .position(|&byte| byte == b'\n' || byte == b'\r')
            .unwrap_or(remaining.len());
        if value_end == 0 {
            i += 1;
            continue;
        }
        let Some(value_bytes) = remaining.get(..value_end) else {
            i += 1;
            continue;
        };
        // 值非 UTF-8 时跳过，不中断扫描。
        if let Ok(value) = std::str::from_utf8(value_bytes)
            && !value.is_empty()
        {
            return Some(value.to_owned());
        }

        i += 1;
    }
    None
}
