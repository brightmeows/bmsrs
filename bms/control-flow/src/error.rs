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
