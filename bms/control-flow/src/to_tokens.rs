//! Reverse conversion: tree → flat token stream.

use bms_tokenizer::{BmsHeader, BmsHeaderControlFlow, BmsToken};

use crate::{
    BranchValue, FlowBlock, FlowDoc, FlowNode, RandomBranchKind, SwitchCaseKind, TokenPayload,
};

impl<C: Clone + PartialEq> FlowDoc<TokenPayload<C>> {
    /// Convert the tree back to a flat token stream.
    ///
    /// Reconstructs the control-flow header commands (`#RANDOM`, `#IF`, etc.)
    /// around the structured branches and unpacks each payload span, producing
    /// a sequence that can be re-tokenized to an equivalent document.
    #[must_use]
    pub fn to_tokens(&self) -> Vec<BmsToken<C>> {
        let mut output = Vec::new();
        for node in self.iter() {
            push_node_tokens(node, &mut output);
        }
        output
    }
}

/// Push tokens for a single [`FlowNode`].
fn push_node_tokens<C: Clone + PartialEq>(
    node: &FlowNode<TokenPayload<C>>,
    output: &mut Vec<BmsToken<C>>,
) {
    match node {
        FlowNode::Payload(payload) => {
            for (_, token) in &payload.tokens {
                output.push(token.clone());
            }
        }
        FlowNode::Block(block) => push_block_tokens(block, output),
    }
}

/// Push tokens for a [`FlowBlock`], including all control-flow headers.
fn push_block_tokens<C: Clone + PartialEq>(
    block: &FlowBlock<TokenPayload<C>>,
    output: &mut Vec<BmsToken<C>>,
) {
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

                for node in &branch.body {
                    push_node_tokens(node, output);
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

                for node in &case.body {
                    push_node_tokens(node, output);
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
