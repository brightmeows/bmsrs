//! Analysis diagnostics.

use bmsrs_bms_syntax::TextRange;

/// Diagnostic severity.
///
/// Shared with the rest of the pipeline through `bmsrs-pipeline`, so every
/// layer reports severities with the same type.
pub use bmsrs_pipeline::Severity;

/// An analysis diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisDiagnostic {
    /// Text range.
    range: TextRange,
    /// Severity.
    severity: Severity,
    /// Message.
    message: String,
    /// Stable error code.
    code: Option<&'static str>,
}

impl AnalysisDiagnostic {
    /// Construct a diagnostic.
    pub(crate) fn new(
        range: TextRange,
        severity: Severity,
        message: impl Into<String>,
        code: Option<&'static str>,
    ) -> Self {
        Self {
            range,
            severity,
            message: message.into(),
            code,
        }
    }

    /// Return the text range.
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    /// Return the severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Return the message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the error code.
    #[must_use]
    pub const fn code(&self) -> Option<&'static str> {
        self.code
    }
}
