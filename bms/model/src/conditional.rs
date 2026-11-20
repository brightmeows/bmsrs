//! `KaleidBeat` runtime conditional blocks.
//!
//! A block switches the following channel rows to the branch whose live-state
//! condition matches first. The player evaluates the block; the model only
//! preserves its structure for the writer and the processor sidecar.

use bmsrs_bms_values::{BmsCondition, BmsMessage};

/// One `#CONDITIONAL` block.
///
/// `KaleidBeat` extension: the block syntax is not defined by the BMS
/// specification or beatoraja.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBlock {
    /// Evaluation position in measures; the fractional part is inside the
    /// measure.
    pub position: f64,
    /// Branches in source order; the branch with no condition is `#DEFAULT`.
    pub branches: Vec<ConditionalBranch>,
}

/// One branch inside a [`ConditionalBlock`].
///
/// `KaleidBeat` extension: not defined by the BMS specification or beatoraja.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBranch {
    /// Comparison for `#WHEN` / `#ELSEWHEN`; `None` for `#DEFAULT`.
    pub condition: Option<BmsCondition>,
    /// Channel rows belonging to this branch.
    ///
    /// Header commands are global in `KaleidBeat`; a header inside a branch is
    /// still applied to the whole chart and is reported by the analysis layer.
    pub messages: Vec<BmsMessage>,
}
