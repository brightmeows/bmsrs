//! Analysis diagnostics.

use bmsrs_bms_syntax::TextRange;
pub use bmsrs_pipeline::DiagnosticCode;

/// Diagnostic severity.
///
/// Shared with the rest of the pipeline through `bmsrs-pipeline`, so every
/// layer reports severities with the same type.
pub use bmsrs_pipeline::Severity;

/// An analysis diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Text range.
    range: TextRange,
    /// Severity.
    severity: Severity,
    /// Message.
    message: String,
    /// Stable diagnostic code.
    code: Option<DiagnosticCode>,
}

impl Diagnostic {
    /// Construct a diagnostic.
    pub(crate) fn new(
        range: TextRange,
        severity: Severity,
        message: impl Into<String>,
        code: Option<DiagnosticCode>,
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
    #[expect(
        clippy::same_name_method,
        reason = "the HasSeverity accessor mirrors this inherent getter"
    )]
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
    pub const fn code(&self) -> Option<DiagnosticCode> {
        self.code
    }
}

impl From<&Diagnostic> for bmsrs_pipeline::Diagnostic {
    fn from(diagnostic: &Diagnostic) -> Self {
        let converted = Self::new(diagnostic.severity, diagnostic.message.clone()).with_span(
            bmsrs_pipeline::SourceSpan::new(
                diagnostic.range.start().into(),
                diagnostic.range.end().into(),
            ),
        );
        match diagnostic.code {
            Some(code) => converted.with_code(code),
            None => converted,
        }
    }
}

impl bmsrs_pipeline::HasSeverity for Diagnostic {
    fn severity(&self) -> bmsrs_pipeline::Severity {
        self.severity
    }
}

impl From<Diagnostic> for bmsrs_pipeline::Diagnostic {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::from(&diagnostic)
    }
}
