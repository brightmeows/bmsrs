//! bmsrs 工作区的重新导出门面。
//!
//! 本 crate 聚合并重新导出各子 crate 的公共 API，
//! 模块层级统一镜像工作区的目录结构。
//! 整条管线的组装入口（[`bms::from_text`]）也在此——parser crate
//! 保持只依赖 tokenizer + control-flow，不越层持有文档层。

/// BMS 格式的解析与处理。
pub mod bms {
    /// 由原始 BMS 文本经过完整管道（分词 → 控制流展开 → 解析）构建
    /// [`Bms`](bms_parser::Bms)，同时返回控制流阶段产生的警告。
    ///
    /// 等价于以下步骤的组合调用：
    /// 1. 使用 [`Document`](document::Document) 构建无损文档
    /// 2. 按 `error_strategy` 抽取 token 并跳过错误
    /// 3. 使用 [`FlowDoc::from_tokens`](control_flow::FlowDoc::from_tokens)
    ///    构建控制流树
    /// 4. 使用 `rng` 选择分支
    /// 5. 使用
    ///    [`Bms::from_flat_tokens`](parser::Bms::from_flat_tokens) 解析
    ///
    /// 分词阶段的逐行解析错误按 `error_strategy` 处理；仅控制流结构
    /// 错误（如不匹配的 `#IF` / `#ENDRANDOM`）会作为错误返回。
    ///
    /// # Errors
    ///
    /// 当控制流结构不合法时返回
    /// [`ControlFlowError`](control_flow::ControlFlowError)。
    pub fn from_text<R>(
        text: &str,
        rng: &mut R,
        error_strategy: document::ErrorStrategy,
    ) -> Result<
        (parser::Bms, std::vec::Vec<control_flow::ControlFlowWarning>),
        control_flow::ControlFlowError,
    >
    where
        R: control_flow::BranchRng,
    {
        let document = document::Document::from_text(text);
        let token_pairs = document
            .token_results_with(error_strategy)
            .filter_map(|(line, result)| result.ok().map(|token| (line, token)));

        let doc = control_flow::FlowDoc::from_tokens(token_pairs)?;
        let warnings = doc.warnings().to_vec();
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
