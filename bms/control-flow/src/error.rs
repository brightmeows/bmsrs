//! Error types for control-flow parsing.

use std::num::NonZeroUsize;

/// Errors that can occur while building a [`FlowDocument`](crate::FlowDocument)
/// from a flat token stream.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ControlFlowError {
    /// `#IF`, `#ELSEIF`, or `#ELSE` appeared outside a `#RANDOM` / `#SETRANDOM` block.
    #[error("#IF/#ELSEIF/#ELSE without matching #RANDOM at line {line}")]
    UnmatchedIf {
        /// Line number of the offending command.
        line: NonZeroUsize,
    },

    /// `#ENDIF` appeared without a matching `#IF`.
    #[error("#ENDIF without matching #IF at line {line}")]
    UnmatchedEndIf {
        /// Line number of the offending command.
        line: NonZeroUsize,
    },

    /// `#ENDRANDOM` appeared without a matching `#RANDOM`.
    #[error("#ENDRANDOM without matching #RANDOM at line {line}")]
    UnmatchedEndRandom {
        /// Line number of the offending command.
        line: NonZeroUsize,
    },

    /// `#CASE` or `#DEF` appeared outside a `#SWITCH` / `#SETSWITCH` block.
    #[error("#CASE/#DEF without matching #SWITCH at line {line}")]
    UnmatchedCase {
        /// Line number of the offending command.
        line: NonZeroUsize,
    },

    /// `#ENDSW` appeared without a matching `#SWITCH`.
    #[error("#ENDSW without matching #SWITCH at line {line}")]
    UnmatchedEndSw {
        /// Line number of the offending command.
        line: NonZeroUsize,
    },

    /// Other control-flow syntax errors (e.g. `#SKIP` outside `#SWITCH`).
    #[error("{message} at line {line}")]
    UnexpectedControlFlow {
        /// Description of the error.
        message: &'static str,
        /// Line number of the offending command.
        line: NonZeroUsize,
    },
}
