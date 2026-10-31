//! Line model: terminators, content spans, and classification.

use std::ops::Range;

use crate::SourceEncoding;

/// Line terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// `LF` (`\n`).
    Lf,
    /// `CRLF` (`\r\n`).
    CrLf,
    /// A lone `CR` (`\r`).
    Cr,
    /// The final line has no terminator.
    None,
}

impl LineEnding {
    /// Return the byte length of this terminator in the given encoding.
    pub(crate) fn byte_len(self, encoding: SourceEncoding) -> usize {
        let unit = if encoding.is_utf16() { 2 } else { 1 };
        match self {
            Self::None => 0,
            Self::Lf | Self::Cr => unit,
            Self::CrLf => unit * 2,
        }
    }

    /// Append this terminator's bytes to `out`.
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

/// A line in a document.
///
/// A line consists of four segments: prefix trivia, content span, suffix
/// trivia, and line terminator. Unedited lines use their raw bytes when written
/// back; edited lines use the replacement bytes recorded by
/// [`Document::render`](crate::Document::render).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Raw byte range, including the terminator; inserted lines have an empty
    /// range.
    pub(crate) span: Range<usize>,
    /// Line terminator.
    pub(crate) terminator: LineEnding,
    /// Content span in the raw bytes; `None` for blank lines, comment-only
    /// lines, and inserted lines.
    pub(crate) content: Option<Range<usize>>,
    /// Semantic classification of the line.
    pub(crate) kind: LineKind,
    /// Full line bytes after editing, including the terminator; `None` means
    /// use the raw bytes.
    pub(crate) replacement: Option<Vec<u8>>,
}

impl Line {
    /// Return the semantic classification of the line.
    #[must_use]
    pub const fn kind(&self) -> &LineKind {
        &self.kind
    }

    /// Return the line terminator.
    #[must_use]
    pub const fn terminator(&self) -> LineEnding {
        self.terminator
    }

    /// Return whether the line is blank or comment-only.
    #[must_use]
    pub const fn is_trivia(&self) -> bool {
        self.kind.is_trivia()
    }

    /// Return whether the line content was edited.
    #[must_use]
    pub const fn is_edited(&self) -> bool {
        self.replacement.is_some()
    }
}

/// Semantic classification of a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// Blank line.
    Blank,
    /// Comment-only line.
    Comment,
    /// A text line that is neither a comment nor a command.
    Text,
    /// A line that failed to decode.
    Error(LineError),
}

impl LineKind {
    /// Return whether the line is blank or comment-only.
    #[must_use]
    pub const fn is_trivia(&self) -> bool {
        matches!(self, Self::Blank | Self::Comment)
    }
}

/// Line parsing error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LineError {
    /// The line bytes cannot be decoded completely with the document encoding.
    #[error("line cannot be decoded with {encoding}")]
    Decode {
        /// Document encoding name.
        encoding: &'static str,
    },
}
