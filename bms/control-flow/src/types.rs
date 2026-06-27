//! BMS 控制流树模型的核心类型。

use std::num::NonZeroUsize;

use bms_tokenizer::BmsToken;

/// 控制流块的活跃值如何确定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchValue {
    /// `#RANDOM N` / `#SWITCH N` —— 由随机数生成器在 `[1, N]` 范围内取值。
    Max(u64),
    /// `#SETRANDOM N` / `#SETSWITCH N` —— 固定值 `N`。
    Set(u64),
}

/// 在每个内容片段处携带载荷类型 `P` 的控制流树。
///
/// 连续的非控制流 token 被打包为单个载荷节点（[`FlowNode::Payload`]）；
/// 控制流命令成为结构化的 [`FlowBlock`] 节点。通过
/// [`FlowDoc::from_tokens`] 构建，再用 [`FlowDoc::select_branches`]、
/// [`FlowDoc::to_tokens`] 或 [`FlowDoc::map_payload`] 派生其他视图。
///
/// `FlowDoc<TokenPayload<C>>` 是 token 级真相源（可编辑、可 roundtrip）。
/// `FlowDoc<Bms>`（通过下游 `map_payload` 获得）是只读视图，展示每个片段
/// 解析后的聚合结果。
///
/// 内部的 `Vec<FlowNode<P>>` 可通过 `Deref`/`DerefMut` 访问 —— 切片与
/// `Vec` 的方法（`iter`、`len`、`first`、`[index]` 等）可直接在
/// `FlowDoc` 上使用。
#[derive(Debug, Clone, PartialEq)]
pub struct FlowDoc<P>(pub Vec<FlowNode<P>>);

/// [`FlowDoc`] 中的单个条目：载荷片段或控制流块。
#[derive(Debug, Clone, PartialEq)]
pub enum FlowNode<P> {
    /// 一段连续的非控制流 token，打包为载荷 `P`。
    Payload(P),
    /// 一个控制流块（`#RANDOM` 或 `#SWITCH`）。
    Block(FlowBlock<P>),
}

/// 一个控制流块。
#[derive(Debug, Clone, PartialEq)]
pub enum FlowBlock<P> {
    /// 含条件分支的 `#RANDOM` / `#SETRANDOM` 块。
    Random(RandomBlock<P>),
    /// 含 case 分支的 `#SWITCH` / `#SETSWITCH` 块。
    Switch(SwitchBlock<P>),
}

/// 一个 `#RANDOM` / `#SETRANDOM` 块及其条件分支。
#[derive(Debug, Clone, PartialEq)]
pub struct RandomBlock<P> {
    /// 分支值的确定方式（随机范围或固定值）。
    pub value: BranchValue,
    /// 闭合 `#ENDRANDOM` 是否存在。
    pub has_end_random: bool,
    /// 块内的条件分支。
    pub branches: Vec<RandomBranch<P>>,
}

/// [`RandomBlock`] 内部的单个条件分支。
#[derive(Debug, Clone, PartialEq)]
pub struct RandomBranch<P> {
    /// 分支条件的种类。
    pub kind: RandomBranchKind,
    /// 属于此分支的节点。
    pub body: Vec<FlowNode<P>>,
}

/// [`RandomBranch`] 的条件种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomBranchKind {
    /// `#IF N` —— 当随机数生成器的值等于 `N` 时匹配。
    If(u64),
    /// `#ELSEIF N` —— 在前序分支未匹配后，当随机数生成器的值等于 `N` 时匹配。
    ElseIf(u64),
    /// `#ELSE` —— 当先前所有分支都未匹配时匹配。
    Else,
}

/// 一个 `#SWITCH` / `#SETSWITCH` 块及其 case 分支。
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchBlock<P> {
    /// 分支值的确定方式（随机范围或固定值）。
    pub value: BranchValue,
    /// 此 switch 块内的 case 列表。
    pub cases: Vec<SwitchCase<P>>,
}

/// [`SwitchBlock`] 内部的单个 case。
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase<P> {
    /// case 条件的种类。
    pub kind: SwitchCaseKind,
    /// 属于此 case 的节点。
    pub body: Vec<FlowNode<P>>,
    /// 此 case 末尾是否存在 `#SKIP` 指令。
    pub has_skip: bool,
}

/// [`SwitchCase`] 的条件种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchCaseKind {
    /// `#CASE N` —— 当随机数生成器的值等于 `N` 时匹配。
    Case(u64),
    /// `#DEF` —— 默认 case，当无 `#CASE` 匹配时匹配。
    Def,
}

/// token 级载荷：保留每个片段中各 token 的原始行号。
///
/// 这是 [`FlowDoc::from_tokens`] 产生的真相源载荷。roundtrip
/// （[`FlowDoc::to_tokens`]）与分支选择（[`FlowDoc::select_branches`]）
/// 都基于此载荷类型运作。
#[derive(Debug, Clone, PartialEq)]
pub struct TokenPayload<C> {
    /// 此片段中连续的 `(line, token)` 对。
    pub tokens: Vec<(NonZeroUsize, BmsToken<C>)>,
}

/// `select_branches` 期间所做分支选择决策的记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSelection {
    /// 针对遇到的每个控制流块所作的决策。
    pub decisions: Vec<BlockDecision>,
}

/// 单个块的随机数生成器值及所选择的分支/case 索引。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDecision {
    /// 用于此块的、由随机数生成器产生的值。
    pub value: u64,
    /// 所选分支/case 在块内列表中的索引。
    pub selected_index: usize,
}

use std::ops::{Deref, DerefMut};

impl<P> Deref for FlowDoc<P> {
    type Target = [FlowNode<P>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P> DerefMut for FlowDoc<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
