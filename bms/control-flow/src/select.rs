//! RNG-based branch selection for [`FlowDocument`].

use bms_tokenizer::BmsToken;

use crate::rng::BranchRng;
use crate::{
    BlockDecision, BranchSelection, BranchValue, FlowBlock, FlowContent, FlowDocumentBuilder,
    FlowItem, RandomBranchKind, SwitchCaseKind,
};

impl FlowDocumentBuilder {
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
    pub fn select_branches<'a>(
        items: &[FlowItem<'a>],
        rng: &mut impl BranchRng,
    ) -> (Vec<BmsToken<'a>>, BranchSelection) {
        let mut output = Vec::new();
        let mut decisions = Vec::new();

        for item in items {
            select_item(item, rng, &mut output, &mut decisions);
        }

        (output, BranchSelection { decisions })
    }
}

/// Process a single [`FlowItem`], appending tokens to `output`.
fn select_item<'a>(
    item: &FlowItem<'a>,
    rng: &mut impl BranchRng,
    output: &mut Vec<BmsToken<'a>>,
    decisions: &mut Vec<BlockDecision>,
) {
    match &item.content {
        FlowContent::Header(h) => {
            output.push(BmsToken::Header(h.clone()));
        }
        FlowContent::Message(m) => {
            output.push(BmsToken::Message(m.clone()));
        }
        FlowContent::Block(block) => {
            select_block(block, rng, output, decisions);
        }
    }
}

/// Select branches within a [`FlowBlock`].
fn select_block<'a>(
    block: &FlowBlock<'a>,
    rng: &mut impl BranchRng,
    output: &mut Vec<BmsToken<'a>>,
    decisions: &mut Vec<BlockDecision>,
) {
    match block {
        FlowBlock::Random(r) => {
            let value = match r.value {
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
                for item in &branch.body {
                    select_item(item, rng, output, decisions);
                }
            }

            decisions.push(BlockDecision {
                value,
                selected_index,
            });
        }
        FlowBlock::Switch(s) => {
            let value = match s.value {
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
                    for item in &case.body {
                        select_item(item, rng, output, decisions);
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
