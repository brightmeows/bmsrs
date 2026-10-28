//! BMS 分析层。
//!
//! 在 `bms-syntax` 的 CST 上构建编辑器所需数据：
//!
//! - 语法与控制流诊断
//! - 资源定义 / 引用索引（WAV、BMP、BGA、BPM、STOP、SCROLL、SPEED 等）
//! - 重复定义与缺失引用诊断
//!
//! 本 crate 不依赖 LSP 类型，协议适配由独立仓库 `bms-lsp` 负责。

mod analysis;
mod diagnostic;
mod docs;
mod index;

pub use analysis::Analysis;
pub use diagnostic::{AnalysisDiagnostic, Severity};
pub use docs::{
    CHANNEL_DOCS, ChannelDoc, HEADER_DOCS, HeaderDoc, channel_completions, channel_doc,
    header_completions, header_doc,
};
pub use index::{SymbolDefinition, SymbolIndex, SymbolKind, SymbolReference};
