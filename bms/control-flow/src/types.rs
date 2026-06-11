//! Core types for the BMS control-flow tree model.

use std::num::NonZeroUsize;

use bms_tokenizer::{BmsHeader, BmsMessage};

/// How a control-flow block's active value is determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchValue {
    /// `#RANDOM N` / `#SWITCH N` — RNG picks a value in `[1, N]`.
    Max(u64),
    /// `#SETRANDOM N` / `#SETSWITCH N` — the fixed value `N`.
    Set(u64),
}

/// Builder for constructing and processing control-flow trees.
///
/// All methods are associated functions on this zero-sized struct.
/// Use `FlowDocumentBuilder::from_tokens(...)` to build, then
/// `FlowDocumentBuilder::select_branches(&items, rng)` or
/// `FlowDocumentBuilder::to_tokens(&items)`.
pub struct FlowDocumentBuilder;

/// A single entry in a control-flow tree, carrying its original line number.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowItem<'a> {
    /// 1-based line number from the original BMS source.
    pub line: NonZeroUsize,
    /// The content of this entry.
    pub content: FlowContent<'a>,
}

/// The content of a [`FlowItem`].
#[derive(Debug, Clone, PartialEq)]
pub enum FlowContent<'a> {
    /// A header command (metadata, resource definition, etc.).
    Header(BmsHeader<'a>),
    /// A channel data line (`#xxxYY:values`).
    Message(BmsMessage<'a>),
    /// A control-flow block (`#RANDOM` or `#SWITCH`).
    Block(FlowBlock<'a>),
}

/// A control-flow block.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowBlock<'a> {
    /// A `#RANDOM` / `#SETRANDOM` block containing conditional branches.
    Random(RandomBlock<'a>),
    /// A `#SWITCH` / `#SETSWITCH` block containing case branches.
    Switch(SwitchBlock<'a>),
}

/// A `#RANDOM` / `#SETRANDOM` block with its conditional branches.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomBlock<'a> {
    /// How the branch value is determined (random range or fixed).
    pub value: BranchValue,
    /// Whether the closing `#ENDRANDOM` was present.
    pub has_end_random: bool,
    /// Conditional branches inside this block.
    pub branches: Vec<RandomBranch<'a>>,
}

/// A single conditional branch inside a [`RandomBlock`].
#[derive(Debug, Clone, PartialEq)]
pub struct RandomBranch<'a> {
    /// The kind of branch condition.
    pub kind: RandomBranchKind,
    /// Items belonging to this branch.
    pub body: Vec<FlowItem<'a>>,
}

/// The kind of condition for a [`RandomBranch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomBranchKind {
    /// `#IF N` — matches when the RNG value equals `N`.
    If(u64),
    /// `#ELSEIF N` — matches when the RNG value equals `N`, after prior branches failed.
    ElseIf(u64),
    /// `#ELSE` — matches when no prior branch matched.
    Else,
}

/// A `#SWITCH` / `#SETSWITCH` block with its case branches.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchBlock<'a> {
    /// How the branch value is determined (random range or fixed).
    pub value: BranchValue,
    /// Cases inside this switch block.
    pub cases: Vec<SwitchCase<'a>>,
}

/// A single case inside a [`SwitchBlock`].
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase<'a> {
    /// The kind of case condition.
    pub kind: SwitchCaseKind,
    /// Items belonging to this case.
    pub body: Vec<FlowItem<'a>>,
    /// Whether a `#SKIP` directive was present at the end of this case.
    pub has_skip: bool,
}

/// The kind of condition for a [`SwitchCase`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchCaseKind {
    /// `#CASE N` — matches when the RNG value equals `N`.
    Case(u64),
    /// `#DEF` — default case, matches when no `#CASE` matched.
    Def,
}

/// Record of branch-selection decisions made during `select_branches`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSelection {
    /// The decision made for each control-flow block encountered.
    pub decisions: Vec<BlockDecision>,
}

/// A single block's RNG value and which branch/case index was selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDecision {
    /// The RNG-generated value used for this block.
    pub value: u64,
    /// The index of the selected branch/case in the block's list.
    pub selected_index: usize,
}
