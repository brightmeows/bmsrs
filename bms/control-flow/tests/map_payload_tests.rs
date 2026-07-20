//! `FlowDoc::map_payload` 与 `try_map_payload` 的集成测试。

#![expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "test code uses assertions and unwraps in Result-returning functions"
)]

use bms_control_flow::{ControlFlowError, FlowBlock, FlowDoc, FlowNode, TokenPayload};
use bms_tokenizer::BmsTokenizer;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// 辅助函数：对文本分词并构建 `FlowDoc<TokenPayload>`。
fn build_doc(input: &str) -> Result<FlowDoc<TokenPayload>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>>(input)
        .into_iter()
        .filter_map(|(line, res)| res.ok().map(|t| (line, t)))
        .collect();
    FlowDoc::from_tokens(tokens)
}

#[test]
fn map_payload_transforms_each_span_preserving_structure() -> TestResult {
    let tree = build_doc(
        "#TITLE top\n\
         #RANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #00101:11\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // 将每个载荷片段映射为其 token 数量。
    let counted: FlowDoc<usize> = tree.map_payload(|p: TokenPayload| p.tokens.len());

    // 根节点先携带顶层载荷（1 个 token：#TITLE），随后是 Random 块。
    assert_eq!(counted.len(), 2);
    let Some(FlowNode::Payload(top_n)) = counted.first() else {
        panic!("expected top-level payload");
    };
    assert_eq!(*top_n, 1);

    let Some(FlowNode::Block(FlowBlock::Random(r))) = counted.get(1) else {
        panic!("expected Random block");
    };
    assert_eq!(r.chains.len(), 2);

    // 链 1 分支的 body：一个含 2 个 token 的载荷片段。
    let b1 = r.chains[0].branches.first().expect("branch 1");
    assert_eq!(b1.body.len(), 1);
    let Some(FlowNode::Payload(n)) = b1.body.first() else {
        panic!("expected payload in branch 1");
    };
    assert_eq!(*n, 2);

    // 链 2 分支的 body：一个含 1 个 token 的载荷片段。
    let b2 = r.chains[1].branches.first().expect("branch 2");
    assert_eq!(b2.body.len(), 1);
    let Some(FlowNode::Payload(n2)) = b2.body.first() else {
        panic!("expected payload in branch 2");
    };
    assert_eq!(*n2, 1);
    Ok(())
}

#[test]
fn map_payload_handles_switch_skeleton() -> TestResult {
    let tree = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #00101:11\n\
         #CASE 2\n\
         #00101:22\n\
         #00102:33\n\
         #ENDSW",
    )?;

    // 收集每个片段的 token 数量。
    let spans: FlowDoc<usize> = tree.map_payload(|p: TokenPayload| p.tokens.len());

    let Some(FlowNode::Block(FlowBlock::Switch(s))) = spans.first() else {
        panic!("expected Switch block");
    };
    assert_eq!(s.cases.len(), 2);
    // Case 1：1 个 token；Case 2：2 个连续 token 被打包。
    let c1 = s.cases.first().expect("case 1");
    let Some(FlowNode::Payload(n1)) = c1.body.first() else {
        panic!("expected payload in case 1");
    };
    assert_eq!(*n1, 1);
    let c2 = s.cases.get(1).expect("case 2");
    let Some(FlowNode::Payload(n2)) = c2.body.first() else {
        panic!("expected payload in case 2");
    };
    assert_eq!(*n2, 2);
    Ok(())
}

/// `try_map_payload` 失败测试所用的错误类型。
#[derive(Debug, PartialEq, Eq)]
struct SpanError;

#[test]
fn try_map_payload_propagates_first_error() -> TestResult {
    // 两个片段：顶层载荷 + Random 分支内的一个载荷。
    let tree = build_doc(
        "#TITLE a\n\
         #RANDOM 2\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // 通过计数调用次数，让第二个片段失败。
    let mut calls = 0;
    let result: Result<FlowDoc<usize>, SpanError> = tree.try_map_payload(|p: TokenPayload| {
        calls += 1;
        if calls == 1 {
            Ok(p.tokens.len())
        } else {
            Err(SpanError)
        }
    });

    assert_eq!(result, Err(SpanError));
    Ok(())
}

#[test]
fn try_map_payload_succeeds_when_all_spans_ok() -> TestResult {
    let tree = build_doc("#RANDOM 2\n#IF 1\n#00101:11\n#ENDIF\n#ENDRANDOM")?;
    let result: Result<FlowDoc<usize>, SpanError> =
        tree.try_map_payload(|p: TokenPayload| Ok(p.tokens.len()));
    let mapped = result.expect("all spans ok");
    assert_eq!(mapped.len(), 1);
    Ok(())
}
