//! bmsrs 工作区的重新导出门面。
//!
//! 本 crate 聚合并重新导出各子 crate 的公共 API，
//! 模块层级统一镜像工作区的目录结构。
//! 整条管线的组装入口（[`bms::from_text`]）也在此——parser crate
//! 保持只依赖 tokenizer + control-flow，不越层持有文档层。

/// BMS 格式的解析与处理。
pub mod bms {
    /// `bms::from_text` 组装管道产生的错误。
    #[derive(Debug, thiserror::Error)]
    pub enum FromTextError {
        /// 控制流结构不合法（如不匹配的 `#IF` / `#ENDRANDOM`）。
        #[error(transparent)]
        ControlFlow(#[from] control_flow::ControlFlowError),
        /// [`ErrorStrategy::FailFast`](document::ErrorStrategy::FailFast)
        /// 在分词阶段遇到首个错误行，管道在该行处终止。
        #[error("tokenization failed at line {line}: {error}")]
        Tokenize {
            /// 出错行的行号（1 起）。
            line: std::num::NonZeroUsize,
            /// 分词错误。
            #[source]
            error: tokenizer::BmsTokenizeError,
        },
    }

    /// `bms::from_text` 组装管道产生的警告。
    #[derive(Debug, Clone, PartialEq, thiserror::Error)]
    #[non_exhaustive]
    pub enum FromTextWarning {
        /// 控制流结构警告（如未闭合的 `#RANDOM` 块）。
        #[error(transparent)]
        ControlFlow(#[from] control_flow::ControlFlowWarning),
        /// 分词失败，该行被跳过
        /// （[`ErrorStrategy::CollectAll`](document::ErrorStrategy::CollectAll)）。
        #[error("line {line}: {error}")]
        Tokenize {
            /// 出错行的行号（1 起）。
            line: std::num::NonZeroUsize,
            /// 分词错误。
            error: tokenizer::BmsTokenizeError,
        },
    }

    /// 由原始 BMS 文本经过完整管道（分词 → 控制流展开 → 解析）构建
    /// [`Bms`](bms_parser::Bms)，同时返回管道产生的警告。
    ///
    /// 等价于以下步骤的组合调用：
    /// 1. 使用 [`Document`](document::Document) 构建无损文档
    /// 2. 按 `error_strategy` 抽取 token
    /// 3. 使用 [`FlowDoc::from_tokens`](control_flow::FlowDoc::from_tokens)
    ///    构建控制流树
    /// 4. 使用 `rng` 选择分支
    /// 5. 使用
    ///    [`Bms::from_flat_tokens`](parser::Bms::from_flat_tokens) 解析
    ///
    /// 分词阶段的逐行错误按 `error_strategy` 分流：
    /// [`CollectAll`](document::ErrorStrategy::CollectAll) 跳过错误行并把
    /// 逐条错误放入返回的警告；
    /// [`FailFast`](document::ErrorStrategy::FailFast) 把首个错误行作为
    /// [`FromTextError::Tokenize`] 返回。控制流结构错误（如不匹配的
    /// `#IF` / `#ENDRANDOM`）作为 [`FromTextError::ControlFlow`] 返回。
    ///
    /// # Errors
    ///
    /// 分词错误（FailFast）或控制流结构不合法时返回
    /// [`FromTextError`]。
    pub fn from_text<R>(
        text: &str,
        rng: &mut R,
        error_strategy: document::ErrorStrategy,
    ) -> Result<(parser::Bms, std::vec::Vec<FromTextWarning>), FromTextError>
    where
        R: control_flow::BranchRng,
    {
        let document = document::Document::from_text(text);
        let mut warnings = std::vec::Vec::new();
        let mut token_pairs = std::vec::Vec::new();
        for (line, result) in document.token_results_with(error_strategy) {
            match result {
                Ok(token) => token_pairs.push((line, token)),
                Err(error) => match error_strategy {
                    document::ErrorStrategy::FailFast => {
                        return Err(FromTextError::Tokenize { line, error });
                    }
                    document::ErrorStrategy::CollectAll => {
                        warnings.push(FromTextWarning::Tokenize { line, error });
                    }
                },
            }
        }

        let doc = control_flow::FlowDoc::from_tokens(token_pairs)?;
        warnings.extend(
            doc.warnings()
                .iter()
                .cloned()
                .map(FromTextWarning::ControlFlow),
        );
        let (flat, _) = doc.select_branches(rng);
        Ok((parser::Bms::from_flat_tokens(flat), warnings))
    }

    /// BMS 单行语法解析。
    pub mod tokenizer {
        pub use bms_tokenizer::*;
    }

    /// 无损文档层——字节保真、行级编辑与编码处理。
    pub mod document {
        pub use bms_document::*;
    }

    /// 源文本层——字节、编码、行索引与字节保真编辑。
    pub mod source {
        pub use bms_source::*;
    }

    /// 语法层——rowan CST、词法、解析与控制流查询。
    pub mod syntax {
        pub use bms_syntax::*;
    }

    /// 分析层——诊断、符号索引与 LSP 数据。
    pub mod analysis {
        pub use bms_analysis::*;
    }

    /// 控制流树构建、分支选择与往返转换。
    pub mod control_flow {
        pub use bms_control_flow::*;
    }

    /// BMS 解析器——从扁平 token 流构建结构化文档模型。
    pub mod parser {
        pub use bms_parser::*;
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
    ///
    /// v0 和 v1 子模块通过显式 `pub use` 和底层的 `*` glob 共同导出，
    /// 确保即使 `bmson_def` 内部调整模块可见性也不会影响此门面。
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
