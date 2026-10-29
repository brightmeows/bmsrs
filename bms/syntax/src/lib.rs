//! BMS syntax layer.
//!
//! This crate builds a rowan CST over decoded text:
//!
//! - **Lossless**: every character enters a token, so `SyntaxNode::text()` can
//!   reconstruct the input.
//! - **Error-tolerant**: invalid lines and malformed control flow do not stop
//!   parsing; errors are reported as diagnostics and as `ErrorLine` /
//!   `ErrorNode`.
//! - **Nested control flow**: `#RANDOM` / `#SWITCH` blocks become CST nodes;
//!   branch selection is performed by later queries.
//!
//! Semantic values (`BmsHeader` / `BmsMessage`) and the `Bms` model belong to
//! `bmsrs-bms-parser`.

mod ast;
mod language;
mod lexer;
mod parser;
mod select;
mod syntax_kind;

pub use ast::{Line, SourceFile};
pub use language::{BmsLanguage, SyntaxElement, SyntaxKindRaw, SyntaxNode, SyntaxToken};
pub use lexer::LineKind;
pub use parser::{Diagnostic, DiagnosticKind, Parse};
pub use rowan::{TextRange, TextSize};
pub use select::{BlockDecision, BranchSelection};
pub use syntax_kind::SyntaxKind;

/// Syntax parsing options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
    /// Allowed header command prefix characters; defaults to `['#', '%']`.
    pub header_prefixes: Vec<char>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            header_prefixes: vec!['#', '%'],
        }
    }
}
