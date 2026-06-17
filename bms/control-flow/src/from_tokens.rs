//! Build a control-flow tree from a flat token stream.

use std::num::NonZeroUsize;

use bms_tokenizer::{BmsHeader, BmsHeaderControlFlow, BmsToken};

use crate::{
    BranchValue, ControlFlowError, FlowBlock, FlowDoc, FlowNode, RandomBlock, RandomBranch,
    RandomBranchKind, SwitchBlock, SwitchCase, SwitchCaseKind, TokenPayload,
};

/// Pending `(line, token)` pairs not yet packed into a payload node.
type Pending<C> = Vec<(NonZeroUsize, BmsToken<C>)>;

/// Internal state for building a `RandomBlock`.
struct RandomState<C: Clone + PartialEq> {
    /// How the branch value is determined.
    value: BranchValue,
    /// Whether `#ENDRANDOM` was seen.
    has_end_random: bool,
    /// Completed branches.
    branches: Vec<RandomBranch<TokenPayload<C>>>,
    /// Branch currently being built.
    current_branch: Option<RandomBranch<TokenPayload<C>>>,
    /// Tokens accumulated for the current branch, not yet packed.
    pending: Pending<C>,
}

/// Internal state for building a `SwitchBlock`.
struct SwitchState<C: Clone + PartialEq> {
    /// How the branch value is determined.
    value: BranchValue,
    /// Completed cases.
    cases: Vec<SwitchCase<TokenPayload<C>>>,
    /// Case currently being built.
    current_case: Option<SwitchCase<TokenPayload<C>>>,
    /// Tokens accumulated for the current case, not yet packed.
    pending: Pending<C>,
}

/// Stack entry for nested block construction.
enum StackEntry<C: Clone + PartialEq> {
    /// Currently building a `#RANDOM` block.
    Random(RandomState<C>),
    /// Currently building a `#SWITCH` block.
    Switch(SwitchState<C>),
}

/// Accumulator packing a flat token stream into a [`FlowDoc`].
///
/// Consecutive non-control-flow tokens are buffered in a per-scope `pending`
/// list and flushed into a [`FlowNode::Payload`] whenever a control-flow
/// boundary is crossed (branch/case switch, block close, or end of stream).
struct Builder<C: Clone + PartialEq> {
    /// Completed top-level nodes.
    top_level: Vec<FlowNode<TokenPayload<C>>>,
    /// Tokens accumulated at top level, not yet packed.
    top_pending: Pending<C>,
    /// Nested block stack.
    stack: Vec<StackEntry<C>>,
}

impl<C: Clone + PartialEq> Builder<C> {
    /// Create an empty builder.
    fn new() -> Self {
        Self {
            top_level: Vec::new(),
            top_pending: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Flush any remaining pending tokens and yield the finished tree.
    fn finish(mut self) -> FlowDoc<TokenPayload<C>> {
        self.flush();
        FlowDoc(self.top_level)
    }

    /// Buffer a non-control-flow token into the current scope's pending list.
    fn push_token(&mut self, line: NonZeroUsize, token: BmsToken<C>) {
        match self.stack.last_mut() {
            Some(StackEntry::Random(state)) => state.pending.push((line, token)),
            Some(StackEntry::Switch(state)) => state.pending.push((line, token)),
            None => self.top_pending.push((line, token)),
        }
    }

    /// Pack the current scope's pending tokens into a payload node, if non-empty.
    fn flush(&mut self) {
        let pending = match self.stack.last_mut() {
            Some(StackEntry::Random(state)) => std::mem::take(&mut state.pending),
            Some(StackEntry::Switch(state)) => std::mem::take(&mut state.pending),
            None => std::mem::take(&mut self.top_pending),
        };
        if !pending.is_empty() {
            self.push_node(FlowNode::Payload(TokenPayload { tokens: pending }));
        }
    }

    /// Push an already-built node to the current scope's body.
    fn push_node(&mut self, node: FlowNode<TokenPayload<C>>) {
        match self.stack.last_mut() {
            Some(StackEntry::Random(state)) => {
                if let Some(branch) = state.current_branch.as_mut() {
                    branch.body.push(node);
                }
            }
            Some(StackEntry::Switch(state)) => {
                if let Some(case) = state.current_case.as_mut() {
                    case.body.push(node);
                }
            }
            None => self.top_level.push(node),
        }
    }

    /// Route a control-flow header to the appropriate structural operation.
    fn handle_control_flow(
        &mut self,
        line: NonZeroUsize,
        cf: &BmsHeaderControlFlow,
    ) -> Result<(), ControlFlowError> {
        match cf {
            BmsHeaderControlFlow::Random(max) => {
                self.flush();
                self.stack.push(StackEntry::Random(RandomState {
                    value: BranchValue::Max(*max),
                    has_end_random: false,
                    branches: Vec::new(),
                    current_branch: None,
                    pending: Vec::new(),
                }));
            }
            BmsHeaderControlFlow::SetRandom(max) => {
                self.flush();
                self.stack.push(StackEntry::Random(RandomState {
                    value: BranchValue::Set(*max),
                    has_end_random: false,
                    branches: Vec::new(),
                    current_branch: None,
                    pending: Vec::new(),
                }));
            }
            BmsHeaderControlFlow::If(v) => self.start_branch(line, RandomBranchKind::If(*v))?,
            BmsHeaderControlFlow::ElseIf(v) => {
                self.start_branch(line, RandomBranchKind::ElseIf(*v))?;
            }
            BmsHeaderControlFlow::Else => self.start_branch(line, RandomBranchKind::Else)?,
            BmsHeaderControlFlow::EndIf => {
                let idx = self
                    .find_random()
                    .ok_or(ControlFlowError::UnmatchedEndIf { line })?;
                self.finalize_random_branch(idx);
            }
            BmsHeaderControlFlow::EndRandom => {
                let idx = self
                    .find_random()
                    .ok_or(ControlFlowError::UnmatchedEndRandom { line })?;
                self.finalize_random_branch(idx);
                if let Some(StackEntry::Random(state)) = self.stack.get_mut(idx) {
                    state.has_end_random = true;
                }
                self.pop_and_build(idx);
            }
            BmsHeaderControlFlow::Switch(max) => {
                self.flush();
                self.stack.push(StackEntry::Switch(SwitchState {
                    value: BranchValue::Max(*max),
                    cases: Vec::new(),
                    current_case: None,
                    pending: Vec::new(),
                }));
            }
            BmsHeaderControlFlow::SetSwitch(max) => {
                self.flush();
                self.stack.push(StackEntry::Switch(SwitchState {
                    value: BranchValue::Set(*max),
                    cases: Vec::new(),
                    current_case: None,
                    pending: Vec::new(),
                }));
            }
            BmsHeaderControlFlow::EndSwitch => {
                let idx = self
                    .find_switch()
                    .ok_or(ControlFlowError::UnmatchedEndSw { line })?;
                self.finalize_switch_case(idx);
                self.pop_and_build(idx);
            }
            BmsHeaderControlFlow::Case(v) => self.start_case(line, SwitchCaseKind::Case(*v))?,
            BmsHeaderControlFlow::Def => self.start_case(line, SwitchCaseKind::Def)?,
            BmsHeaderControlFlow::Skip(_) => {
                let idx = self
                    .find_switch()
                    .ok_or(ControlFlowError::UnexpectedControlFlow {
                        message: "#SKIP without matching #SWITCH",
                        line,
                    })?;
                if let Some(StackEntry::Switch(state)) = self.stack.get_mut(idx) {
                    if let Some(case) = state.current_case.as_mut() {
                        case.has_skip = true;
                    }
                }
            }
        }
        Ok(())
    }

    /// Start a new branch in the nearest `Random` block.
    fn start_branch(
        &mut self,
        line: NonZeroUsize,
        kind: RandomBranchKind,
    ) -> Result<(), ControlFlowError> {
        let idx = self
            .find_random()
            .ok_or(ControlFlowError::UnmatchedIf { line })?;
        self.finalize_random_branch(idx);
        if let Some(StackEntry::Random(state)) = self.stack.get_mut(idx) {
            state.current_branch = Some(RandomBranch {
                kind,
                body: Vec::new(),
            });
        }
        Ok(())
    }

    /// Start a new case in the nearest `Switch` block.
    fn start_case(
        &mut self,
        line: NonZeroUsize,
        kind: SwitchCaseKind,
    ) -> Result<(), ControlFlowError> {
        let idx = self
            .find_switch()
            .ok_or(ControlFlowError::UnmatchedCase { line })?;
        self.finalize_switch_case(idx);
        if let Some(StackEntry::Switch(state)) = self.stack.get_mut(idx) {
            state.current_case = Some(SwitchCase {
                kind,
                body: Vec::new(),
                has_skip: false,
            });
        }
        Ok(())
    }

    /// Pack pending tokens into the current branch and move it to `branches`.
    fn finalize_random_branch(&mut self, idx: usize) {
        if let Some(StackEntry::Random(state)) = self.stack.get_mut(idx) {
            flush_into(
                &mut state.pending,
                state.current_branch.as_mut().map(|b| &mut b.body),
            );
            if let Some(branch) = state.current_branch.take() {
                state.branches.push(branch);
            }
        }
    }

    /// Pack pending tokens into the current case and move it to `cases`.
    fn finalize_switch_case(&mut self, idx: usize) {
        if let Some(StackEntry::Switch(state)) = self.stack.get_mut(idx) {
            flush_into(
                &mut state.pending,
                state.current_case.as_mut().map(|c| &mut c.body),
            );
            if let Some(case) = state.current_case.take() {
                state.cases.push(case);
            }
        }
    }

    /// Index of the nearest `Random` entry searching from stack top.
    fn find_random(&self) -> Option<usize> {
        self.stack
            .iter()
            .rposition(|e| matches!(e, StackEntry::Random(_)))
    }

    /// Index of the nearest `Switch` entry searching from stack top.
    fn find_switch(&self) -> Option<usize> {
        self.stack
            .iter()
            .rposition(|e| matches!(e, StackEntry::Switch(_)))
    }

    /// Pop entries from `idx` to the top, building `FlowBlock`s into the parent scope.
    fn pop_and_build(&mut self, idx: usize) {
        let removed: Vec<StackEntry<C>> = self.stack.drain(idx..).collect();
        for entry in removed {
            let block = match entry {
                StackEntry::Random(mut state) => {
                    flush_into(
                        &mut state.pending,
                        state.current_branch.as_mut().map(|b| &mut b.body),
                    );
                    if let Some(branch) = state.current_branch.take() {
                        state.branches.push(branch);
                    }
                    FlowBlock::Random(RandomBlock {
                        value: state.value,
                        has_end_random: state.has_end_random,
                        branches: state.branches,
                    })
                }
                StackEntry::Switch(mut state) => {
                    flush_into(
                        &mut state.pending,
                        state.current_case.as_mut().map(|c| &mut c.body),
                    );
                    if let Some(case) = state.current_case.take() {
                        state.cases.push(case);
                    }
                    FlowBlock::Switch(SwitchBlock {
                        value: state.value,
                        cases: state.cases,
                    })
                }
            };
            self.push_node(FlowNode::Block(block));
        }
    }
}

/// Pack `pending` into a payload node appended to `body_owner`, if non-empty.
///
/// Generic over `RandomBranch` / `SwitchCase` (both carry `body: Vec<FlowNode<_>>`).
fn flush_into<C: Clone + PartialEq>(
    pending: &mut Pending<C>,
    body_owner: Option<&mut Vec<FlowNode<TokenPayload<C>>>>,
) {
    if pending.is_empty() {
        return;
    }
    let Some(body) = body_owner else { return };
    let tokens = std::mem::take(pending);
    body.push(FlowNode::Payload(TokenPayload { tokens }));
}

impl<C: Clone + PartialEq> FlowDoc<TokenPayload<C>> {
    /// Build a [`FlowDoc`] from an iterator of `(line, token)` pairs.
    ///
    /// Non-control-flow tokens are packed into consecutive payload spans.
    /// Control-flow tokens (`#RANDOM`, `#SWITCH`, etc.) are structured into
    /// nested [`FlowBlock`] trees.
    ///
    /// # Errors
    ///
    /// Returns [`ControlFlowError`] when control-flow commands are mis-nested
    /// (e.g., `#IF` without `#RANDOM`, `#ENDRANDOM` without matching open).
    pub fn from_tokens(
        tokens: impl IntoIterator<Item = (NonZeroUsize, BmsToken<C>)>,
    ) -> Result<Self, ControlFlowError> {
        let mut builder = Builder::new();
        for (line, token) in tokens {
            match token {
                BmsToken::Header(BmsHeader::ControlFlow(cf)) => {
                    builder.handle_control_flow(line, &cf)?;
                }
                other => builder.push_token(line, other),
            }
        }
        Ok(builder.finish())
    }
}
