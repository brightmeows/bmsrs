//! RNG-based branch selection for [`FlowDoc`].

use bms_tokenizer::BmsToken;

use crate::rng::BranchRng;
use crate::{
    BlockDecision, BranchSelection, BranchValue, FlowBlock, FlowDoc, FlowNode, RandomBranchKind,
    SwitchCaseKind, TokenPayload,
};

impl<C: Clone + PartialEq> FlowDoc<TokenPayload<C>> {
    /// Select one branch per control-flow block using `rng`.
    ///
    /// Returns a flat token stream with only the chosen branches, plus a
    /// [`BranchSelection`] recording every decision.
    ///
    /// - `#SETRANDOM` / `#SETSWITCH` blocks use their `max` value directly
    ///   without calling `rng`.
    /// - `#RANDOM` blocks: the first branch whose condition matches the RNG
    ///   value is selected.
    /// - `#SWITCH` blocks: the first `#CASE` matching the value is selected,
    ///   with fall-through to subsequent cases until `#SKIP` is hit.
    #[must_use]
    pub fn select_branches(&self, rng: &mut impl BranchRng) -> (Vec<BmsToken<C>>, BranchSelection) {
        let mut output = Vec::new();
        let mut decisions = Vec::new();

        for node in self.iter() {
            select_node(node, rng, &mut output, &mut decisions);
        }

        (output, BranchSelection { decisions })
    }
}

/// Process a single [`FlowNode`], appending tokens to `output`.
fn select_node<C: Clone + PartialEq>(
    node: &FlowNode<TokenPayload<C>>,
    rng: &mut impl BranchRng,
    output: &mut Vec<BmsToken<C>>,
    decisions: &mut Vec<BlockDecision>,
) {
    match node {
        FlowNode::Payload(payload) => {
            for (_, token) in &payload.tokens {
                output.push(token.clone());
            }
        }
        FlowNode::Block(block) => select_block(block, rng, output, decisions),
    }
}

/// Select branches within a [`FlowBlock`].
fn select_block<C: Clone + PartialEq>(
    block: &FlowBlock<TokenPayload<C>>,
    rng: &mut impl BranchRng,
    output: &mut Vec<BmsToken<C>>,
    decisions: &mut Vec<BlockDecision>,
) {
    match block {
        FlowBlock::Random(r) => {
            // `#RANDOM 0` is malformed (an empty range would panic in the
            // RNG). Treat it as a value matching no `#IF` branch, yielding a
            // silent empty block — consistent with how unmatched branches
            // are handled below.
            let value = match r.value {
                BranchValue::Max(0) => 0,
                BranchValue::Max(max) => rng.gen_range(max),
                BranchValue::Set(n) => n,
            };
            let selected_index = r
                .branches
                .iter()
                .position(|branch| match branch.kind {
                    RandomBranchKind::If(v) | RandomBranchKind::ElseIf(v) => v == value,
                    RandomBranchKind::Else => true,
                })
                .unwrap_or(r.branches.len());

            if let Some(branch) = r.branches.get(selected_index) {
                for node in &branch.body {
                    select_node(node, rng, output, decisions);
                }
            }

            decisions.push(BlockDecision {
                value,
                selected_index,
            });
        }
        FlowBlock::Switch(s) => {
            // `#SWITCH 0` is malformed (see the Random arm above): avoid the
            // empty-range panic by using a value that matches no `#CASE`.
            let value = match s.value {
                BranchValue::Max(0) => 0,
                BranchValue::Max(max) => rng.gen_range(max),
                BranchValue::Set(n) => n,
            };
            let mut selected_index = s.cases.len();
            let mut found_match = false;
            let mut stopped = false;

            for (i, case) in s.cases.iter().enumerate() {
                if stopped {
                    break;
                }

                let case_matches = match case.kind {
                    SwitchCaseKind::Case(v) => v == value,
                    SwitchCaseKind::Def => true,
                };

                if case_matches {
                    if selected_index == s.cases.len() {
                        selected_index = i;
                    }
                    found_match = true;
                }

                if found_match {
                    for node in &case.body {
                        select_node(node, rng, output, decisions);
                    }
                    if case.has_skip {
                        stopped = true;
                    }
                }
            }

            decisions.push(BlockDecision {
                value,
                selected_index,
            });
        }
    }
}
