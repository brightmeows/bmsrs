//! Integration tests for `FlowDoc::from_tokens`.

use std::num::NonZeroUsize;

use bms_control_flow::{
    ControlFlowError, FlowBlock, FlowDoc, FlowNode, RandomBlock, RandomBranchKind, SwitchBlock,
    SwitchCaseKind, TokenPayload,
};
use bms_tokenizer::{BmsToken, BmsTokenizer};

type TestResult = std::result::Result<(), ControlFlowError>;

/// Helper: tokenize BMS text and filter to successful tokens only.
fn tokenize(input: &str) -> Vec<(NonZeroUsize, BmsToken<&str>)> {
    BmsTokenizer::new()
        .tokenize::<Vec<_>, &str>(input)
        .into_iter()
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect()
}

/// Helper: tokenize and build a `FlowDoc<TokenPayload<&str>>`.
fn build_doc(input: &str) -> std::result::Result<FlowDoc<TokenPayload<&str>>, ControlFlowError> {
    let tokens = tokenize(input);
    FlowDoc::from_tokens(tokens)
}

/// Extract the first root node as a `Random` block reference, or panic.
fn as_random<'a>(
    root: &'a [FlowNode<TokenPayload<&'a str>>],
) -> &'a RandomBlock<TokenPayload<&'a str>> {
    let Some(FlowNode::Block(FlowBlock::Random(r))) = root.first() else {
        panic!("expected Random block");
    };
    r
}

/// Extract the first root node as a `Switch` block reference, or panic.
fn as_switch<'a>(
    root: &'a [FlowNode<TokenPayload<&'a str>>],
) -> &'a SwitchBlock<TokenPayload<&'a str>> {
    let Some(FlowNode::Block(FlowBlock::Switch(s))) = root.first() else {
        panic!("expected Switch block");
    };
    s
}

#[test]
fn plain_headers_no_control_flow_packs_into_single_payload() -> TestResult {
    let tree = build_doc("#TITLE Test\n#BPM 120\n#00101:1122")?;
    assert_eq!(tree.len(), 1);
    let Some(FlowNode::Payload(payload)) = tree.first() else {
        panic!("expected single payload node");
    };
    assert_eq!(payload.tokens.len(), 3);
    assert!(matches!(payload.tokens[0].1, BmsToken::Header(_)));
    assert!(matches!(payload.tokens[1].1, BmsToken::Header(_)));
    assert!(matches!(payload.tokens[2].1, BmsToken::Message(_)));
    Ok(())
}

#[test]
fn simple_random_block_has_two_branches() -> TestResult {
    let tree = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #IF 2\n\
         #00101:22\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    assert_eq!(tree.len(), 1);
    let block = as_random(&tree);
    assert_eq!(block.branches.len(), 2);
    assert_eq!(block.value, bms_control_flow::BranchValue::Max(2));
    assert!(block.has_end_random);
    assert_eq!(
        block.branches.first().map(|b| b.kind),
        Some(RandomBranchKind::If(1))
    );
    assert_eq!(
        block.branches.get(1).map(|b| b.kind),
        Some(RandomBranchKind::If(2))
    );
    Ok(())
}

#[test]
fn random_with_elseif_else_has_three_branches() -> TestResult {
    let tree = build_doc(
        "#RANDOM 3\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #ELSEIF 2\n\
         #00101:22\n\
         #ENDIF\n\
         #ELSE\n\
         #00101:33\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let block = as_random(&tree);
    assert_eq!(block.branches.len(), 3);
    assert_eq!(
        block.branches.first().map(|b| b.kind),
        Some(RandomBranchKind::If(1))
    );
    assert_eq!(
        block.branches.get(1).map(|b| b.kind),
        Some(RandomBranchKind::ElseIf(2))
    );
    assert_eq!(
        block.branches.get(2).map(|b| b.kind),
        Some(RandomBranchKind::Else)
    );
    Ok(())
}

#[test]
fn simple_switch_block_has_two_cases() -> TestResult {
    let tree = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #00101:11\n\
         #CASE 2\n\
         #00101:22\n\
         #ENDSW",
    )?;

    assert_eq!(tree.len(), 1);
    let block = as_switch(&tree);
    assert_eq!(block.cases.len(), 2);
    assert_eq!(block.value, bms_control_flow::BranchValue::Max(2));
    assert_eq!(
        block.cases.first().map(|c| c.kind),
        Some(SwitchCaseKind::Case(1))
    );
    assert_eq!(
        block.cases.get(1).map(|c| c.kind),
        Some(SwitchCaseKind::Case(2))
    );
    Ok(())
}

#[test]
fn switch_with_def_and_skip_flags_set() -> TestResult {
    let tree = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #00101:11\n\
         #SKIP 1\n\
         #DEF\n\
         #00101:22\n\
         #ENDSW",
    )?;

    let block = as_switch(&tree);
    assert_eq!(block.cases.len(), 2);
    assert_eq!(
        block.cases.first().map(|c| c.kind),
        Some(SwitchCaseKind::Case(1))
    );
    assert_eq!(block.cases.first().map(|c| c.has_skip), Some(true));
    assert_eq!(
        block.cases.get(1).map(|c| c.kind),
        Some(SwitchCaseKind::Def)
    );
    assert_eq!(block.cases.get(1).map(|c| c.has_skip), Some(false));
    Ok(())
}

#[test]
fn nested_random_in_switch_builds_tree() -> TestResult {
    let tree = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #RANDOM 2\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #IF 2\n\
         #00101:22\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #CASE 2\n\
         #00101:33\n\
         #ENDSW",
    )?;

    assert_eq!(tree.len(), 1);
    let switch = as_switch(&tree);
    assert_eq!(switch.cases.len(), 2);

    // Case 1 body should contain a nested Random block
    let case1 = switch.cases.first().expect("case 1");
    assert_eq!(case1.body.len(), 1);
    assert!(matches!(
        case1.body.first(),
        Some(FlowNode::Block(FlowBlock::Random(_)))
    ));

    // Case 2 body should contain a single payload (the message)
    let case2 = switch.cases.get(1).expect("case 2");
    assert_eq!(case2.body.len(), 1);
    assert!(matches!(case2.body.first(), Some(FlowNode::Payload(_))));
    Ok(())
}

#[test]
fn set_random_sets_value() -> TestResult {
    let tree = build_doc(
        "#SETRANDOM 5\n\
         #IF 5\n\
         #00101:11\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let block = as_random(&tree);
    assert_eq!(block.value, bms_control_flow::BranchValue::Set(5));
    Ok(())
}

#[test]
fn set_switch_sets_value() -> TestResult {
    let tree = build_doc(
        "#SETSWITCH 3\n\
         #CASE 3\n\
         #00101:11\n\
         #ENDSW",
    )?;

    let block = as_switch(&tree);
    assert_eq!(block.value, bms_control_flow::BranchValue::Set(3));
    Ok(())
}

#[test]
fn endrandom_present_sets_has_end_random_true() -> TestResult {
    let tree = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let block = as_random(&tree);
    assert!(block.has_end_random);
    Ok(())
}

#[test]
fn no_endrandom_leaves_block_unpopped() -> TestResult {
    let tree = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #ENDIF",
    )?;

    assert!(tree.is_empty());
    Ok(())
}

#[test]
fn unmatched_if_returns_error() {
    let tokens = tokenize("#IF 1\n#ENDIF");
    let result = FlowDoc::from_tokens(tokens);
    assert!(matches!(result, Err(ControlFlowError::UnmatchedIf { .. })));
}

#[test]
fn unmatched_endif_returns_error() {
    let tokens = tokenize("#ENDIF");
    let result = FlowDoc::from_tokens(tokens);
    assert!(matches!(
        result,
        Err(ControlFlowError::UnmatchedEndIf { .. })
    ));
}

#[test]
fn unmatched_endrandom_returns_error() {
    let tokens = tokenize("#ENDRANDOM");
    let result = FlowDoc::from_tokens(tokens);
    assert!(matches!(
        result,
        Err(ControlFlowError::UnmatchedEndRandom { .. })
    ));
}

#[test]
fn unmatched_case_returns_error() {
    let tokens = tokenize("#CASE 1");
    let result = FlowDoc::from_tokens(tokens);
    assert!(matches!(
        result,
        Err(ControlFlowError::UnmatchedCase { .. })
    ));
}

#[test]
fn unmatched_endsw_returns_error() {
    let tokens = tokenize("#ENDSW");
    let result = FlowDoc::from_tokens(tokens);
    assert!(matches!(
        result,
        Err(ControlFlowError::UnmatchedEndSw { .. })
    ));
}

#[test]
fn branch_body_packs_consecutive_tokens_into_payload() -> TestResult {
    let tree = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 kick.wav\n\
         #00101:AA\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 snare.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let block = as_random(&tree);
    // Branch 1 has two consecutive tokens packed into one payload node.
    let branch1 = block.branches.first().expect("branch 1");
    assert_eq!(branch1.body.len(), 1);
    let Some(FlowNode::Payload(p1)) = branch1.body.first() else {
        panic!("expected payload in branch 1");
    };
    assert_eq!(p1.tokens.len(), 2);
    // Branch 2 has a single token in one payload node.
    let branch2 = block.branches.get(1).expect("branch 2");
    assert_eq!(branch2.body.len(), 1);
    Ok(())
}

#[test]
fn switch_case_bodies_pack_consecutive_tokens() -> TestResult {
    let tree = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #WAV01 kick.wav\n\
         #00101:11\n\
         #CASE 2\n\
         #00101:22\n\
         #ENDSW",
    )?;

    let block = as_switch(&tree);
    // Case 1 has two consecutive tokens packed into one payload node.
    let case1 = block.cases.first().expect("case 1");
    assert_eq!(case1.body.len(), 1);
    let Some(FlowNode::Payload(p1)) = case1.body.first() else {
        panic!("expected payload in case 1");
    };
    assert_eq!(p1.tokens.len(), 2);
    // Case 2 has a single token in one payload node.
    let case2 = block.cases.get(1).expect("case 2");
    assert_eq!(case2.body.len(), 1);
    Ok(())
}
