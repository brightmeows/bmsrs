//! Integration tests for `FlowDoc::map_payload` and `try_map_payload`.

#![expect(
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    reason = "test code uses assertions and unwraps in Result-returning functions"
)]

use bms_control_flow::{ControlFlowError, FlowBlock, FlowDoc, FlowNode, TokenPayload};
use bms_tokenizer::BmsTokenizer;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// Helper: tokenize and build a `FlowDoc<TokenPayload<&str>>`.
fn build_doc(input: &str) -> Result<FlowDoc<TokenPayload<&str>>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, &str>(input)
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

    // Map each payload span to its token count.
    let counted: FlowDoc<usize> = tree.map_payload(|p: TokenPayload<&str>| p.tokens.len());

    // Root carries the top-level payload (1 token: #TITLE) then the Random block.
    assert_eq!(counted.len(), 2);
    let Some(FlowNode::Payload(top_n)) = counted.first() else {
        panic!("expected top-level payload");
    };
    assert_eq!(*top_n, 1);

    let Some(FlowNode::Block(FlowBlock::Random(r))) = counted.get(1) else {
        panic!("expected Random block");
    };
    assert_eq!(r.branches.len(), 2);

    // Branch 1 body: one payload span of 2 tokens.
    let b1 = r.branches.first().expect("branch 1");
    assert_eq!(b1.body.len(), 1);
    let Some(FlowNode::Payload(n)) = b1.body.first() else {
        panic!("expected payload in branch 1");
    };
    assert_eq!(*n, 2);

    // Branch 2 body: one payload span of 1 token.
    let b2 = r.branches.get(1).expect("branch 2");
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

    // Collect each span's token count.
    let spans: FlowDoc<usize> = tree.map_payload(|p: TokenPayload<&str>| p.tokens.len());

    let Some(FlowNode::Block(FlowBlock::Switch(s))) = spans.first() else {
        panic!("expected Switch block");
    };
    assert_eq!(s.cases.len(), 2);
    // Case 1: 1 token; Case 2: 2 consecutive tokens packed.
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

/// Error used by the `try_map_payload` failure test.
#[derive(Debug, PartialEq, Eq)]
struct SpanError;

#[test]
fn try_map_payload_propagates_first_error() -> TestResult {
    // Two spans: top-level payload + a payload inside the Random branch.
    let tree = build_doc(
        "#TITLE a\n\
         #RANDOM 2\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // Fail on the second span by counting calls.
    let mut calls = 0;
    let result: Result<FlowDoc<usize>, SpanError> =
        tree.try_map_payload(|p: TokenPayload<&str>| {
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
        tree.try_map_payload(|p: TokenPayload<&str>| Ok(p.tokens.len()));
    let mapped = result.expect("all spans ok");
    assert_eq!(mapped.len(), 1);
    Ok(())
}
