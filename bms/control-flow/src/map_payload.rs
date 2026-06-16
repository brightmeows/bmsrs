//! Generic payload transformation for [`FlowTree`].
//!
//! These operations walk the control-flow skeleton and rewrite every payload
//! span via a caller-supplied closure, leaving the branch/case structure
//! untouched. They let downstream code derive payload views the control-flow
//! crate itself does not know about (e.g. `FlowTree<Bms>` built from
//! `FlowTree<TokenPayload<C>>`).

use crate::{FlowBlock, FlowNode, FlowTree, RandomBlock, RandomBranch, SwitchBlock, SwitchCase};

impl<P> FlowTree<P> {
    /// Transform every payload span by `f`, preserving the control-flow skeleton.
    ///
    /// Each [`FlowNode::Payload`] is replaced by `Payload(f(p))`; block nodes
    /// keep their branches/cases and only their nested payloads are mapped.
    /// The closure runs once per payload span in tree order.
    #[must_use]
    pub fn map_payload<Q>(self, mut f: impl FnMut(P) -> Q) -> FlowTree<Q> {
        // The mapping closure is infallible, so wrap it with `Infallible` and
        // exhaustively match the impossible error — no `expect`/`unwrap` needed.
        match self.try_map_payload(|p| Ok::<Q, std::convert::Infallible>(f(p))) {
            Ok(tree) => tree,
            Err(void) => match void {},
        }
    }

    /// Fallible variant of [`FlowTree::map_payload`].
    ///
    /// Stops at the first span whose closure returns `Err`, propagating that
    /// error immediately. Useful when payload construction can fail (e.g.
    /// parsing a token span into a structured type).
    ///
    /// # Errors
    ///
    /// Returns `Err(e)` if `f` returns `Err(e)` for any span; iteration stops
    /// at the first failing span.
    pub fn try_map_payload<Q, E>(
        self,
        mut f: impl FnMut(P) -> Result<Q, E>,
    ) -> Result<FlowTree<Q>, E> {
        let root = map_nodes(self.0, &mut f)?;
        Ok(FlowTree(root))
    }
}

/// Map a sequence of nodes, short-circuiting on the first error.
fn map_nodes<P, Q, E>(
    nodes: Vec<FlowNode<P>>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<Vec<FlowNode<Q>>, E> {
    nodes.into_iter().map(|node| map_node(node, f)).collect()
}

/// Map a single node: apply `f` to a payload, or recurse into a block.
fn map_node<P, Q, E>(
    node: FlowNode<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<FlowNode<Q>, E> {
    Ok(match node {
        FlowNode::Payload(p) => FlowNode::Payload(f(p)?),
        FlowNode::Block(block) => FlowNode::Block(map_block(block, f)?),
    })
}

/// Map every payload nested within a control-flow block.
fn map_block<P, Q, E>(
    block: FlowBlock<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<FlowBlock<Q>, E> {
    Ok(match block {
        FlowBlock::Random(r) => FlowBlock::Random(RandomBlock {
            value: r.value,
            has_end_random: r.has_end_random,
            branches: r
                .branches
                .into_iter()
                .map(|branch| map_random_branch(branch, f))
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

/// Map payloads inside a single `RandomBranch`.
fn map_random_branch<P, Q, E>(
    branch: RandomBranch<P>,
    f: &mut impl FnMut(P) -> Result<Q, E>,
) -> Result<RandomBranch<Q>, E> {
    Ok(RandomBranch {
        kind: branch.kind,
        body: map_nodes(branch.body, f)?,
    })
}

/// Map payloads inside a single `SwitchCase`.
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
