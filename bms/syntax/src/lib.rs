//! BMS syntax layer.
//!
//! This crate builds a rowan CST over decoded text:
//!
//! - **Lossless**: every character enters exactly one token, so
//!   `SyntaxNode::text()` reconstructs the input byte for byte.
//! - **Error-tolerant**: invalid lines, unclosed blocks, and orphan directives
//!   never panic; errors become `Diagnostic`s and `ErrorLine` / `ErrorToken`
//!   nodes that preserve the original text where possible.
//! - **Line-oriented**: every line has at least one `Line` node; control-flow
//!   blocks nest under `SourceFile`.
//! - **Semantics-free**: no `BmsHeader` / `BmsMessage` construction here;
//!   only command names and value text.
//! - **Branches optional**: the CST keeps every branch; `select_lines`
//!   returns the lines of one resolved pass.
//! - **Nested control flow**: `#RANDOM` / `#SWITCH` blocks become CST nodes;
//!   branch selection is performed by later queries.
//!
//! Semantic values (`BmsHeader` / `BmsMessage`) and the `Bms` model belong to
//! `bmsrs-bms-parser`.

mod ast;
mod java_random;
mod language;
mod lexer;
mod parser;
mod random;
mod select;
mod syntax_kind;

pub use ast::{SourceFile, SyntaxLine};
pub use bmsrs_pipeline::{DiagnosticCode, Severity};
pub use java_random::JavaRandom;
pub use language::{BmsLanguage, SyntaxElement, SyntaxKindRaw, SyntaxNode, SyntaxToken};
pub use lexer::SyntaxLineKind;
pub use parser::{Diagnostic, IncrementalParser, SyntaxParse};
pub use random::{ControlFlowRng, FirstBranch, LR2_SEED_MAX, Lr2Mt19937};
pub use rowan::{TextRange, TextSize};
pub use select::{BlockDecision, BlockKind, BranchMismatch, BranchSelection, ControlFlowCompat};
pub use syntax_kind::SyntaxKind;

/// Syntax parsing options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
    /// Source format selected by the entry layer.
    ///
    /// The syntax layer itself ignores this; the parser uses it for `.bmd`
    /// base and comment rules.
    pub format: bmsrs_pipeline::SourceFormat,
    /// Allowed header command prefix characters; defaults to `['#', '%']`.
    pub header_prefixes: Vec<char>,
    /// Control-flow compatibility mode; defaults to [`ControlFlowCompat::Ideal`].
    pub control_flow_compat: ControlFlowCompat,
    /// Error collection strategy; defaults to
    /// [`bmsrs_pipeline::ErrorStrategy::CollectAll`].
    pub error_strategy: bmsrs_pipeline::ErrorStrategy,
}

impl ParseOptions {
    /// Default control-flow branch selector: always selects the first branch,
    /// regardless of the drawn upper bound.
    #[must_use]
    pub const fn first_branch(_max: u64) -> u64 {
        1
    }
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            header_prefixes: vec!['#', '%'],
            control_flow_compat: ControlFlowCompat::Ideal,
            error_strategy: bmsrs_pipeline::ErrorStrategy::CollectAll,
            format: bmsrs_pipeline::SourceFormat::Bms,
        }
    }
}
