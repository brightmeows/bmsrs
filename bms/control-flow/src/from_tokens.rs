//! Build a control-flow tree from a flat token stream.

use std::num::NonZeroUsize;

use bms_tokenizer::{BmsHeader, BmsHeaderControlFlow, BmsToken};

use crate::{
    BranchValue, ControlFlowError, FlowBlock, FlowContent, FlowDocumentBuilder, FlowItem,
    RandomBlock, RandomBranch, RandomBranchKind, SwitchBlock, SwitchCase, SwitchCaseKind,
};

/// Internal state for building a `RandomBlock`.
struct RandomState<C: Clone + PartialEq> {
    /// Line number of the opening `#RANDOM` / `#SETRANDOM`.
    open_line: NonZeroUsize,
    /// How the branch value is determined.
    value: BranchValue,
    /// Whether `#ENDRANDOM` was seen.
    has_end_random: bool,
    /// Completed branches.
    branches: Vec<RandomBranch<C>>,
    /// Branch currently being built.
    current_branch: Option<RandomBranch<C>>,
}

/// Internal state for building a `SwitchBlock`.
struct SwitchState<C: Clone + PartialEq> {
    /// Line number of the opening `#SWITCH` / `#SETSWITCH`.
    open_line: NonZeroUsize,
    /// How the branch value is determined.
    value: BranchValue,
    /// Completed cases.
    cases: Vec<SwitchCase<C>>,
    /// Case currently being built.
    current_case: Option<SwitchCase<C>>,
}

/// Stack entry for nested block construction.
enum StackEntry<C: Clone + PartialEq> {
    /// Currently building a `#RANDOM` block.
    Random(RandomState<C>),
    /// Currently building a `#SWITCH` block.
    Switch(SwitchState<C>),
}

impl FlowDocumentBuilder {
    /// Build a [`Vec<FlowItem>`] from an iterator of `(line, token)` pairs.
    ///
    /// Non-control-flow tokens are preserved as top-level items.
    /// Control-flow tokens (`#RANDOM`, `#SWITCH`, etc.) are structured into
    /// nested [`FlowBlock`] trees.
    ///
    /// # Errors
    ///
    /// Returns [`ControlFlowError`] when control-flow commands are mis-nested
    /// (e.g., `#IF` without `#RANDOM`, `#ENDRANDOM` without matching open).
    pub fn from_tokens<C: Clone + PartialEq>(
        tokens: impl IntoIterator<Item = (NonZeroUsize, BmsToken<C>)>,
    ) -> Result<Vec<FlowItem<C>>, ControlFlowError> {
        let mut top_level: Vec<FlowItem<C>> = Vec::new();
        let mut stack: Vec<StackEntry<C>> = Vec::new();

        for (line, token) in tokens {
            match token {
                BmsToken::Header(BmsHeader::ControlFlow(cf)) => {
                    handle_control_flow(&mut top_level, &mut stack, line, &cf)?;
                }
                other => {
                    push_to_scope(&mut top_level, &mut stack, line, other);
                }
            }
        }

        Ok(top_level)
    }
}

/// Push a non-control-flow token to the current scope.
fn push_to_scope<C: Clone + PartialEq>(
    top_level: &mut Vec<FlowItem<C>>,
    stack: &mut [StackEntry<C>],
    line: NonZeroUsize,
    token: BmsToken<C>,
) {
    let item = FlowItem {
        line,
        content: match token {
            BmsToken::Header(h) => FlowContent::Header(h),
            BmsToken::Message(m) => FlowContent::Message(m),
        },
    };

    match stack.last_mut() {
        Some(StackEntry::Random(state)) => {
            if let Some(branch) = state.current_branch.as_mut() {
                branch.body.push(item);
            }
        }
        Some(StackEntry::Switch(state)) => {
            if let Some(case) = state.current_case.as_mut() {
                case.body.push(item);
            }
        }
        None => {
            top_level.push(item);
        }
    }
}

/// Handle a control-flow header.
fn handle_control_flow<C: Clone + PartialEq>(
    top_level: &mut Vec<FlowItem<C>>,
    stack: &mut Vec<StackEntry<C>>,
    line: NonZeroUsize,
    cf: &BmsHeaderControlFlow,
) -> Result<(), ControlFlowError> {
    match cf {
        BmsHeaderControlFlow::Random(max) => {
            stack.push(StackEntry::Random(RandomState {
                open_line: line,
                value: BranchValue::Max(*max),
                has_end_random: false,
                branches: Vec::new(),
                current_branch: None,
            }));
        }
        BmsHeaderControlFlow::SetRandom(max) => {
            stack.push(StackEntry::Random(RandomState {
                open_line: line,
                value: BranchValue::Set(*max),
                has_end_random: false,
                branches: Vec::new(),
                current_branch: None,
            }));
        }
        BmsHeaderControlFlow::If(v) => start_new_branch(stack, line, RandomBranchKind::If(*v))?,
        BmsHeaderControlFlow::ElseIf(v) => {
            start_new_branch(stack, line, RandomBranchKind::ElseIf(*v))?;
        }
        BmsHeaderControlFlow::Else => start_new_branch(stack, line, RandomBranchKind::Else)?,
        BmsHeaderControlFlow::EndIf => {
            let idx = find_random(stack).ok_or(ControlFlowError::UnmatchedEndIf { line })?;
            finalize_random_branch_at(stack, idx);
            let _ = idx;
        }
        BmsHeaderControlFlow::EndRandom => {
            let idx = find_random(stack).ok_or(ControlFlowError::UnmatchedEndRandom { line })?;
            finalize_random_branch_at(stack, idx);
            let Some(StackEntry::Random(state)) = stack.get_mut(idx) else {
                return Err(ControlFlowError::UnmatchedEndRandom { line });
            };
            state.has_end_random = true;
            pop_and_build(top_level, stack, idx);
        }
        BmsHeaderControlFlow::Switch(max) => {
            stack.push(StackEntry::Switch(SwitchState {
                open_line: line,
                value: BranchValue::Max(*max),
                cases: Vec::new(),
                current_case: None,
            }));
        }
        BmsHeaderControlFlow::SetSwitch(max) => {
            stack.push(StackEntry::Switch(SwitchState {
                open_line: line,
                value: BranchValue::Set(*max),
                cases: Vec::new(),
                current_case: None,
            }));
        }
        BmsHeaderControlFlow::EndSwitch => {
            let idx = find_switch(stack).ok_or(ControlFlowError::UnmatchedEndSw { line })?;
            finalize_switch_case_at(stack, idx);
            pop_and_build(top_level, stack, idx);
        }
        BmsHeaderControlFlow::Case(v) => start_new_case(stack, line, SwitchCaseKind::Case(*v))?,
        BmsHeaderControlFlow::Def => start_new_case(stack, line, SwitchCaseKind::Def)?,
        BmsHeaderControlFlow::Skip(_) => {
            let idx = find_switch(stack).ok_or(ControlFlowError::UnexpectedControlFlow {
                message: "#SKIP without matching #SWITCH",
                line,
            })?;
            let Some(StackEntry::Switch(state)) = stack.get_mut(idx) else {
                return Err(ControlFlowError::UnexpectedControlFlow {
                    message: "#SKIP without matching #SWITCH",
                    line,
                });
            };
            if let Some(case) = state.current_case.as_mut() {
                case.has_skip = true;
            }
        }
    }
    Ok(())
}

/// Find the index of the nearest `Random` entry searching from stack top.
fn find_random<C: Clone + PartialEq>(stack: &[StackEntry<C>]) -> Option<usize> {
    stack
        .iter()
        .rposition(|e| matches!(e, StackEntry::Random(_)))
}

/// Find the index of the nearest `Switch` entry searching from stack top.
fn find_switch<C: Clone + PartialEq>(stack: &[StackEntry<C>]) -> Option<usize> {
    stack
        .iter()
        .rposition(|e| matches!(e, StackEntry::Switch(_)))
}

/// Finalize the current branch of the `Random` at `idx`.
fn finalize_random_branch_at<C: Clone + PartialEq>(stack: &mut [StackEntry<C>], idx: usize) {
    if let Some(StackEntry::Random(state)) = stack.get_mut(idx) {
        if let Some(branch) = state.current_branch.take() {
            state.branches.push(branch);
        }
    }
}

/// Finalize the current case of the `Switch` at `idx`.
fn finalize_switch_case_at<C: Clone + PartialEq>(stack: &mut [StackEntry<C>], idx: usize) {
    if let Some(StackEntry::Switch(state)) = stack.get_mut(idx) {
        if let Some(case) = state.current_case.take() {
            state.cases.push(case);
        }
    }
}

/// Start a new branch in the nearest `Random` block.
///
/// Finalizes any open branch first, then sets `current_branch` to a new
/// empty branch with the given `kind`.
fn start_new_branch<C: Clone + PartialEq>(
    stack: &mut [StackEntry<C>],
    line: NonZeroUsize,
    kind: RandomBranchKind,
) -> Result<(), ControlFlowError> {
    let idx = find_random(stack).ok_or(ControlFlowError::UnmatchedIf { line })?;
    finalize_random_branch_at(stack, idx);
    let Some(StackEntry::Random(state)) = stack.get_mut(idx) else {
        return Err(ControlFlowError::UnmatchedIf { line });
    };
    state.current_branch = Some(RandomBranch {
        kind,
        body: Vec::new(),
    });
    Ok(())
}

/// Start a new case in the nearest `Switch` block.
///
/// Finalizes any open case first, then sets `current_case` to a new
/// empty case with the given `kind`.
fn start_new_case<C: Clone + PartialEq>(
    stack: &mut [StackEntry<C>],
    line: NonZeroUsize,
    kind: SwitchCaseKind,
) -> Result<(), ControlFlowError> {
    let idx = find_switch(stack).ok_or(ControlFlowError::UnmatchedCase { line })?;
    finalize_switch_case_at(stack, idx);
    let Some(StackEntry::Switch(state)) = stack.get_mut(idx) else {
        return Err(ControlFlowError::UnmatchedCase { line });
    };
    state.current_case = Some(SwitchCase {
        kind,
        body: Vec::new(),
        has_skip: false,
    });
    Ok(())
}

/// Pop entries from `idx` to the top, building `FlowBlock`s.
///
/// Each popped entry is converted to a `FlowBlock` and added to its parent
/// scope (the new stack top, or top-level if the stack is empty after popping).
fn pop_and_build<C: Clone + PartialEq>(
    top_level: &mut Vec<FlowItem<C>>,
    stack: &mut Vec<StackEntry<C>>,
    idx: usize,
) {
    let removed: Vec<StackEntry<C>> = stack.drain(idx..).collect();

    for entry in removed {
        let item = match entry {
            StackEntry::Random(state) => FlowItem {
                line: state.open_line,
                content: FlowContent::Block(FlowBlock::Random(RandomBlock {
                    value: state.value,
                    has_end_random: state.has_end_random,
                    branches: state.branches,
                })),
            },
            StackEntry::Switch(state) => FlowItem {
                line: state.open_line,
                content: FlowContent::Block(FlowBlock::Switch(SwitchBlock {
                    value: state.value,
                    cases: state.cases,
                })),
            },
        };

        // Add to parent scope
        if stack.is_empty() {
            top_level.push(item);
        } else {
            match stack.last_mut() {
                Some(StackEntry::Random(state)) => {
                    if let Some(branch) = state.current_branch.as_mut() {
                        branch.body.push(item);
                    }
                }
                Some(StackEntry::Switch(state)) => {
                    if let Some(case) = state.current_case.as_mut() {
                        case.body.push(item);
                    }
                }
                None => top_level.push(item),
            }
        }
    }
}
