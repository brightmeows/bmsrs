//! `FlowDoc::to_tokens` roundtrip 的集成测试。

#![expect(
    clippy::panic_in_result_fn,
    reason = "test code uses assertions in Result-returning functions"
)]

use bms_control_flow::ControlFlowError;
use bms_control_flow::{FlowDoc, TokenPayload};
use bms_tokenizer::{BmsHeader, BmsToken, BmsTokenizer};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// 辅助函数：对文本分词并构建 `FlowDoc<TokenPayload<&str>>`。
fn build_doc(input: &str) -> std::result::Result<FlowDoc<TokenPayload<&str>>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, &str>(input)
        .into_iter()
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect();
    FlowDoc::from_tokens(tokens)
}

/// 辅助函数：以 `C = String` 分词并构建，用于多态覆盖测试。
fn build_doc_string(
    input: &str,
) -> std::result::Result<FlowDoc<TokenPayload<String>>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, String>(input)
        .into_iter()
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect();
    FlowDoc::from_tokens(tokens)
}

/// 从 token 列表中提取控制流头部命令，作为 Debug 字符串返回。
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
    let tokens = items.to_tokens();

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

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Random(2)".to_owned(),
            "If(1)".to_owned(),
            "EndIf".to_owned(),
            "If(2)".to_owned(),
            "EndIf".to_owned(),
            "EndRandom".to_owned(),
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

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(2)".to_owned(),
            "Case(1)".to_owned(),
            "Case(2)".to_owned(),
            "EndSwitch".to_owned(),
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

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(2)".to_owned(),
            "Case(1)".to_owned(),
            "Random(2)".to_owned(),
            "If(1)".to_owned(),
            "EndIf".to_owned(),
            "If(2)".to_owned(),
            "EndIf".to_owned(),
            "EndRandom".to_owned(),
            "Case(2)".to_owned(),
            "EndSwitch".to_owned(),
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

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "SetRandom(1)".to_owned(),
            "If(1)".to_owned(),
            "EndIf".to_owned(),
            "EndRandom".to_owned(),
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

    let tokens = items.to_tokens();
    assert!(tokens.is_empty());
    Ok(())
}

#[test]
fn elseif_and_else_roundtrip() -> TestResult {
    // 非标准写法（每分支独立 #ENDIF）：roundtrip 应忠实保留输入结构。
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

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Random(3)".to_owned(),
            "If(1)".to_owned(),
            "EndIf".to_owned(),
            "ElseIf(2)".to_owned(),
            "EndIf".to_owned(),
            "Else".to_owned(),
            "EndIf".to_owned(),
            "EndRandom".to_owned(),
        ]
    );
    Ok(())
}

#[test]
fn standard_elseif_chain_roundtrip() -> TestResult {
    // 标准写法（memo/13）：#IF…#ELSEIF…#ELSE 共享单个 #ENDIF，
    // 构成一条互斥链。roundtrip 后必须仍为单个 #ENDIF。
    let items = build_doc(
        "#RANDOM 3\n\
         #IF 1\n\
         #00101:11\n\
         #ELSEIF 2\n\
         #00101:22\n\
         #ELSE\n\
         #00101:33\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Random(3)".to_owned(),
            "If(1)".to_owned(),
            "ElseIf(2)".to_owned(),
            "Else".to_owned(),
            "EndIf".to_owned(),
            "EndRandom".to_owned(),
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
         #SKIP\n\
         #CASE 2\n\
         #00101:22\n\
         #ENDSW",
    )?;

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(2)".to_owned(),
            "Case(1)".to_owned(),
            "Skip".to_owned(),
            "Case(2)".to_owned(),
            "EndSwitch".to_owned(),
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

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "SetSwitch(1)".to_owned(),
            "Case(1)".to_owned(),
            "EndSwitch".to_owned(),
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
         #SKIP\n\
         #DEF\n\
         #00101:22\n\
         #SKIP\n\
         #ENDSW",
    )?;

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(3)".to_owned(),
            "Case(1)".to_owned(),
            "Skip".to_owned(),
            "Def".to_owned(),
            "Skip".to_owned(),
            "EndSwitch".to_owned(),
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
         #SKIP\n\
         #ENDSW",
    )?;

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Switch(3)".to_owned(),
            "Def".to_owned(),
            "Skip".to_owned(),
            "EndSwitch".to_owned(),
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

    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(
        cfs,
        vec![
            "Random(5)".to_owned(),
            "If(1)".to_owned(),
            "EndIf".to_owned(),
            "ElseIf(2)".to_owned(),
            "EndIf".to_owned(),
            "ElseIf(3)".to_owned(),
            "EndIf".to_owned(),
            "Else".to_owned(),
            "EndIf".to_owned(),
            "EndRandom".to_owned(),
        ]
    );
    Ok(())
}

#[test]
fn random_empty_block_roundtrip() -> TestResult {
    let items = build_doc("#RANDOM 2\n#ENDRANDOM")?;
    let tokens = items.to_tokens();
    let cfs = cf_debug(&tokens);

    assert_eq!(cfs, vec!["Random(2)".to_owned(), "EndRandom".to_owned()]);
    Ok(())
}

/// 验证控制流 roundtrip 在 `C = String` 时也能工作。
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

    let tokens: Vec<BmsToken<String>> = items.to_tokens();
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
            "Random(2)".to_owned(),
            "If(1)".to_owned(),
            "EndIf".to_owned(),
            "If(2)".to_owned(),
            "EndIf".to_owned(),
            "EndRandom".to_owned(),
        ]
    );
    Ok(())
}
