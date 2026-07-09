//! 反向转换：树 → 扁平 token 流。

use bms_tokenizer::{BmsHeader, BmsHeaderControlFlow, BmsToken};

use crate::{
    BranchValue, FlowBlock, FlowDoc, FlowNode, RandomBranchKind, SwitchCaseKind, TokenPayload,
};

impl<C: Clone + PartialEq> FlowDoc<TokenPayload<C>> {
    /// 将树转换回扁平 token 流。
    ///
    /// 在结构化分支周围重建控制流头部命令（`#RANDOM`、`#IF` 等），并解包
    /// 每个载荷片段，产出的序列可被重新分词为等价的文档。
    #[must_use]
    pub fn to_tokens(&self) -> Vec<BmsToken<C>> {
        let mut output = Vec::new();
        for node in self.iter() {
            push_node_tokens(node, &mut output);
        }
        output
    }
}

/// 压入单个 [`FlowNode`] 的 token。
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

/// 压入 [`FlowBlock`] 的 token，包含所有控制流头部命令。
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
                        BmsHeaderControlFlow::Skip,
                    )));
                }
            }

            output.push(BmsToken::Header(BmsHeader::ControlFlow(
                BmsHeaderControlFlow::EndSwitch,
            )));
        }
    }
}
