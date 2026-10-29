//! BMS source-text layer.
//!
//! This crate handles only the "bytes to lines" part and does not parse BMS
//! syntax:
//!
//! - **Byte fidelity**: it retains the raw bytes and encoding. Unedited lines
//!   are sliced verbatim when written back, without decoding or re-encoding.
//! - **Line structure**: it splits the file into lines and records each line's
//!   byte range, terminator, content span, and whitespace/comment trivia.
//! - **Editing and write-back**: replace, insert, and delete by line;
//!   [`Document::render`] emits a new byte sequence that includes unedited lines.
//!
//! Header commands, channel lines, and the structure of `#RANDOM` / `#SWITCH`
//! are handled by the CST in `bmsrs-bms-syntax`. This layer only classifies a
//! line as blank, comment, plain text, or a decode error.
//!
//! # Line model
//!
//! A line consists of four segments:
//!
//! ```text
//! <prefix trivia><content span><suffix trivia><line terminator>
//! ```
//!
//! The content span is the first contiguous non-trivia region; the prefix and
//! suffix contain indentation, inline comments, and remnants of multi-line
//! block comments. Structured edits replace only the content span, while the
//! prefix and suffix are spliced from the raw bytes, so comments and
//! indentation in edited lines are never re-encoded.
//!
//! # Encoding
//!
//! Supports every ASCII-compatible, stateless encoding (UTF-8, `Shift_JIS`,
//! `EUC-KR`, `GBK`, `Big5`, and so on), plus UTF-16LE and UTF-16BE with a BOM.
//! `ISO-2022-JP` and the `replacement` encoding are rejected: the former has
//! cross-line state and cannot be encoded line by line; the latter cannot
//! decode.

mod classify;
mod document;
mod encoding;
mod line;
mod source;

pub use classify::{ScanState, Scanned};
pub use document::{Document, DocumentOptions, EditError, TextDecodeError};
pub use encoding::{
    DetectedEncoding, DetectionMethod, SourceEncoding, UnmappableCharacter, UnsupportedEncoding,
};
pub use line::{Line, LineEnding, LineError, LineKind};
