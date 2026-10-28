//! 行模型：终止符、内容段与分类。

use std::ops::Range;

use crate::SourceEncoding;

/// 行尾终止符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// `LF`（`\n`）。
    Lf,
    /// `CRLF`（`\r\n`）。
    CrLf,
    /// 独立的 `CR`（`\r`）。
    Cr,
    /// 文件末行没有终止符。
    None,
}

impl LineEnding {
    /// 返回此终止符在给定编码下的字节长度。
    pub(crate) fn byte_len(self, encoding: SourceEncoding) -> usize {
        let unit = if encoding.is_utf16() { 2 } else { 1 };
        match self {
            Self::None => 0,
            Self::Lf | Self::Cr => unit,
            Self::CrLf => unit * 2,
        }
    }

    /// 将此终止符的字节追加到 `out`。
    pub(crate) fn append_to(self, out: &mut Vec<u8>, encoding: SourceEncoding) {
        match (self, encoding.utf16_little_endian()) {
            (Self::None, _) => {}
            (Self::Lf, Some(true)) => out.extend_from_slice(&[0x0A, 0x00]),
            (Self::Lf, Some(false)) => out.extend_from_slice(&[0x00, 0x0A]),
            (Self::Lf, None) => out.push(0x0A),
            (Self::Cr, Some(true)) => out.extend_from_slice(&[0x0D, 0x00]),
            (Self::Cr, Some(false)) => out.extend_from_slice(&[0x00, 0x0D]),
            (Self::Cr, None) => out.push(0x0D),
            (Self::CrLf, Some(true)) => out.extend_from_slice(&[0x0D, 0x00, 0x0A, 0x00]),
            (Self::CrLf, Some(false)) => out.extend_from_slice(&[0x00, 0x0D, 0x00, 0x0A]),
            (Self::CrLf, None) => out.extend_from_slice(&[0x0D, 0x0A]),
        }
    }
}

/// 文档中的一行。
///
/// 行由四段组成：前缀 trivia、内容段、后缀 trivia、行尾终止符。
/// 未被编辑的行在写回时直接使用原始字节；被编辑的行使用
/// [`Document::render`](crate::Document::render) 记录的替换字节。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// 原始字节区间（含终止符）；插入的行是空区间。
    pub(crate) span: Range<usize>,
    /// 行尾终止符。
    pub(crate) terminator: LineEnding,
    /// 内容段在原始字节中的区间；空白行、纯注释行与插入行为 `None`。
    pub(crate) content: Option<Range<usize>>,
    /// 行的语义分类。
    pub(crate) kind: LineKind,
    /// 被编辑后的完整行字节（含终止符）；`None` 表示使用原始字节。
    pub(crate) replacement: Option<Vec<u8>>,
}

impl Line {
    /// 返回行的语义分类。
    #[must_use]
    pub const fn kind(&self) -> &LineKind {
        &self.kind
    }

    /// 返回行尾终止符。
    #[must_use]
    pub const fn terminator(&self) -> LineEnding {
        self.terminator
    }

    /// 返回是否是空白行或纯注释行。
    #[must_use]
    pub const fn is_trivia(&self) -> bool {
        self.kind.is_trivia()
    }

    /// 返回行内容是否经过编辑。
    #[must_use]
    pub const fn is_edited(&self) -> bool {
        self.replacement.is_some()
    }
}

/// 行的语义分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// 空白行。
    Blank,
    /// 纯注释行。
    Comment,
    /// 既不是注释也不是命令的文本行。
    Text,
    /// 解码失败的行。
    Error(LineError),
}

impl LineKind {
    /// 返回是否是空白行或纯注释行。
    #[must_use]
    pub const fn is_trivia(&self) -> bool {
        matches!(self, Self::Blank | Self::Comment)
    }
}

/// 行解析错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LineError {
    /// 行的字节在文档编码下无法完整解码。
    #[error("line cannot be decoded with {encoding}")]
    Decode {
        /// 文档编码名。
        encoding: &'static str,
    },
}
