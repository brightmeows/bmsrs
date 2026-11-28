//! Parser options: syntax options plus the whole-document error policy.

use bmsrs_bms_syntax::SyntaxOptions;
use bmsrs_pipeline::ErrorStrategy;

/// Options for the parser's whole-document entry points.
///
/// Syntax-level entry points take [`SyntaxOptions`] directly. The error
/// strategy belongs here because only whole-document parsing has enough
/// diagnostics to stop early.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
    /// Syntax-layer options.
    pub syntax: SyntaxOptions,
    /// Error collection strategy for whole-document parsing.
    pub error_strategy: ErrorStrategy,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            syntax: SyntaxOptions::default(),
            error_strategy: ErrorStrategy::CollectAll,
        }
    }
}

impl From<SyntaxOptions> for ParseOptions {
    fn from(syntax: SyntaxOptions) -> Self {
        Self {
            syntax,
            error_strategy: ErrorStrategy::CollectAll,
        }
    }
}
