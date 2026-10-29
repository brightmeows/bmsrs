//! bmsrs 工作区的重新导出门面。
//!
//! 本 crate 聚合并重新导出各子 crate 的公共 API，
//! 模块层级统一镜像工作区的目录结构。
//! 整条管线的组装入口 [`bms::from_text`] / [`bms::from_bytes`] 也在此。

/// BMS 格式的解析与处理。
pub mod bms {
    pub use parser::ErrorStrategy;

    /// 整管线入口的错误。
    #[derive(Debug, thiserror::Error)]
    pub enum FromTextError {
        /// CST 语法或控制流结构错误。
        #[error("syntax error at {range:?}: {message}")]
        Syntax {
            /// 文本范围。
            range: syntax::TextRange,
            /// 错误消息。
            message: String,
        },
        /// 单行值解析错误。
        #[error("tokenization error at {range:?}: {error}")]
        Tokenize {
            /// 文本范围。
            range: syntax::TextRange,
            /// 分词错误。
            error: parser::BmsTokenizeError,
        },
        /// 文档解码错误。
        #[error("decode error at {range:?}: {error}")]
        Decode {
            /// 文本范围。
            range: syntax::TextRange,
            /// 解码错误。
            error: source::TextDecodeError,
        },
    }

    /// 整管线入口的非致命警告。
    #[derive(Debug, Clone, PartialEq, thiserror::Error)]
    #[non_exhaustive]
    pub enum FromTextWarning {
        /// CST 语法或控制流结构警告。
        #[error("syntax warning at {range:?}: {message}")]
        Syntax {
            /// 文本范围。
            range: syntax::TextRange,
            /// 警告消息。
            message: String,
        },
        /// 单行值解析失败，该行被跳过。
        #[error("tokenization warning at {range:?}: {error}")]
        Tokenize {
            /// 文本范围。
            range: syntax::TextRange,
            /// 分词错误。
            error: parser::BmsTokenizeError,
        },
        /// 文档解码失败。
        #[error("decode warning at {range:?}: {error}")]
        Decode {
            /// 文本范围。
            range: syntax::TextRange,
            /// 解码错误。
            error: source::TextDecodeError,
        },
    }

    /// 由原始 BMS 文本构建 [`Bms`](parser::Bms)。
    ///
    /// 管道：CST 解析 → 控制流分支选择 → 单行值解析 → `Bms`。
    ///
    /// # Errors
    ///
    /// `FailFast` 遇到首个 CST 或分词错误时返回 [`FromTextError`]。
    pub fn from_text(
        text: &str,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        from_text_with(text, &syntax::ParseOptions::default(), rng, error_strategy)
    }

    /// 使用解析选项由原始 BMS 文本构建 [`Bms`](parser::Bms)。
    ///
    /// # Errors
    ///
    /// `FailFast` 遇到首个 CST、分词或解码错误时返回 [`FromTextError`]。
    pub fn from_text_with(
        text: &str,
        options: &syntax::ParseOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        let document = source::Document::from_text(text);
        from_document(&document, options, rng, error_strategy)
    }

    /// 由原始 BMS 字节构建 [`Bms`](parser::Bms)，自动检测编码。
    ///
    /// 与 [`from_text`] 的区别是保留 `bmsrs-bms-source` 的编码检测与解码路径，
    /// 适用于 `Shift_JIS` 等非 UTF-8 谱面。
    ///
    /// # Errors
    ///
    /// `FailFast` 遇到首个 CST、分词或解码错误时返回 [`FromTextError`]。
    pub fn from_bytes(
        bytes: Vec<u8>,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        from_bytes_with(
            bytes,
            &source::DocumentOptions::default(),
            rng,
            error_strategy,
        )
    }

    /// 使用文档解码选项由原始 BMS 字节构建 [`Bms`](parser::Bms)。
    ///
    /// # Errors
    ///
    /// `FailFast` 遇到首个 CST、分词或解码错误时返回 [`FromTextError`]。
    pub fn from_bytes_with(
        bytes: Vec<u8>,
        document_options: &source::DocumentOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        let document = source::Document::from_bytes_with(bytes, document_options);
        from_document(
            &document,
            &syntax::ParseOptions::default(),
            rng,
            error_strategy,
        )
    }

    /// 从已构建的文档跑完整语义管道。
    fn from_document(
        document: &source::Document,
        options: &syntax::ParseOptions,
        rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        let parsed = parser::Bms::from_document_with(document, options, rng, error_strategy)
            .map_err(from_cst_error)?;
        Ok((
            parsed.bms,
            parsed.warnings.into_iter().map(from_cst_warning).collect(),
        ))
    }

    /// 把解析层警告转换为 facade 警告。
    fn from_cst_warning(warning: parser::CstWarning) -> FromTextWarning {
        match warning.kind {
            parser::CstWarningKind::Syntax(message) => FromTextWarning::Syntax {
                range: warning.range,
                message,
            },
            parser::CstWarningKind::Tokenize(error) => FromTextWarning::Tokenize {
                range: warning.range,
                error,
            },
            parser::CstWarningKind::Decode(error) => FromTextWarning::Decode {
                range: warning.range,
                error,
            },
        }
    }

    /// 把解析层警告转换为 facade 错误。
    fn from_cst_error(warning: parser::CstWarning) -> FromTextError {
        match warning.kind {
            parser::CstWarningKind::Syntax(message) => FromTextError::Syntax {
                range: warning.range,
                message,
            },
            parser::CstWarningKind::Tokenize(error) => FromTextError::Tokenize {
                range: warning.range,
                error,
            },
            parser::CstWarningKind::Decode(error) => FromTextError::Decode {
                range: warning.range,
                error,
            },
        }
    }

    /// 源文本层——字节、编码、行索引与字节保真编辑。
    pub mod source {
        pub use bmsrs_bms_source::*;
    }

    /// 语法层——rowan CST、词法、解析与控制流查询。
    pub mod syntax {
        pub use bmsrs_bms_syntax::*;
    }

    /// BMS 语义层——类型化 header/message 与 `Bms` 模型。
    ///
    /// `Bms` 与 `Messages` 的规范路径。值类型（`WavIndex` 等）的规范
    /// 路径是 [`values`]；此处 re-export 仅为 parser
    /// 使用者的便利别名。
    pub mod parser {
        pub use bmsrs_bms_parser::*;
    }

    /// 单行语法与类型化值——header/message、索引与解析。
    ///
    /// 值类型的规范路径。
    pub mod values {
        pub use bmsrs_bms_values::*;
    }

    /// 分析层——诊断、符号索引与编辑器数据。
    pub mod analysis {
        pub use bmsrs_bms_analysis::*;
    }

    /// BMS 处理器——将 `Bms` 转换为格式无关的 `Chart`。
    pub mod processor {
        pub use bmsrs_bms_processor::*;
    }

    /// BMS 生成器——将 `Bms` 全量重写为 BMS 文本。
    pub mod writer {
        pub use bmsrs_bms_writer::*;
    }
}

/// BMSON 格式的类型定义与转换器。
pub mod bmson {
    /// BMSON 类型系统（v0/v1/v2）。
    pub mod def {
        pub use bmsrs_bmson_def::{v0, v1, *};
    }

    /// BMSON 处理器——将 bmson 数据转换为格式无关的 `Chart`。
    pub mod processor {
        pub use bmsrs_bmson_processor::*;
    }
}

/// 格式无关的谱面数据模型。
pub mod chart {
    pub use bmsrs_chart::*;
}

/// 纯仿真 / 播放层。
pub mod player {
    pub use bmsrs_player::*;
}

/// 跨层统一的诊断出口。
///
/// 各层保留自己的 warning 类型；需要单一诊断流的消费者在这里做转换，
/// 不强迫 processor 依赖 analysis。
pub mod diagnostic {
    use bmsrs_bms_analysis::{AnalysisDiagnostic, Severity};
    use bmsrs_bms_syntax::TextRange;

    use crate::bms::FromTextWarning;

    /// 统一诊断。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Diagnostic {
        /// 严重度。
        pub severity: Severity,
        /// 文本范围；语义警告可能没有范围。
        pub range: Option<TextRange>,
        /// 消息。
        pub message: String,
        /// 稳定错误码。
        pub code: &'static str,
    }

    impl Diagnostic {
        /// 构造一条诊断。
        #[must_use]
        pub fn new(
            severity: Severity,
            range: Option<TextRange>,
            message: impl Into<String>,
            code: &'static str,
        ) -> Self {
            Self {
                severity,
                range,
                message: message.into(),
                code,
            }
        }
    }

    impl From<FromTextWarning> for Diagnostic {
        fn from(warning: FromTextWarning) -> Self {
            match warning {
                FromTextWarning::Syntax { range, message } => {
                    Self::new(Severity::Warning, Some(range), message, "syntax")
                }
                FromTextWarning::Tokenize { range, error } => Self::new(
                    Severity::Warning,
                    Some(range),
                    error.to_string(),
                    "tokenize",
                ),
                FromTextWarning::Decode { range, error } => {
                    Self::new(Severity::Warning, Some(range), error.to_string(), "decode")
                }
            }
        }
    }

    impl From<bmsrs_bms_processor::ProcessWarning> for Diagnostic {
        fn from(warning: bmsrs_bms_processor::ProcessWarning) -> Self {
            Self::new(Severity::Warning, None, warning.to_string(), "bms-process")
        }
    }

    impl From<bmsrs_bmson_processor::BmsonProcessWarning> for Diagnostic {
        fn from(warning: bmsrs_bmson_processor::BmsonProcessWarning) -> Self {
            Self::new(
                Severity::Warning,
                None,
                warning.to_string(),
                "bmson-process",
            )
        }
    }

    impl From<AnalysisDiagnostic> for Diagnostic {
        fn from(diagnostic: AnalysisDiagnostic) -> Self {
            Self::new(
                diagnostic.severity(),
                Some(diagnostic.range()),
                diagnostic.message(),
                diagnostic.code().unwrap_or("analysis"),
            )
        }
    }
}
