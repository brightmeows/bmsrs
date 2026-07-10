//! BMS 文件编码检测与转换。
//!
//! 提供 [`BmsEncoding`] 枚举（含 [`BmsEncoding::detect`] 方法），
//! 用于在分词前识别及转换 BMS 文件的字符编码。

use encoding_rs::Encoding;

/// BMS 文件检测到的文本编码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmsEncoding {
    /// UTF-8（可能带 BOM）。
    Utf8,
    /// UTF-16LE with BOM。
    Utf16Le,
    /// UTF-16BE with BOM。
    Utf16Be,
    /// `Shift_JIS`（常见于日文 BMS 文件）。
    ShiftJis,
    /// `EUC-KR`（常见于韩文 BMS 文件）。
    EucKr,
}

impl BmsEncoding {
    /// 检测 BMS 文件编码，通过扫描 BOM 和 `#CHARSET` 头。
    ///
    /// 检测顺序：
    /// 1. BOM 检测（UTF-8、UTF-16LE、UTF-16BE）
    /// 2. 无 BOM 时扫描 `#CHARSET` 头（字节级匹配，不依赖 UTF-8 解码）
    /// 3. 试探 UTF-8 / `Shift_JIS` / `EUC-KR`
    /// 4. 回落至 UTF-8
    #[must_use]
    pub fn detect(input: &[u8]) -> Self {
        if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Self::Utf8;
        }
        if input.starts_with(&[0xFF, 0xFE]) {
            return Self::Utf16Le;
        }
        if input.starts_with(&[0xFE, 0xFF]) {
            return Self::Utf16Be;
        }

        if let Some(charset) = scan_charset_header(input) {
            return encoding_from_charset_value(&charset);
        }

        if std::str::from_utf8(input).is_ok() {
            return Self::Utf8;
        }

        {
            let (_, _, had_errors) = encoding_rs::SHIFT_JIS.decode(input);
            if !had_errors {
                return Self::ShiftJis;
            }
        }

        {
            let (_, _, had_errors) = encoding_rs::EUC_KR.decode(input);
            if !had_errors {
                return Self::EucKr;
            }
        }

        Self::Utf8
    }

    /// 使用 `encoding_rs` 将字节解码为 UTF-8 `String`。
    ///
    /// 如果输入包含对应编码的 BOM，则自动去除。
    #[must_use]
    pub fn decode(&self, input: &[u8]) -> String {
        match self {
            Self::Utf8 => {
                let data = input.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(input);
                // 对有效 UTF-8 避免 encoding_rs 的 Cow::into_owned 分配：
                // from_utf8 在成功时零拷贝，失败时 fallback 到 encoding_rs
                std::str::from_utf8(data).map_or_else(
                    |_| encoding_rs::UTF_8.decode(data).0.into_owned(),
                    str::to_owned,
                )
            }
            Self::Utf16Le => {
                let data = input.strip_prefix(&[0xFF, 0xFE]).unwrap_or(input);
                encoding_rs::UTF_16LE.decode(data).0.into_owned()
            }
            Self::Utf16Be => {
                let data = input.strip_prefix(&[0xFE, 0xFF]).unwrap_or(input);
                encoding_rs::UTF_16BE.decode(data).0.into_owned()
            }
            Self::ShiftJis => encoding_rs::SHIFT_JIS.decode(input).0.into_owned(),
            Self::EucKr => encoding_rs::EUC_KR.decode(input).0.into_owned(),
        }
    }
}

/// 字节级扫描 `#CHARSET` 或 `%CHARSET` 行首指令。
///
/// 仅在行首位置匹配（`input[0]` 或前一个字节为 `\n`/`\r`），
/// 避免匹配注释或通道数据中的内容。
fn scan_charset_header(input: &[u8]) -> Option<String> {
    let mut i = 0;
    while i < input.len() {
        let is_line_start = i == 0 || input.get(i - 1).is_some_and(|&b| b == b'\n' || b == b'\r');

        if !is_line_start {
            i += 1;
            continue;
        }

        let Some(&prefix_byte) = input.get(i) else {
            break;
        };
        if prefix_byte != b'#' && prefix_byte != b'%' {
            i += 1;
            continue;
        }

        let Some(after_prefix) = input.get(i + 1..) else {
            break;
        };
        if after_prefix.len() < 7
            || !after_prefix
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
        // 值为空行或全空白时跳过此行，继续扫描后续 #CHARSET 行
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
        // 值非 UTF-8 时跳过，不中断扫描
        if let Ok(value) = std::str::from_utf8(value_bytes)
            && !value.is_empty()
        {
            return Some(value.to_owned());
        }

        i += 1;
    }
    None
}

/// 将 `#CHARSET` 的值映射为 [`BmsEncoding`]。
///
/// 优先匹配已知名称（`UTF-8`、`SHIFT-JIS`、`EUC-KR` 及常见变体）；
/// 若不匹配则尝试 `encoding_rs::Encoding::for_label` 兜底。
fn encoding_from_charset_value(value: &str) -> BmsEncoding {
    let upper = value.trim().to_ascii_uppercase();
    match upper.as_str() {
        "UTF-8" | "UTF8" => BmsEncoding::Utf8,
        "SHIFT-JIS" | "SHIFT_JIS" | "SJIS" => BmsEncoding::ShiftJis,
        "EUC-KR" | "EUCKR" | "EUC_KR" => BmsEncoding::EucKr,
        other => Encoding::for_label(other.as_bytes()).map_or(BmsEncoding::Utf8, |enc| {
            if enc == encoding_rs::UTF_8 {
                BmsEncoding::Utf8
            } else if enc == encoding_rs::SHIFT_JIS {
                BmsEncoding::ShiftJis
            } else if enc == encoding_rs::EUC_KR {
                BmsEncoding::EucKr
            } else {
                BmsEncoding::Utf8
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_utf8_detected() {
        let input = &[0xEF, 0xBB, 0xBF, b'T', b'e', b's', b't'];
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::Utf8);
    }

    #[test]
    fn bom_utf16le_detected() {
        let input = &[0xFF, 0xFE, 0x54, 0x00, 0x65, 0x00];
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::Utf16Le);
    }

    #[test]
    fn bom_utf16be_detected() {
        let input = &[0xFE, 0xFF, 0x00, 0x54, 0x00, 0x65];
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::Utf16Be);
    }

    #[test]
    fn charset_shift_jis_detected() {
        let input = b"#TITLE\r\n#CHARSET Shift_JIS\r\n#BPM 180";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::ShiftJis);
    }

    #[test]
    fn charset_euc_kr_detected() {
        let input = b"#CHARSET EUC-KR";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::EucKr);
    }

    #[test]
    fn charset_utf8_detected() {
        let input = b"#CHARSET UTF-8\n#TITLE test";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::Utf8);
    }

    #[test]
    fn charset_percent_prefix_detected() {
        let input = b"%CHARSET Shift-JIS\n#TITLE test";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::ShiftJis);
    }

    #[test]
    fn charset_sjis_alias_detected() {
        let input = b"#CHARSET SJIS\n";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::ShiftJis);
    }

    #[test]
    fn charset_utf8_alias_detected() {
        let input = b"#CHARSET UTF8";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::Utf8);
    }

    #[test]
    fn charset_euckr_alias_detected() {
        let input = b"#CHARSET EUCKR";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::EucKr);
    }

    #[test]
    fn plain_ascii_detected_as_utf8() {
        let input = b"#TITLE Hello\n#BPM 180";
        assert_eq!(BmsEncoding::detect(input), BmsEncoding::Utf8);
    }

    #[test]
    fn empty_input_detected_as_utf8() {
        assert_eq!(BmsEncoding::detect(b""), BmsEncoding::Utf8);
    }

    #[test]
    fn decode_utf8_preserves_content() {
        let input = b"#TITLE \xe3\x83\x86\xe3\x82\xb9\xe3\x83\x88";
        let decoded = BmsEncoding::Utf8.decode(input);
        assert_eq!(decoded, "#TITLE \u{30c6}\u{30b9}\u{30c8}");
    }

    #[test]
    fn decode_utf8_strips_bom() {
        let input = &[0xEF, 0xBB, 0xBF, b'H', b'i'];
        let decoded = BmsEncoding::Utf8.decode(input);
        assert_eq!(decoded, "Hi");
    }

    #[test]
    fn decode_utf16le_strips_bom() {
        // U+0048 U+0069 = "Hi" in UTF-16LE
        let input = &[0xFF, 0xFE, 0x48, 0x00, 0x69, 0x00];
        let decoded = BmsEncoding::Utf16Le.decode(input);
        assert_eq!(decoded, "Hi");
    }

    #[test]
    fn decode_shift_jis_correctly() {
        // "あ" in Shift_JIS = 0x82 0xA0
        let input = &[0x82, 0xA0];
        let decoded = BmsEncoding::ShiftJis.decode(input);
        assert_eq!(decoded, "\u{3042}");
    }

    #[test]
    fn decode_euc_kr_correctly() {
        // "가" in EUC-KR = 0xB0 0xA1
        let input = &[0xB0, 0xA1];
        let decoded = BmsEncoding::EucKr.decode(input);
        assert_eq!(decoded, "\u{AC00}");
    }
}
