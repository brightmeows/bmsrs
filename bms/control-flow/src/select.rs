//! 基于 RNG 的 [`FlowDoc`] 分支选择。

use bms_tokenizer::BmsToken;

use crate::rng::BranchRng;
use crate::{
    BlockDecision, BranchSelection, BranchValue, FlowBlock, FlowDoc, FlowNode, RandomBranchKind,
    SwitchCaseKind, TokenPayload,
};

impl<C: Clone + PartialEq> FlowDoc<TokenPayload<C>> {
    /// 使用 `rng` 为每个控制流块选择一个分支。
    ///
    /// 返回仅含已选分支的扁平 token 流，以及记录每次决策的
    /// [`BranchSelection`]。
    ///
    /// - `#SETRANDOM` / `#SETSWITCH` 块直接使用其 `max` 值，不调用 `rng`。
    /// - `#RANDOM` 块：选择条件与 RNG 值匹配的第一个分支。
    /// - `#SWITCH` 块：选择与值匹配的第一个 `#CASE`，并 fall-through 到
    ///   后续 case，直到遇到 `#SKIP`。
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

/// 处理单个 [`FlowNode`]，将 token 追加到 `output`。
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

/// 在 [`FlowBlock`] 内选择分支。
fn select_block<C: Clone + PartialEq>(
    block: &FlowBlock<TokenPayload<C>>,
    rng: &mut impl BranchRng,
    output: &mut Vec<BmsToken<C>>,
    decisions: &mut Vec<BlockDecision>,
) {
    match block {
        FlowBlock::Random(r) => {
            // `#RANDOM 0` 是格式错误的（空范围会让 RNG panic）。将值视为
            // 不匹配任何 `#IF` 分支，产生一个静默空块 —— 与下方未匹配分支
            // 的处理方式一致。
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
            // `#SWITCH 0` 是格式错误的（参见上方 Random 分支）：使用一个
            // 不匹配任何 `#CASE` 的值，避免空范围 panic。
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
