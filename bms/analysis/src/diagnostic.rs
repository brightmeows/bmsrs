//! Analysis diagnostics.

use bmsrs_bms_syntax::TextRange;

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Error.
    Error,
    /// Warning.
    Warning,
    /// Information.
    Information,
}

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
