//! [`FlowDoc`] 的通用载荷转换。
//!
//! 这些操作遍历控制流骨架，并通过调用者提供的闭包重写每个载荷片段，
//! 保留分支/case 结构不变。下游代码可借此派生控制流 crate 自身并不知晓
//! 的载荷视图（例如由 `FlowDoc<TokenPayload>` 构建出
//! `FlowDoc<Bms>`）。

use crate::{
    FlowBlock, FlowDoc, FlowNode, RandomBlock, RandomBranch, RandomChain, SwitchBlock, SwitchCase,
};

impl<P> FlowDoc<P> {
    /// 用 `f` 转换每个载荷片段，保留控制流骨架。
    ///
    /// 每个 [`FlowNode::Payload`] 被替换为 `Payload(f(p))`；块节点保留其
    /// 分支/case，仅其内嵌的载荷会被映射。闭包按树的顺序对每个载荷片段
    /// 调用一次。
    #[must_use]
    pub fn map_payload<Q>(self, mut f: impl FnMut(P) -> Q) -> FlowDoc<Q> {
        // 映射闭包不会失败，因此用 `Infallible` 包装并对不可能出现的错误
        // 做穷尽匹配 —— 无需 `expect`/`unwrap`。
        match self.try_map_payload(|p| Ok::<Q, std::convert::Infallible>(f(p))) {
            Ok(tree) => tree,
            Err(void) => match void {},
        }
    }

    /// [`FlowDoc::map_payload`] 的可失败变体。
    ///
    /// 一旦某个片段的闭包返回 `Err` 即停止，并立即传播该错误。适用于
    /// 载荷构造可能失败的情况（例如将 token 片段解析为结构化类型）。
    ///
    /// # Errors
    ///
    /// 若 `f` 对任一片段返回 `Err(e)`，则返回 `Err(e)`；遇到第一个失败
    /// 片段即停止迭代。
    pub fn try_map_payload<Q, E>(
        self,
        mut f: impl FnMut(P) -> Result<Q, E>,
    ) -> Result<FlowDoc<Q>, E> {
        let warnings = self.warnings;
        let root = map_nodes(self.nodes, &mut f)?;
        Ok(FlowDoc {
            nodes: root,
            warnings,
        })
    }
}

/// 映射节点序列，遇到第一个错误即短路。
fn map_nodes<P, Q, E>(
    nodes: Vec<FlowNode<P>>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<Vec<FlowNode<Q>>, E> {
    nodes.into_iter().map(|node| map_node(node, f)).collect()
}

/// 映射单个节点：对载荷应用 `f`，或递归进入块。
fn map_node<P, Q, E>(
    node: FlowNode<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<FlowNode<Q>, E> {
    Ok(match node {
        FlowNode::Payload(p) => FlowNode::Payload(f(p)?),
        FlowNode::Block(block) => FlowNode::Block(map_block(block, f)?),
    })
}

/// 映射控制流块内嵌套的所有载荷。
fn map_block<P, Q, E>(
    block: FlowBlock<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<FlowBlock<Q>, E> {
    Ok(match block {
        FlowBlock::Random(r) => FlowBlock::Random(RandomBlock {
            value: r.value,
            has_end_random: r.has_end_random,
            chains: r
                .chains
                .into_iter()
                .map(|chain| map_random_chain(chain, f))
                .collect::<Result<Vec<_>, E>>()?,
        }),
        FlowBlock::Switch(s) => FlowBlock::Switch(SwitchBlock {
            value: s.value,
            cases: s
                .cases
                .into_iter()
                .map(|case| map_switch_case(case, f))
                .collect::<Result<Vec<_>, E>>()?,
        }),
    })
}

/// 映射单条互斥分支链内的载荷。
fn map_random_chain<P, Q, E>(
    chain: RandomChain<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<RandomChain<Q>, E> {
    Ok(RandomChain {
        branches: chain
            .branches
            .into_iter()
            .map(|branch| map_random_branch(branch, f))
            .collect::<Result<Vec<_>, E>>()?,
    })
}

/// 映射单个 `RandomBranch` 内的载荷。
fn map_random_branch<P, Q, E>(
    branch: RandomBranch<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<RandomBranch<Q>, E> {
    Ok(RandomBranch {
        kind: branch.kind,
        body: map_nodes(branch.body, f)?,
    })
}

/// 映射单个 `SwitchCase` 内的载荷。
fn map_switch_case<P, Q, E>(
    case: SwitchCase<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<SwitchCase<Q>, E> {
    Ok(SwitchCase {
        kind: case.kind,
        body: map_nodes(case.body, f)?,
        has_skip: case.has_skip,
    })
}
