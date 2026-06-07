//! Tokenizer for BMS (Be-Music Script) format.
//!
//! This crate provides the first stage of the BMS parsing pipeline:
//! converting raw BMS text into a structured token stream.
//!
//! # Architecture
//!
//! - [`BmsToken`] represents a single meaningful line in a BMS file.
//! - [`BmsHeader`] covers all header commands, categorized by semantic domain.
//! - [`BmsMessage`] covers channel data lines (`#xxxYY:values`).
//!
//! Submodules are private; all public types are re-exported from the crate root.
//!
//! # Zero-copy
//!
//! All string data borrows from the input; no allocation occurs during
//! tokenization.

mod error;
mod header;
mod message;

pub use error::TokenizerError;
pub use header::{
    BmsHeader, BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderExt, BmsHeaderGameplay,
    BmsHeaderMetadata, BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming,
};
pub use message::BmsMessage;

use header::parse_header_line;
use message::parse_message_line;

/// A single token produced by tokenizing a BMS file.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsToken<'a> {
    /// A header command (metadata, gameplay, timing, resources, etc.).
    #[serde(borrow)]
    Header(BmsHeader<'a>),
    /// A channel data line (`#xxxYY:values`).
    #[serde(borrow)]
    Message(BmsMessage<'a>),
}

/// Parse a single line of BMS text into an optional token.
///
/// Returns `Ok(None)` for lines that do not carry meaning
/// (empty lines, comments, whitespace-only).
///
/// # Errors
///
/// Returns [`TokenizerError`] if a channel message line has an invalid
/// measure or channel number.
pub fn tokenize_line(line: &str) -> Result<Option<BmsToken<'_>>, TokenizerError> {
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with("//") {
        return Ok(None);
    }

    // Try message first (channel data lines have `#xxxYY:values` format).
    if let Some(msg) = parse_message_line(trimmed)? {
        return Ok(Some(BmsToken::Message(msg)));
    }

    // Try header second.
    if let Some(hdr) = parse_header_line(trimmed) {
        return Ok(Some(BmsToken::Header(hdr)));
    }

    Ok(None)
}

/// Parse a complete BMS text into a vector of tokens.
///
/// Supports LF (`\n`), CRLF (`\r\n`), and standalone CR (`\r`) line endings
/// without heap allocation.
///
/// # Errors
///
/// Returns [`TokenizerError`] if any line has an invalid channel message format.
pub fn tokenize(input: &str) -> Result<Vec<BmsToken<'_>>, TokenizerError> {
    let mut tokens = Vec::new();

    // `str::lines()` splits on `\n` and strips trailing `\r`.
    // Standalone `\r` (old Mac style) is handled via `.split('\r')`.
    for raw_line in input.lines() {
        for segment in raw_line.split('\r') {
            if let Some(token) = tokenize_line(segment)? {
                tokens.push(token);
            }
        }
    }

    Ok(tokens)
}
