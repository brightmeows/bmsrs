//! BMS analysis layer.
//!
//! Builds the data an editor needs on top of the `bmsrs-bms-syntax` CST:
//!
//! - syntax and control-flow diagnostics
//! - resource definition and reference indices (WAV, BMP, BGA, BPM, STOP,
//!   SCROLL, SPEED, and so on)
//! - duplicate-definition and missing-reference diagnostics
//!
//! This crate does not depend on LSP types; protocol adaptation belongs to the
//! separate `bms-lsp` repository.

mod analysis;
mod diagnostic;
mod docs;
mod index;

pub use analysis::Analysis;
pub use diagnostic::{AnalysisDiagnostic, Severity};
pub use docs::{
    CHANNEL_DOCS, ChannelDoc, HEADER_DOCS, HeaderDoc, channel_completions, channel_doc,
    header_completions, header_doc,
};
pub use index::{SymbolDefinition, SymbolIndex, SymbolKind, SymbolReference};
