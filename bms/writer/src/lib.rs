//! Full-rewrite generator from the `Bms` model to BMS text.
//!
//! [`BmsWriter`] regenerates BMS text from the structured [`Bms`] model. The
//! output is **canonical**: headers are emitted grouped by domain, and channel
//! lines are sorted and merged by (measure, channel); comments, original line
//! formatting, and blank lines are not preserved. For byte-faithful line-level
//! editing, use `bmsrs-bms-source`'s line editing instead; this crate is the fallback
//! rewrite after structural changes.
//!
//! # Pipeline
//!
//! ```text
//! bmsrs_bms_parser::Bms -> BmsWriter::write -> String (BMS text)
//! ```

mod headers;
mod messages;

use bmsrs_bms_parser::Bms;

/// BMS text generator.
///
/// Zero-sized type; all functionality is exposed through the associated
/// function [`BmsWriter::write`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BmsWriter;

impl BmsWriter {
    /// Generate BMS text from the [`Bms`] model.
    ///
    /// Output layout:
    /// 1. Header commands grouped as metadata, gameplay, timing, display,
    ///    audio, visual, then unrecognized headers (original order)
    /// 2. Channel lines sorted ascending by (measure, channel); multiple lines
    ///    for the same (measure, channel) are merged into one line by position
    ///    (non-`00` overrides, `00` preserves the position), matching
    ///    [`bmsrs_bms_parser`]'s parse-time merge semantics
    /// 3. Terminated by a newline
    ///
    /// Output is Unicode text; character-encoding conversion (`Shift-JIS` and
    /// so on) is the caller's responsibility.
    #[must_use]
    pub fn write(bms: &Bms) -> String {
        let mut lines = headers::write_headers(bms);
        lines.extend(messages::write_messages(&bms.messages));
        let mut text = lines.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        text
    }
}
