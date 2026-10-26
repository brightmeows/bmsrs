//! 文件格式嗅探。

use std::path::Path;

/// 谱面文件格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// BMS 文本格式（`.bms`/`.bme`/`.bml`/`.pms` 等）。
    Bms,
    /// bmson JSON 格式。
    Bmson,
}

/// 按扩展名与内容嗅探格式。
///
/// 扩展名 `.bmson`/`.json` 直接判 bmson；其余按内容——首个
/// 非空白字节为 `{` 判 bmson，否则判 BMS。
#[must_use]
pub fn detect(path: &Path, bytes: &[u8]) -> FileFormat {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("bmson") || ext.eq_ignore_ascii_case("json"))
    {
        return FileFormat::Bmson;
    }
    if bytes
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'{')
    {
        return FileFormat::Bmson;
    }
    FileFormat::Bms
}
