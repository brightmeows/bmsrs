//! bmsrs 工作区的重新导出门面。
//!
//! 本 crate 聚合并重新导出各子 crate 的公共 API，
//! 模块层级统一镜像工作区的目录结构。
//! 整条管线的组装入口 [`bms::from_text`] 也在此。

/// BMS 格式的解析与处理。
pub mod bms {
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
    }

    /// token 抽取的错误处理策略。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ErrorStrategy {
        /// 遍历全部行，错误以逐项警告出现。
        #[default]
        CollectAll,
        /// 在首个错误处停止，结果包含该错误。
        FailFast,
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
        mut rng: impl FnMut(u64) -> u64,
        error_strategy: ErrorStrategy,
    ) -> Result<(parser::Bms, Vec<FromTextWarning>), FromTextError> {
        let source = syntax::SourceFile::parse(text);
        let mut parsed = parser::Bms::from_cst(&source.source_file(), &mut rng);
        let mut warnings = Vec::new();

        for diagnostic in source.errors() {
            warnings.push(FromTextWarning::Syntax {
                range: diagnostic.range(),
                message: diagnostic.message().to_owned(),
            });
        }
        for warning in parsed.warnings.drain(..) {
            warnings.push(FromTextWarning::Tokenize {
                range: warning.range,
                error: warning.error,
            });
        }

        if error_strategy == ErrorStrategy::FailFast
            && let Some(first) = warnings.first()
        {
            return Err(match first {
                FromTextWarning::Syntax { range, message } => FromTextError::Syntax {
                    range: *range,
                    message: message.clone(),
                },
                FromTextWarning::Tokenize { range, error } => FromTextError::Tokenize {
                    range: *range,
                    error: error.clone(),
                },
            });
        }

        Ok((parsed.bms, warnings))
    }

    /// 源文本层——字节、编码、行索引与字节保真编辑。
    pub mod source {
        pub use bms_source::*;
    }

    /// 语法层——rowan CST、词法、解析与控制流查询。
    pub mod syntax {
        pub use bms_syntax::*;
    }

    /// BMS 语义层——类型化 header/message 与 `Bms` 模型。
    pub mod parser {
        pub use bms_parser::*;
    }

    /// 单行语法与类型化值——header/message、索引与解析。
    pub mod values {
        pub use bms_values::*;
    }

    /// 分析层——诊断、符号索引与编辑器数据。
    pub mod analysis {
        pub use bms_analysis::*;
    }

    /// BMS 处理器——将 `Bms` 转换为格式无关的 `Chart`。
    pub mod processor {
        pub use bms_processor::*;
    }

    /// BMS 生成器——将 `Bms` 全量重写为 BMS 文本。
    pub mod writer {
        pub use bms_writer::*;
    }
}

/// BMSON 格式的类型定义与转换器。
pub mod bmson {
    /// BMSON 类型系统（v0/v1/v2）。
    pub mod def {
        pub use bmson_def::{v0, v1, *};
    }

    /// BMSON 处理器——将 bmson 数据转换为格式无关的 `Chart`。
    pub mod processor {
        pub use bmson_processor::*;
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
