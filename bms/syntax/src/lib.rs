//! BMS 语法层。
//!
//! 本 crate 在解码后的文本上构建 rowan CST：
//!
//! - **无损**：所有字符都进入 token，`SyntaxNode::text()` 可以还原输入。
//! - **容错**：非法行与控制流结构错误不中断解析，错误以诊断和
//!   `ErrorLine` / `ErrorNode` 表示。
//! - **控制流嵌套**：`#RANDOM` / `#SWITCH` 块成为 CST 节点，分支选择由
//!   后续查询完成。
//!
//! 语义值（`BmsHeader` / `BmsMessage`）与 `Bms` 模型由 `bms-parser` 负责。

mod ast;
mod language;
mod lexer;
mod parser;
mod select;
mod syntax_kind;

pub use ast::{Line, SourceFile};
pub use language::{BmsLanguage, SyntaxElement, SyntaxKindRaw, SyntaxNode, SyntaxToken};
pub use lexer::LineKind;
pub use parser::{Diagnostic, DiagnosticKind, Parse};
pub use rowan::{TextRange, TextSize};
pub use select::{BlockDecision, BranchSelection};
pub use syntax_kind::SyntaxKind;

/// 语法解析选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
    /// 允许的头部命令前缀字符；默认 `['#', '%']`。
    pub header_prefixes: Vec<char>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            header_prefixes: vec!['#', '%'],
        }
    }
}
