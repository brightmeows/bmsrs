//! Integration tests for `FlowDocumentBuilder::to_tokens` roundtrip.

use bms_control_flow::ControlFlowError;
use bms_control_flow::{FlowDocumentBuilder, FlowItem};
use bms_tokenizer::{BmsHeader, BmsToken, BmsTokenizer};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// Helper: tokenize and build a `Vec<FlowItem>` with `C = &str`.
fn build_doc(input: &str) -> std::result::Result<Vec<FlowItem<&str>>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, &str>(input)
        .into_iter()
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect();
    FlowDocumentBuilder::from_tokens(tokens)
}

/// Helper: tokenize and build with `C = String` for polymorphism coverage.
fn build_doc_string(input: &str) -> std::result::Result<Vec<FlowItem<String>>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, String>(input)
        .into_iter()
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect();
    FlowDocumentBuilder::from_tokens(tokens)
}

/// Extract control-flow headers from a token list as debug strings.
fn cf_debug(tokens: &[BmsToken<&str>]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|t| match t {
            BmsToken::Header(BmsHeader::ControlFlow(cf)) => Some(format!("{cf:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn plain_tokens_roundtrip_no_control_flow() -> TestResult {
    let items = build_doc("#TITLE Test\n#BPM 120\n#00101:1122")?;
    let tokens = FlowDocumentBuilder::to_tokens(&items);

    assert_eq!(tokens.len(), 3);
    assert!(matches!(tokens.first(), Some(BmsToken::Header(_))));
    assert!(matches!(tokens.get(1), Some(BmsToken::Header(_))));
    assert!(matches!(tokens.get(2), Some(BmsToken::Message(_))));
    Ok(())
}

#[test]
fn random_block_roundtrip_preserves_structure() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #IF 2\n\
         #00101:22\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Random(2)".to_string(),
            "If(1)".to_string(),
            "EndIf".to_string(),
            "If(2)".to_string(),
            "EndIf".to_string(),
            "EndRandom".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn switch_block_roundtrip_preserves_structure() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #00101:11\n\
         #CASE 2\n\
         #00101:22\n\
         #ENDSW",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(2)".to_string(),
            "Case(1)".to_string(),
            "Case(2)".to_string(),
            "EndSwitch".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn nested_blocks_roundtrip_preserves_structure() -> TestResult {
    let items = build_doc(
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

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(2)".to_string(),
            "Case(1)".to_string(),
            "Random(2)".to_string(),
            "If(1)".to_string(),
            "EndIf".to_string(),
            "If(2)".to_string(),
            "EndIf".to_string(),
            "EndRandom".to_string(),
            "Case(2)".to_string(),
            "EndSwitch".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn setrandom_preserved_as_setrandom() -> TestResult {
    let items = build_doc(
        "#SETRANDOM 1\n\
         #IF 1\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "SetRandom(1)".to_string(),
            "If(1)".to_string(),
            "EndIf".to_string(),
            "EndRandom".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn no_endrandom_block_not_in_top_level() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #ENDIF",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    assert!(tokens.is_empty());
    Ok(())
}

#[test]
fn elseif_and_else_roundtrip() -> TestResult {
    let items = build_doc(
        "#RANDOM 3\n\
         #IF 1\n\
         #ENDIF\n\
         #ELSEIF 2\n\
         #ENDIF\n\
         #ELSE\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Random(3)".to_string(),
            "If(1)".to_string(),
            "EndIf".to_string(),
            "ElseIf(2)".to_string(),
            "EndIf".to_string(),
            "Else".to_string(),
            "EndIf".to_string(),
            "EndRandom".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn skip_preserved_in_switch() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #00101:11\n\
         #SKIP 1\n\
         #CASE 2\n\
         #00101:22\n\
         #ENDSW",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(2)".to_string(),
            "Case(1)".to_string(),
            "Skip(0)".to_string(),
            "Case(2)".to_string(),
            "EndSwitch".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn setswitch_preserved_as_setswitch() -> TestResult {
    let items = build_doc(
        "#SETSWITCH 1\n\
         #CASE 1\n\
         #00101:11\n\
         #ENDSW",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "SetSwitch(1)".to_string(),
            "Case(1)".to_string(),
            "EndSwitch".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn switch_def_with_skip_roundtrip() -> TestResult {
    let items = build_doc(
        "#SWITCH 3\n\
         #CASE 1\n\
         #00101:11\n\
         #SKIP 0\n\
         #DEF\n\
         #00101:22\n\
         #SKIP 0\n\
         #ENDSW",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(3)".to_string(),
            "Case(1)".to_string(),
            "Skip(0)".to_string(),
            "Def".to_string(),
            "Skip(0)".to_string(),
            "EndSwitch".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn switch_only_def_roundtrip() -> TestResult {
    let items = build_doc(
        "#SWITCH 3\n\
         #DEF\n\
         #00101:11\n\
         #SKIP 0\n\
         #ENDSW",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(3)".to_string(),
            "Def".to_string(),
            "Skip(0)".to_string(),
            "EndSwitch".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn elseif_with_content_roundtrip() -> TestResult {
    let items = build_doc(
        "#RANDOM 5\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #ELSEIF 2\n\
         #00101:22\n\
         #ENDIF\n\
         #ELSEIF 3\n\
         #00101:33\n\
         #ENDIF\n\
         #ELSE\n\
         #00101:44\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Random(5)".to_string(),
            "If(1)".to_string(),
            "EndIf".to_string(),
            "ElseIf(2)".to_string(),
            "EndIf".to_string(),
            "ElseIf(3)".to_string(),
            "EndIf".to_string(),
            "Else".to_string(),
            "EndIf".to_string(),
            "EndRandom".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn random_empty_block_roundtrip() -> TestResult {
    let items = build_doc("#RANDOM 2\n#ENDRANDOM")?;
    let tokens = FlowDocumentBuilder::to_tokens(&items);
    let cfs = cf_debug(&tokens);

    assert_eq!(cfs, vec!["Random(2)".to_string(), "EndRandom".to_string()]);
    Ok(())
}

/// Verify control-flow roundtrip works with `C = String`.
#[test]
fn random_block_roundtrip_with_string_container() -> TestResult {
    let items = build_doc_string(
        "#RANDOM 2\n\
         #IF 1\n\
         #00101:11\n\
         #ENDIF\n\
         #IF 2\n\
         #00101:22\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let tokens: Vec<BmsToken<String>> = FlowDocumentBuilder::to_tokens(&items);
    let cfs: Vec<String> = tokens
        .iter()
        .filter_map(|t| match t {
            BmsToken::Header(BmsHeader::ControlFlow(cf)) => Some(format!("{cf:?}")),
            _ => None,
        })
        .collect();

    assert_eq!(
        cfs,
        vec![
            "Random(2)".to_string(),
            "If(1)".to_string(),
            "EndIf".to_string(),
            "If(2)".to_string(),
            "EndIf".to_string(),
            "EndRandom".to_string(),
        ]
    );
    Ok(())
}
