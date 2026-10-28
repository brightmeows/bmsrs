//! 分析诊断。

use bms_syntax::TextRange;

/// 诊断严重度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// 错误。
    Error,
    /// 警告。
    Warning,
    /// 提示。
    Information,
}

/// 一条分析诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisDiagnostic {
    /// 文本范围。
    range: TextRange,
    /// 严重度。
    severity: Severity,
    /// 消息。
    message: String,
    /// 稳定错误码。
    code: Option<&'static str>,
}

impl AnalysisDiagnostic {
    /// 构造诊断。
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

    /// 返回文本范围。
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    /// 返回严重度。
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// 返回消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 返回错误码。
    #[must_use]
    pub const fn code(&self) -> Option<&'static str> {
        self.code
    }
}
