//! Reverse conversion: tree → flat token stream.

use bms_tokenizer::{BmsHeader, BmsHeaderControlFlow, BmsToken};

use crate::{
    BranchValue, FlowBlock, FlowContent, FlowDocumentBuilder, FlowItem, RandomBranchKind,
    SwitchCaseKind,
};

impl FlowDocumentBuilder {
    /// Convert a sequence of [`FlowItem`]s back to a flat token stream.
    ///
    /// This reconstructs the control-flow header commands (`#RANDOM`, `#IF`,
    /// etc.) around the structured branches, producing a sequence that can
    /// be re-tokenized to produce an equivalent document.
    #[must_use]
    pub fn to_tokens<C: Clone + PartialEq>(items: &[FlowItem<C>]) -> Vec<BmsToken<C>> {
        let mut output = Vec::new();
        for item in items {
            push_item_tokens(item, &mut output);
        }
        output
    }
}

/// Push tokens for a single [`FlowItem`].
fn push_item_tokens<C: Clone + PartialEq>(item: &FlowItem<C>, output: &mut Vec<BmsToken<C>>) {
    match &item.content {
        FlowContent::Header(h) => {
            output.push(BmsToken::Header(h.clone()));
        }
        FlowContent::Message(m) => {
            output.push(BmsToken::Message(m.clone()));
        }
        FlowContent::Block(block) => {
            push_block_tokens(block, output);
        }
    }
}

/// Push tokens for a [`FlowBlock`], including all control-flow headers.
fn push_block_tokens<C: Clone + PartialEq>(block: &FlowBlock<C>, output: &mut Vec<BmsToken<C>>) {
    match block {
        FlowBlock::Random(r) => {
            let open = match r.value {
                BranchValue::Max(n) => BmsHeaderControlFlow::Random(n),
                BranchValue::Set(n) => BmsHeaderControlFlow::SetRandom(n),
            };
            output.push(BmsToken::Header(BmsHeader::ControlFlow(open)));

            for branch in &r.branches {
                let branch_header = match branch.kind {
                    RandomBranchKind::If(v) => BmsHeaderControlFlow::If(v),
                    RandomBranchKind::ElseIf(v) => BmsHeaderControlFlow::ElseIf(v),
                    RandomBranchKind::Else => BmsHeaderControlFlow::Else,
                };
                output.push(BmsToken::Header(BmsHeader::ControlFlow(branch_header)));

                for item in &branch.body {
                    push_item_tokens(item, output);
                }

                output.push(BmsToken::Header(BmsHeader::ControlFlow(
                    BmsHeaderControlFlow::EndIf,
                )));
            }

            if r.has_end_random {
                output.push(BmsToken::Header(BmsHeader::ControlFlow(
                    BmsHeaderControlFlow::EndRandom,
                )));
            }
        }
        FlowBlock::Switch(s) => {
            let open = match s.value {
                BranchValue::Max(n) => BmsHeaderControlFlow::Switch(n),
                BranchValue::Set(n) => BmsHeaderControlFlow::SetSwitch(n),
            };
            output.push(BmsToken::Header(BmsHeader::ControlFlow(open)));

            for case in &s.cases {
                let case_header = match case.kind {
                    SwitchCaseKind::Case(v) => BmsHeaderControlFlow::Case(v),
                    SwitchCaseKind::Def => BmsHeaderControlFlow::Def,
                };
                output.push(BmsToken::Header(BmsHeader::ControlFlow(case_header)));

                for item in &case.body {
                    push_item_tokens(item, output);
                }

                if case.has_skip {
                    output.push(BmsToken::Header(BmsHeader::ControlFlow(
                        BmsHeaderControlFlow::Skip(0),
                    )));
                }
            }

            output.push(BmsToken::Header(BmsHeader::ControlFlow(
                BmsHeaderControlFlow::EndSwitch,
            )));
        }
    }
}
