//! Core types for the BMS control-flow tree model.

use std::num::NonZeroUsize;

use bms_tokenizer::BmsToken;

/// How a control-flow block's active value is determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchValue {
    /// `#RANDOM N` / `#SWITCH N` — RNG picks a value in `[1, N]`.
    Max(u64),
    /// `#SETRANDOM N` / `#SETSWITCH N` — the fixed value `N`.
    Set(u64),
}

/// A control-flow tree carrying payload type `P` at each content span.
///
/// Consecutive non-control-flow tokens are packed into a single payload
/// node ([`FlowNode::Payload`]); control-flow commands become structured
/// [`FlowBlock`] nodes. Build with [`FlowTree::from_tokens`], then use
/// [`FlowTree::select_branches`], [`FlowTree::to_tokens`], or
/// [`FlowTree::map_payload`] to derive other views.
///
/// `FlowTree<TokenPayload<C>>` is the token-level source of truth (editable,
/// roundtrippable). `FlowTree<Bms>` (obtained via `map_payload` downstream) is
/// a read-only view showing each span's parsed aggregate.
///
/// The inner `Vec<FlowNode<P>>` is accessible via `Deref`/`DerefMut` — slice
/// and `Vec` methods (`iter`, `len`, `first`, `[index]`, …) work directly on
/// `FlowTree`.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowTree<P>(pub Vec<FlowNode<P>>);

/// A single entry in a [`FlowTree`]: either a payload span or a control-flow block.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowNode<P> {
    /// A span of consecutive non-control-flow tokens, packed into payload `P`.
    Payload(P),
    /// A control-flow block (`#RANDOM` or `#SWITCH`).
    Block(FlowBlock<P>),
}

/// A control-flow block.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowBlock<P> {
    /// A `#RANDOM` / `#SETRANDOM` block containing conditional branches.
    Random(RandomBlock<P>),
    /// A `#SWITCH` / `#SETSWITCH` block containing case branches.
    Switch(SwitchBlock<P>),
}

/// A `#RANDOM` / `#SETRANDOM` block with its conditional branches.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomBlock<P> {
    /// How the branch value is determined (random range or fixed).
    pub value: BranchValue,
    /// Whether the closing `#ENDRANDOM` was present.
    pub has_end_random: bool,
    /// Conditional branches inside this block.
    pub branches: Vec<RandomBranch<P>>,
}

/// A single conditional branch inside a [`RandomBlock`].
#[derive(Debug, Clone, PartialEq)]
pub struct RandomBranch<P> {
    /// The kind of branch condition.
    pub kind: RandomBranchKind,
    /// Nodes belonging to this branch.
    pub body: Vec<FlowNode<P>>,
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
pub struct SwitchBlock<P> {
    /// How the branch value is determined (random range or fixed).
    pub value: BranchValue,
    /// Cases inside this switch block.
    pub cases: Vec<SwitchCase<P>>,
}

/// A single case inside a [`SwitchBlock`].
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase<P> {
    /// The kind of case condition.
    pub kind: SwitchCaseKind,
    /// Nodes belonging to this case.
    pub body: Vec<FlowNode<P>>,
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

/// Token-level payload: preserves each token of a span with its original line.
///
/// This is the source-of-truth payload produced by [`FlowTree::from_tokens`].
/// Roundtrip ([`FlowTree::to_tokens`]) and branch selection
/// ([`FlowTree::select_branches`]) operate on this payload kind.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenPayload<C> {
    /// The consecutive `(line, token)` pairs in this span.
    pub tokens: Vec<(NonZeroUsize, BmsToken<C>)>,
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

use std::ops::{Deref, DerefMut};

impl<P> Deref for FlowTree<P> {
    type Target = [FlowNode<P>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P> DerefMut for FlowTree<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
