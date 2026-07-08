//! bmsrs 工作区的重新导出门面。
//!
//! 本 crate 聚合并重新导出各子 crate 的公共 API，
//! 模块层级统一镜像工作区的目录结构。

/// BMS 格式的解析与处理。
pub mod bms {
    /// BMS 分词器——底层逐行解析。
    pub mod tokenizer {
        pub use bms_tokenizer::*;
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

    /// BMSON 反序列化（chumsky 实现）。
    pub mod de {
        pub use bmson_de_chumsky::*;
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
