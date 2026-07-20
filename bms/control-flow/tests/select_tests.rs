//! `FlowDocument::select_branches`（任务 3）的集成测试。

#![expect(
    clippy::panic_in_result_fn,
    clippy::similar_names,
    reason = "test code uses assertions and RNG-sequence variable names"
)]

use bms_control_flow::{BranchRng, ControlFlowError, FlowDoc, TokenPayload};
use bms_tokenizer::{
    BmsHeader, BmsHeaderControlFlow, BmsHeaderResDefAudio, BmsToken, BmsTokenizer,
};
use rand::SeedableRng as _;
use rand::rngs::StdRng;
use std::fmt::Write as _;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// 辅助函数：对 BMS 文本分词并构建 `FlowDoc<TokenPayload>`。
fn build_doc(input: &str) -> std::result::Result<FlowDoc<TokenPayload>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>>(input)
        .into_iter()
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect();
    FlowDoc::from_tokens(tokens)
}

/// 辅助函数：寻找一个种子，使 `StdRng` 在首次调用 `gen_range(max)` 时
/// 产生 `target`。
fn find_seed(target: u64, max: u64) -> Option<u64> {
    for seed in 0..1000 {
        let mut rng = StdRng::seed_from_u64(seed);
        if rng.gen_range(max) == target {
            return Some(seed);
        }
    }
    None
}

/// 从 token 中提取 WAV 文件名。
fn wav_filenames(tokens: &[BmsToken]) -> Vec<&str> {
    tokens
        .iter()
        .filter_map(|t| match t {
            BmsToken::Header(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                filename,
                ..
            })) => Some(filename.as_str()),
            _ => None,
        })
        .collect()
}

/// 从 token 列表中提取控制流头部命令。
fn cf_headers(tokens: &[BmsToken]) -> Vec<BmsHeaderControlFlow> {
    tokens
        .iter()
        .filter_map(|t| match t {
            BmsToken::Header(BmsHeader::ControlFlow(cf)) => Some(cf.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn random_selects_matching_branch() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 kick.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 snare.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(1, 2).ok_or("no seed found for value 1")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, selection) = items.select_branches(&mut rng);

    assert_eq!(selection.decisions.len(), 1);
    let decision = selection.decisions.first().ok_or("no decision")?;
    assert_eq!(decision.value, 1);
    assert_eq!(decision.selected_index, 0);

    let wavs = wav_filenames(&tokens);
    assert_eq!(wavs, vec!["kick.wav"]);
    Ok(())
}

#[test]
fn random_else_fallback_selected_when_no_match() -> TestResult {
    let items = build_doc(
        "#RANDOM 3\n\
         #IF 1\n\
         #WAV01 kick.wav\n\
         #ELSEIF 2\n\
         #WAV02 snare.wav\n\
         #ELSE\n\
         #WAV03 hihat.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(3, 3).ok_or("no seed found for value 3")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, selection) = items.select_branches(&mut rng);

    let decision = selection.decisions.first().ok_or("no decision")?;
    assert_eq!(decision.value, 3);
    assert_eq!(decision.selected_index, 2);

    let wavs = wav_filenames(&tokens);
    assert_eq!(wavs, vec!["hihat.wav"]);
    Ok(())
}

#[test]
fn switch_selects_matching_case() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #WAV01 kick.wav\n\
         #CASE 2\n\
         #WAV02 snare.wav\n\
         #ENDSW",
    )?;

    let seed = find_seed(2, 2).ok_or("no seed found for value 2")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, selection) = items.select_branches(&mut rng);

    let decision = selection.decisions.first().ok_or("no decision")?;
    assert_eq!(decision.value, 2);
    assert_eq!(decision.selected_index, 1);

    let wavs = wav_filenames(&tokens);
    assert_eq!(wavs, vec!["snare.wav"]);
    Ok(())
}

#[test]
fn switch_fall_through_continues_to_next_case() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #WAV01 kick.wav\n\
         #CASE 2\n\
         #WAV02 snare.wav\n\
         #ENDSW",
    )?;

    let seed = find_seed(1, 2).ok_or("no seed found for value 1")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, selection) = items.select_branches(&mut rng);

    let decision = selection.decisions.first().ok_or("no decision")?;
    assert_eq!(decision.value, 1);
    assert_eq!(decision.selected_index, 0);

    let wavs = wav_filenames(&tokens);
    assert_eq!(wavs, vec!["kick.wav", "snare.wav"]);
    Ok(())
}

#[test]
fn switch_stops_at_skip() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #WAV01 kick.wav\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV02 snare.wav\n\
         #ENDSW",
    )?;

    let seed = find_seed(1, 2).ok_or("no seed found for value 1")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, selection) = items.select_branches(&mut rng);

    let decision = selection.decisions.first().ok_or("no decision")?;
    assert_eq!(decision.value, 1);

    let wavs = wav_filenames(&tokens);
    assert_eq!(wavs, vec!["kick.wav"]);
    Ok(())
}

#[test]
fn set_random_uses_fixed_value() -> TestResult {
    let items = build_doc(
        "#SETRANDOM 1\n\
         #IF 1\n\
         #WAV01 kick.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 snare.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let mut rng = StdRng::seed_from_u64(999);
    let (tokens, selection) = items.select_branches(&mut rng);

    let decision = selection.decisions.first().ok_or("no decision")?;
    assert_eq!(decision.value, 1);
    assert_eq!(decision.selected_index, 0);

    let wavs = wav_filenames(&tokens);
    assert_eq!(wavs, vec!["kick.wav"]);
    Ok(())
}

#[test]
fn set_switch_uses_fixed_value() -> TestResult {
    let items = build_doc(
        "#SETSWITCH 2\n\
         #CASE 1\n\
         #WAV01 kick.wav\n\
         #CASE 2\n\
         #WAV02 snare.wav\n\
         #ENDSW",
    )?;

    let mut rng = StdRng::seed_from_u64(999);
    let (tokens, selection) = items.select_branches(&mut rng);

    let decision = selection.decisions.first().ok_or("no decision")?;
    assert_eq!(decision.value, 2);
    assert_eq!(decision.selected_index, 1);

    let wavs = wav_filenames(&tokens);
    assert_eq!(wavs, vec!["snare.wav"]);
    Ok(())
}

#[test]
fn nested_block_selection_both_blocks_decided() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #RANDOM 2\n\
         #IF 1\n\
         #WAV01 kick.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 snare.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #CASE 2\n\
         #WAV03 hihat.wav\n\
         #ENDSW",
    )?;

    let mut rng = SequenceRng::new(&[1, 1]);
    let (_, selection) = items.select_branches(&mut rng);

    assert_eq!(selection.decisions.len(), 2);
    Ok(())
}

#[test]
fn no_control_flow_preserves_all_tokens() -> TestResult {
    let items = build_doc("#TITLE Test\n#BPM 120\n#00101:1122")?;
    let mut rng = StdRng::seed_from_u64(0);
    let (tokens, selection) = items.select_branches(&mut rng);

    assert!(selection.decisions.is_empty());
    assert_eq!(tokens.len(), 3);
    Ok(())
}

#[test]
fn cf_headers_not_in_selected_output() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 kick.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(1, 2).ok_or("no seed found for value 1")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, _) = items.select_branches(&mut rng);

    let cfs = cf_headers(&tokens);
    assert!(
        cfs.is_empty(),
        "expected no control-flow headers, got {cfs:?}"
    );
    Ok(())
}

// 源自 BMS 规范的测试（bmspec-4、memo/13-control-flow）

// bmspec-4 场景 2：RNG 产生 2 → 选择 #IF 2 分支。
#[test]
fn random_selects_second_branch() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(2, 2).ok_or("no seed found for value 2")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(decisions.decisions[0].value, 2);
    assert_eq!(decisions.decisions[0].selected_index, 1);
    assert_eq!(wav_filenames(&tokens), vec!["b.wav"]);
    Ok(())
}

// bmspec-4 场景 3：两个连续的 #RANDOM 块，各自独立选择。
#[test]
fn multiple_sequential_random_blocks_select() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #RANDOM 2\n\
         #IF 1\n\
         #WAV02 c.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // 第一个 RANDOM → 2（b.wav），第二个 RANDOM → 1（c.wav）
    let mut rng = SequenceRng::new(&[2, 1]);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(decisions.decisions[0].value, 2);
    assert_eq!(decisions.decisions[1].value, 1);
    assert_eq!(wav_filenames(&tokens), vec!["b.wav", "c.wav"]);
    Ok(())
}

// memo/13-control-flow：#IF 1 / #ELSEIF 2 / #ELSEIF 3 / #ELSE，RNG = 2 → 首匹配（#ELSEIF 2）。
#[test]
fn elseif_first_match_wins() -> TestResult {
    let items = build_doc(
        "#RANDOM 5\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ELSEIF 2\n\
         #WAV01 b.wav\n\
         #ELSEIF 3\n\
         #WAV01 c.wav\n\
         #ELSE\n\
         #WAV01 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(2, 5).ok_or("no seed found for value 2")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].value, 2);
    assert_eq!(decisions.decisions[0].selected_index, 1); // #ELSEIF 2
    assert_eq!(wav_filenames(&tokens), vec!["b.wav"]);
    Ok(())
}

// memo/13-control-flow：RNG = 4（ELSEIF 链中无匹配）→ 选择 #ELSE。
#[test]
fn elseif_no_match_falls_to_else() -> TestResult {
    let items = build_doc(
        "#RANDOM 5\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ELSEIF 2\n\
         #WAV01 b.wav\n\
         #ELSEIF 3\n\
         #WAV01 c.wav\n\
         #ELSE\n\
         #WAV01 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(4, 5).ok_or("no seed found for value 4")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].selected_index, 3); // #ELSE
    assert_eq!(wav_filenames(&tokens), vec!["d.wav"]);
    Ok(())
}

// memo/13-control-flow：无 #CASE 匹配时选择 #DEF。
#[test]
fn switch_def_fallback() -> TestResult {
    let items = build_doc(
        "#SWITCH 3\n\
         #CASE 1\n\
         #WAV01 a.wav\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV01 b.wav\n\
         #SKIP\n\
         #DEF\n\
         #WAV01 c.wav\n\
         #SKIP\n\
         #ENDSW",
    )?;

    let seed = find_seed(3, 3).ok_or("no seed found for value 3")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].selected_index, 2); // #DEF
    assert_eq!(wav_filenames(&tokens), vec!["c.wav"]);
    Ok(())
}

// 边界情况：RNG 值不匹配任何分支 → 不选出任何内容。
#[test]
fn random_no_branch_matches_yields_no_output() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(2, 2).ok_or("no seed found for value 2")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert!(wav_filenames(&tokens).is_empty());
    assert_eq!(decisions.decisions[0].selected_index, 1); // 无匹配（branches.len()）
    Ok(())
}

// 源自参考实现的测试（bms-rs control-flow 语义）

// 只含 #DEF 的 #SWITCH —— 最简单的 switch 块。
#[test]
fn switch_only_def_selected_when_no_case() -> TestResult {
    let items = build_doc(
        "#SWITCH 3\n\
         #DEF\n\
         #WAV01 a.wav\n\
         #SKIP\n\
         #ENDSW",
    )?;

    let seed = find_seed(3, 3).ok_or("no seed found for value 3")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].selected_index, 0);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav"]);
    Ok(())
}

// 空的 #RANDOM 块（完全没有分支）不产出任何输出。
#[test]
fn empty_random_block_select_yields_nothing() -> TestResult {
    let items = build_doc("#RANDOM 2\n#ENDRANDOM")?;
    let mut rng = StdRng::seed_from_u64(0);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert!(tokens.is_empty());
    assert_eq!(decisions.decisions[0].selected_index, 0);
    Ok(())
}

// 含单个 #IF 分支的 #RANDOM。
#[test]
fn random_single_branch_selected() -> TestResult {
    let items = build_doc(
        "#RANDOM 1\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(1, 1).ok_or("no seed found for value 1")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(wav_filenames(&tokens), vec!["a.wav"]);
    assert_eq!(decisions.decisions[0].selected_index, 0);
    Ok(())
}

// 三层嵌套：RANDOM 内嵌 SWITCH 再内嵌 RANDOM。
#[test]
fn nested_three_levels_selection() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 outer_a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #SWITCH 2\n\
         #CASE 1\n\
         #WAV01 inner_switch_case1.wav\n\
         #CASE 2\n\
         #RANDOM 2\n\
         #IF 1\n\
         #WAV01 innermost.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 fallback.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #SKIP\n\
         #ENDSW\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // 外层 → 2（switch 分支），Switch → 2（含内层 random 的 case2），
    // 内层 → 1（innermost.wav）
    let mut rng = SequenceRng::new(&[2, 2, 1]);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions.len(), 3);
    assert!(wav_filenames(&tokens).contains(&"innermost.wav"));
    Ok(())
}

// 控制流前后的头部命令与 message 在选择输出中保持不变。
#[test]
fn headers_around_control_flow_preserved_in_selection() -> TestResult {
    let items = build_doc(
        "#TITLE Test\n\
         #RANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #BPM 120",
    )?;

    let seed = find_seed(1, 2).ok_or("no seed found")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let (tokens, _) = items.select_branches(&mut rng);

    assert_eq!(tokens.len(), 3);
    Ok(())
}

// 从 bms-rs 移植的嵌套控制流测试（nested_switch.rs）

/// 使用预定义值的 RNG（忽略 max）。
struct SequenceRng {
    values: Vec<u64>,
    pos: usize,
}

impl SequenceRng {
    fn new(values: &[u64]) -> Self {
        Self {
            values: values.to_vec(),
            pos: 0,
        }
    }
}

impl BranchRng for SequenceRng {
    fn gen_range(&mut self, _max: u64) -> u64 {
        #[expect(
            clippy::indexing_slicing,
            reason = "panics intentionally when values exhausted"
        )]
        let val = self.values[self.pos];
        self.pos += 1;
        val
    }
}

// 移植自 nested_switch.rs → nested_switch_simpler / nested_switch
// SWITCH → CASE 1 → SWITCH → CASE 1，含音符内容。
#[test]
fn switch_in_switch_select() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #WAV01 a.wav\n\
         #SWITCH 2\n\
         #CASE 1\n\
         #WAV02 b.wav\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV03 c.wav\n\
         #SKIP\n\
         #ENDSW\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV04 d.wav\n\
         #SKIP\n\
         #ENDSW",
    )?;

    // RNG [1, 1]：外层→1（CASE 1），内层→1（CASE 1）→ a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]：外层→1，内层→2（CASE 2）→ a.wav, c.wav
    let (result_12, dec_12) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(dec_12.decisions.len(), 2);
    assert_eq!(wav_filenames(&result_12), vec!["a.wav", "c.wav"]);

    // RNG [2]：外层→2（CASE 2）→ d.wav
    let (result_2, dec_2) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(dec_2.decisions.len(), 1);
    assert_eq!(wav_filenames(&result_2), vec!["d.wav"]);
    Ok(())
}

// 移植自 nested_switch.rs → nested_random_in_switch
// SWITCH → CASE 1 → RANDOM 2（IF 1 / ELSEIF 2）。
#[test]
fn nested_random_in_switch_select() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #WAV01 a.wav\n\
         #RANDOM 2\n\
         #IF 1\n\
         #WAV02 b.wav\n\
         #ENDIF\n\
         #ELSEIF 2\n\
         #WAV03 c.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV04 d.wav\n\
         #SKIP\n\
         #ENDSW",
    )?;

    // RNG [1, 1]：SWITCH→1，RANDOM→1（IF 1）→ a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]：SWITCH→1，RANDOM→2（ELSEIF 2）→ a.wav, c.wav
    let (result_12, dec_12) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(dec_12.decisions.len(), 2);
    assert_eq!(wav_filenames(&result_12), vec!["a.wav", "c.wav"]);

    // RNG [2]：SWITCH→2（CASE 2）→ d.wav
    let (result_2, dec_2) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(dec_2.decisions.len(), 1);
    assert_eq!(wav_filenames(&result_2), vec!["d.wav"]);
    Ok(())
}

// 移植自 nested_switch.rs → nested_switch_in_random
// RANDOM 2 → IF 1 → SWITCH 2，含 ELSE 回退。
#[test]
fn nested_switch_in_random_select() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #SWITCH 2\n\
         #CASE 1\n\
         #WAV02 b.wav\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV03 c.wav\n\
         #SKIP\n\
         #ENDSW\n\
         #ELSE\n\
         #WAV04 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // RNG [1, 1]：RANDOM→1（IF 1），SWITCH→1（CASE 1）→ a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]：RANDOM→1，SWITCH→2（CASE 2）→ a.wav, c.wav
    let (result_12, dec_12) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(dec_12.decisions.len(), 2);
    assert_eq!(wav_filenames(&result_12), vec!["a.wav", "c.wav"]);

    // RNG [2]：RANDOM→2（ELSE）→ d.wav
    let (result_2, dec_2) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(dec_2.decisions.len(), 1);
    assert_eq!(wav_filenames(&result_2), vec!["d.wav"]);
    Ok(())
}

// 移植自 nested_switch.rs → test_switch_insane（BMIIDXView2010 测试用例）。
// 含 5 层 SWITCH、RANDOM 与内层 SWITCH 的复杂多层嵌套。
#[test]
fn switch_insane_multi_level_select() -> TestResult {
    let items = build_doc(
        "#SWITCH 5\n\
         #CASE 1\n\
         #WAV01 a.wav\n\
         #RANDOM 2\n\
         #IF 1\n\
         #WAV02 b.wav\n\
         #ELSE\n\
         #WAV03 c.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV04 d.wav\n\
         #SKIP\n\
         #CASE 3\n\
         #WAV05 e.wav\n\
         #SWITCH 2\n\
         #CASE 1\n\
         #WAV06 f.wav\n\
         #SKIP\n\
         #CASE 2\n\
         #WAV07 g.wav\n\
         #SKIP\n\
         #ENDSW\n\
         #SKIP\n\
         #ENDSW",
    )?;

    // RNG [1, 1]：SWITCH→1（CASE 1），RANDOM→1（IF 1）→ a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]：SWITCH→1，RANDOM→2（ELSE）→ a.wav, c.wav
    let (result_12, dec_12) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(dec_12.decisions.len(), 2);
    assert_eq!(wav_filenames(&result_12), vec!["a.wav", "c.wav"]);

    // RNG [2]：SWITCH→2（CASE 2）→ d.wav
    let (result_2, dec_2) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(dec_2.decisions.len(), 1);
    assert_eq!(wav_filenames(&result_2), vec!["d.wav"]);

    // RNG [3, 1]：SWITCH→3（CASE 3），内层 SWITCH→1 → e.wav, f.wav
    let (result_31, dec_31) = items.select_branches(&mut SequenceRng::new(&[3, 1]));
    assert_eq!(dec_31.decisions.len(), 2);
    assert_eq!(wav_filenames(&result_31), vec!["e.wav", "f.wav"]);

    // RNG [3, 2]：SWITCH→3，内层 SWITCH→2 → e.wav, g.wav
    let (result_32, dec_32) = items.select_branches(&mut SequenceRng::new(&[3, 2]));
    assert_eq!(dec_32.decisions.len(), 2);
    assert_eq!(wav_filenames(&result_32), vec!["e.wav", "g.wav"]);

    // RNG [4]：SWITCH→4（无 CASE 4，不选出任何内容）→ 空
    let (result_4, dec_4) = items.select_branches(&mut SequenceRng::new(&[4]));
    assert_eq!(dec_4.decisions.len(), 1);
    assert!(wav_filenames(&result_4).is_empty());
    Ok(())
}

#[test]
fn random_zero_yields_empty_branch_without_panicking() -> TestResult {
    // `#RANDOM 0` 是格式错误的（空范围）；select_branches 不得 panic ——
    // 它使用值 0，不匹配任何 `#IF`，因此该块为静默空块。
    let doc = build_doc("#RANDOM 0\n#IF 1\n#WAV01 a.wav\n#ENDIF\n#ENDRANDOM")?;
    let (tokens, decisions) = doc.select_branches(&mut SequenceRng::new(&[]));
    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(decisions.decisions[0].value, 0);
    assert!(wav_filenames(&tokens).is_empty());
    Ok(())
}

#[test]
fn switch_zero_yields_empty_branch_without_panicking() -> TestResult {
    // `#SWITCH 0` 是格式错误的（空范围）；遵循同样的优雅空块约定。
    let doc = build_doc("#SWITCH 0\n#CASE 1\n#WAV01 a.wav\n#SKIP\n#ENDSW")?;
    let (tokens, decisions) = doc.select_branches(&mut SequenceRng::new(&[]));
    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(decisions.decisions[0].value, 0);
    assert!(wav_filenames(&tokens).is_empty());
    Ok(())
}

#[test]
fn deep_nested_random_10_levels_succeeds() -> TestResult {
    let mut bms = String::new();
    for _ in 0..10 {
        writeln!(bms, "#RANDOM 2")?;
        writeln!(bms, "#IF 1")?;
    }
    for _ in 0..10 {
        writeln!(bms, "#ENDIF")?;
        writeln!(bms, "#ENDRANDOM")?;
    }
    let tree = build_doc(&bms)?;
    assert!(!tree.is_empty());
    Ok(())
}

#[test]
fn deep_nested_random_20_levels_succeeds() -> TestResult {
    let mut bms = String::new();
    for _ in 0..20 {
        writeln!(bms, "#RANDOM 2")?;
        writeln!(bms, "#IF 1")?;
    }
    for _ in 0..20 {
        writeln!(bms, "#ENDIF")?;
        writeln!(bms, "#ENDRANDOM")?;
    }
    let tree = build_doc(&bms)?;
    assert!(!tree.is_empty());
    Ok(())
}

#[test]
fn deep_nested_random_50_levels_succeeds() -> TestResult {
    let mut bms = String::new();
    for _ in 0..50 {
        writeln!(bms, "#RANDOM 2")?;
        writeln!(bms, "#IF 1")?;
    }
    for _ in 0..50 {
        writeln!(bms, "#ENDIF")?;
        writeln!(bms, "#ENDRANDOM")?;
    }
    let tree = build_doc(&bms)?;
    assert!(!tree.is_empty());
    Ok(())
}

#[test]
fn deep_nested_switch_10_levels_succeeds() -> TestResult {
    let mut bms = String::new();
    for _ in 0..10 {
        writeln!(bms, "#SWITCH 2")?;
        writeln!(bms, "#CASE 1")?;
    }
    for _ in 0..10 {
        writeln!(bms, "#SKIP")?;
        writeln!(bms, "#ENDSW")?;
    }
    let tree = build_doc(&bms)?;
    assert!(!tree.is_empty());
    Ok(())
}
