//! 控制流解析的错误类型。

use std::num::NonZeroUsize;

/// 从扁平 token 流构建 [`FlowDoc`](crate::FlowDoc) 时可能发生的错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ControlFlowError {
    /// `#IF`、`#ELSEIF` 或 `#ELSE` 出现在 `#RANDOM` / `#SETRANDOM` 块之外。
    #[error("#IF/#ELSEIF/#ELSE without matching #RANDOM at line {line}")]
    UnmatchedIf {
        /// 触发错误的命令所在行号。
        line: NonZeroUsize,
    },

    /// 出现 `#ENDIF` 但没有匹配的 `#IF`。
    #[error("#ENDIF without matching #IF at line {line}")]
    UnmatchedEndIf {
        /// 触发错误的命令所在行号。
        line: NonZeroUsize,
    },

    /// 出现 `#ENDRANDOM` 但没有匹配的 `#RANDOM`。
    #[error("#ENDRANDOM without matching #RANDOM at line {line}")]
    UnmatchedEndRandom {
        /// 触发错误的命令所在行号。
        line: NonZeroUsize,
    },

    /// `#CASE` 或 `#DEF` 出现在 `#SWITCH` / `#SETSWITCH` 块之外。
    #[error("#CASE/#DEF without matching #SWITCH at line {line}")]
    UnmatchedCase {
        /// 触发错误的命令所在行号。
        line: NonZeroUsize,
    },

    /// 出现 `#ENDSW` 但没有匹配的 `#SWITCH`。
    #[error("#ENDSW without matching #SWITCH at line {line}")]
    UnmatchedEndSw {
        /// 触发错误的命令所在行号。
        line: NonZeroUsize,
    },

    /// 其他控制流语法错误（例如 `#SKIP` 出现在 `#SWITCH` 之外）。
    #[error("{message} at line {line}")]
    UnexpectedControlFlow {
        /// 错误描述。
        message: &'static str,
        /// 触发错误的命令所在行号。
        line: NonZeroUsize,
    },
}

/// 从扁平 token 流构建 [`FlowDoc`](crate::FlowDoc) 时发现的非致命问题。
///
/// 警告不阻止构建，但可能丢失控制流结构信息（例如未闭合的块
/// 其内容被提升到父作用域）。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ControlFlowWarning {
    /// `#RANDOM` 或 `#SWITCH` 块在流结束前未用 `#ENDRANDOM` / `#ENDSW` 闭合。
    ///
    /// 块内容（包括所有分支 / case）被提升到父作用域，控制流结构信息丢失。
    #[error(
        "unclosed #{block_kind} block (started at line {line}) — content promoted to parent scope"
    )]
    UnclosedBlock {
        /// 块类型（`"RANDOM"` 或 `"SWITCH"`）。
        block_kind: &'static str,
        /// 块起始命令的行号。
        line: NonZeroUsize,
    },
}
