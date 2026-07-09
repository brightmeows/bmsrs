//! 从扁平 token 流构建控制流树。

use std::num::NonZeroUsize;

use bms_tokenizer::{BmsHeader, BmsHeaderControlFlow, BmsToken};
use itertools::Itertools as _;

use crate::{
    BranchValue, ControlFlowError, FlowBlock, FlowDoc, FlowNode, RandomBlock, RandomBranch,
    RandomBranchKind, SwitchBlock, SwitchCase, SwitchCaseKind, TokenPayload,
};

/// 尚未打包为载荷节点的待定 `(line, token)` 对。
type Pending<C> = Vec<(NonZeroUsize, BmsToken<C>)>;

/// 构建 `RandomBlock` 的内部状态。
struct RandomState<C: Clone + PartialEq> {
    /// 分支值的确定方式。
    value: BranchValue,
    /// 是否已遇到 `#ENDRANDOM`。
    has_end_random: bool,
    /// 已完成的分支。
    branches: Vec<RandomBranch<TokenPayload<C>>>,
    /// 正在构建的分支。
    current_branch: Option<RandomBranch<TokenPayload<C>>>,
    /// 当前分支累积、尚未打包的 token。
    pending: Pending<C>,
}

/// 构建 `SwitchBlock` 的内部状态。
struct SwitchState<C: Clone + PartialEq> {
    /// 分支值的确定方式。
    value: BranchValue,
    /// 已完成的 case。
    cases: Vec<SwitchCase<TokenPayload<C>>>,
    /// 正在构建的 case。
    current_case: Option<SwitchCase<TokenPayload<C>>>,
    /// 当前 case 累积、尚未打包的 token。
    pending: Pending<C>,
}

/// 嵌套块构造的栈条目。
enum StackEntry<C: Clone + PartialEq> {
    /// 正在构建一个 `#RANDOM` 块。
    Random(RandomState<C>),
    /// 正在构建一个 `#SWITCH` 块。
    Switch(SwitchState<C>),
}

/// 将扁平 token 流打包为 [`FlowDoc`] 的累加器。
///
/// 连续的非控制流 token 缓存在按作用域划分的 `pending` 列表中，每当越过
/// 控制流边界（分支/case 切换、块闭合或流结束）时，就刷新为一个
/// [`FlowNode::Payload`]。
struct Builder<C: Clone + PartialEq> {
    /// 已完成的顶层节点。
    top_level: Vec<FlowNode<TokenPayload<C>>>,
    /// 顶层累积、尚未打包的 token。
    top_pending: Pending<C>,
    /// 嵌套块栈。
    stack: Vec<StackEntry<C>>,
}

impl<C: Clone + PartialEq> Builder<C> {
    /// 创建一个空的构建器。
    const fn new() -> Self {
        Self {
            top_level: Vec::new(),
            top_pending: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// 刷新所有剩余待定 token 并产出完成的树。
    fn finish(mut self) -> FlowDoc<TokenPayload<C>> {
        self.flush();
        FlowDoc(self.top_level)
    }

    /// 将非控制流 token 缓存到当前作用域的待定列表。
    fn push_token(&mut self, line: NonZeroUsize, token: BmsToken<C>) {
        match self.stack.last_mut() {
            Some(StackEntry::Random(state)) => state.pending.push((line, token)),
            Some(StackEntry::Switch(state)) => state.pending.push((line, token)),
            None => self.top_pending.push((line, token)),
        }
    }

    /// 若待定 token 非空，则将当前作用域的待定 token 打包为载荷节点。
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

    /// 将已构建好的节点压入当前作用域的 body。
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

    /// 将一个控制流头部命令路由到对应的结构化操作。
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
            BmsHeaderControlFlow::Skip => {
                let idx = self
                    .find_switch()
                    .ok_or(ControlFlowError::UnexpectedControlFlow {
                        message: "#SKIP without matching #SWITCH",
                        line,
                    })?;
                if let Some(StackEntry::Switch(state)) = self.stack.get_mut(idx)
                    && let Some(case) = state.current_case.as_mut()
                {
                    case.has_skip = true;
                }
            }
        }
        Ok(())
    }

    /// 在最近的 `Random` 块中开启一个新分支。
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

    /// 在最近的 `Switch` 块中开启一个新 case。
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

    /// 将待定 token 打包到当前分支中，并将其移入 `branches`。
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

    /// 将待定 token 打包到当前 case 中，并将其移入 `cases`。
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

    /// 从栈顶向下搜索，返回最近的 `Random` 条目的索引。
    fn find_random(&self) -> Option<usize> {
        self.stack
            .iter()
            .rposition(|e| matches!(e, StackEntry::Random(_)))
    }

    /// 从栈顶向下搜索，返回最近的 `Switch` 条目的索引。
    fn find_switch(&self) -> Option<usize> {
        self.stack
            .iter()
            .rposition(|e| matches!(e, StackEntry::Switch(_)))
    }

    /// 弹出从 `idx` 到栈顶的所有条目，将构建好的 `FlowBlock` 压入父作用域。
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

/// 若 `pending` 非空，将其打包为载荷节点并追加到 `body_owner`。
///
/// 对 `RandomBranch` / `SwitchCase` 泛型化（两者都持有
/// `body: Vec<FlowNode<_>>`）。
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
    /// 从 `(line, token)` 对的迭代器构建 [`FlowDoc`]。
    ///
    /// 非控制流 token 被打包为连续的载荷片段。控制流 token（`#RANDOM`、
    /// `#SWITCH` 等）被组织为嵌套的 [`FlowBlock`] 树。
    ///
    /// # Errors
    ///
    /// 当控制流命令嵌套错误时（例如 `#IF` 没有 `#RANDOM`、`#ENDRANDOM`
    /// 没有匹配的起始命令），返回 [`ControlFlowError`]。
    pub fn from_tokens(
        tokens: impl IntoIterator<Item = (NonZeroUsize, BmsToken<C>)>,
    ) -> Result<Self, ControlFlowError> {
        let mut builder = Builder::new();
        tokens
            .into_iter()
            .map(|(line, token)| -> Result<(), ControlFlowError> {
                match token {
                    BmsToken::Header(BmsHeader::ControlFlow(cf)) => {
                        builder.handle_control_flow(line, &cf)
                    }
                    other => {
                        builder.push_token(line, other);
                        Ok(())
                    }
                }
            })
            .process_results(|iter| iter.for_each(drop))?;
        Ok(builder.finish())
    }
}
