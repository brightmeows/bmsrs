//! Integration tests for `FlowDocument::select_branches` (Task 3).

use bms_control_flow::{BranchRng, ControlFlowError, DeterministicRng, FlowDoc, TokenPayload};
use bms_tokenizer::{
    BmsHeader, BmsHeaderControlFlow, BmsHeaderResDefAudio, BmsToken, BmsTokenizer,
};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// Helper: tokenize BMS text and build a `FlowDoc<TokenPayload<&str>>`.
fn build_doc(input: &str) -> std::result::Result<FlowDoc<TokenPayload<&str>>, ControlFlowError> {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, &str>(input)
        .into_iter()
        .filter_map(|(line, result)| result.ok().map(|token| (line, token)))
        .collect();
    FlowDoc::from_tokens(tokens)
}

/// Helper: find a seed that makes `DeterministicRng` produce `target` on first
/// `gen_range(max)` call.
fn find_seed(target: u64, max: u64) -> Option<u64> {
    for seed in 0..1000 {
        let mut rng = DeterministicRng::new(seed);
        if rng.gen_range(max) == target {
            return Some(seed);
        }
    }
    None
}

/// Extract WAV filenames from tokens.
fn wav_filenames<'a>(tokens: &'a [BmsToken<&str>]) -> Vec<&'a str> {
    tokens
        .iter()
        .filter_map(|t| match t {
            BmsToken::Header(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                filename,
                ..
            })) => Some(*filename),
            _ => None,
        })
        .collect()
}

/// Extract the control-flow headers from a token list.
fn cf_headers(tokens: &[BmsToken<&str>]) -> Vec<BmsHeaderControlFlow> {
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
    let mut rng = DeterministicRng::new(seed);
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
         #ENDIF\n\
         #IF 2\n\
         #WAV02 snare.wav\n\
         #ENDIF\n\
         #ELSE\n\
         #WAV03 hihat.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(3, 3).ok_or("no seed found for value 3")?;
    let mut rng = DeterministicRng::new(seed);
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
    let mut rng = DeterministicRng::new(seed);
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
    let mut rng = DeterministicRng::new(seed);
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
         #SKIP 1\n\
         #CASE 2\n\
         #WAV02 snare.wav\n\
         #ENDSW",
    )?;

    let seed = find_seed(1, 2).ok_or("no seed found for value 1")?;
    let mut rng = DeterministicRng::new(seed);
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

    let mut rng = DeterministicRng::new(999);
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

    let mut rng = DeterministicRng::new(999);
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

    let mut rng = DeterministicRng::new(0);
    let (_, selection) = items.select_branches(&mut rng);

    assert_eq!(selection.decisions.len(), 2);
    Ok(())
}

#[test]
fn no_control_flow_preserves_all_tokens() -> TestResult {
    let items = build_doc("#TITLE Test\n#BPM 120\n#00101:1122")?;
    let mut rng = DeterministicRng::new(0);
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
    let mut rng = DeterministicRng::new(seed);
    let (tokens, _) = items.select_branches(&mut rng);

    let cfs = cf_headers(&tokens);
    assert!(
        cfs.is_empty(),
        "expected no control-flow headers, got {cfs:?}"
    );
    Ok(())
}

// BMS spec-derived tests (bmspec-4, memo/13-control-flow)

/// Ad-hoc RNG that uses pre-determined seeds in sequence.
struct TwoStepRng {
    rng1: DeterministicRng,
    rng2: DeterministicRng,
    call_count: u64,
}

impl TwoStepRng {
    fn new(seed1: u64, seed2: u64) -> Self {
        Self {
            rng1: DeterministicRng::new(seed1),
            rng2: DeterministicRng::new(seed2),
            call_count: 0,
        }
    }
}

impl BranchRng for TwoStepRng {
    fn gen_range(&mut self, max: u64) -> u64 {
        let result = if self.call_count == 0 {
            self.rng1.gen_range(max)
        } else {
            self.rng2.gen_range(max)
        };
        self.call_count += 1;
        result
    }
}

// bmspec-4 Scenario 2: RNG yields 2 → #IF 2 branch selected.
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
    let mut rng = DeterministicRng::new(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(decisions.decisions[0].value, 2);
    assert_eq!(decisions.decisions[0].selected_index, 1);
    assert_eq!(wav_filenames(&tokens), vec!["b.wav"]);
    Ok(())
}

// bmspec-4 Scenario 3: two sequential #RANDOM blocks, each selected independently.
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

    // First RANDOM → 2 (b.wav), second RANDOM → 1 (c.wav)
    let seed1 = find_seed(2, 2).ok_or("no seed found")?;
    let seed2 = find_seed(1, 2).ok_or("no seed found")?;
    let mut rng = TwoStepRng::new(seed1, seed2);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(decisions.decisions[0].value, 2);
    assert_eq!(decisions.decisions[1].value, 1);
    assert_eq!(wav_filenames(&tokens), vec!["b.wav", "c.wav"]);
    Ok(())
}

// memo/13-control-flow: #IF 1 / #ELSEIF 2 / #ELSEIF 3 / #ELSE, RNG = 2 → first match (#ELSEIF 2).
#[test]
fn elseif_first_match_wins() -> TestResult {
    let items = build_doc(
        "#RANDOM 5\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #ELSEIF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ELSEIF 3\n\
         #WAV01 c.wav\n\
         #ENDIF\n\
         #ELSE\n\
         #WAV01 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(2, 5).ok_or("no seed found for value 2")?;
    let mut rng = DeterministicRng::new(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].value, 2);
    assert_eq!(decisions.decisions[0].selected_index, 1); // #ELSEIF 2
    assert_eq!(wav_filenames(&tokens), vec!["b.wav"]);
    Ok(())
}

// memo/13-control-flow: RNG = 4 (no match in ELSEIF chain) → #ELSE selected.
#[test]
fn elseif_no_match_falls_to_else() -> TestResult {
    let items = build_doc(
        "#RANDOM 5\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #ELSEIF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ELSEIF 3\n\
         #WAV01 c.wav\n\
         #ENDIF\n\
         #ELSE\n\
         #WAV01 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    let seed = find_seed(4, 5).ok_or("no seed found for value 4")?;
    let mut rng = DeterministicRng::new(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].selected_index, 3); // #ELSE
    assert_eq!(wav_filenames(&tokens), vec!["d.wav"]);
    Ok(())
}

// memo/13-control-flow: #DEF selected when no #CASE matches.
#[test]
fn switch_def_fallback() -> TestResult {
    let items = build_doc(
        "#SWITCH 3\n\
         #CASE 1\n\
         #WAV01 a.wav\n\
         #SKIP 0\n\
         #CASE 2\n\
         #WAV01 b.wav\n\
         #SKIP 0\n\
         #DEF\n\
         #WAV01 c.wav\n\
         #SKIP 0\n\
         #ENDSW",
    )?;

    let seed = find_seed(3, 3).ok_or("no seed found for value 3")?;
    let mut rng = DeterministicRng::new(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].selected_index, 2); // #DEF
    assert_eq!(wav_filenames(&tokens), vec!["c.wav"]);
    Ok(())
}

// Edge case: RNG value that doesn't match any branch → nothing selected.
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
    let mut rng = DeterministicRng::new(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert!(wav_filenames(&tokens).is_empty());
    assert_eq!(decisions.decisions[0].selected_index, 1); // no match (branches.len())
    Ok(())
}

// Reference-derived tests (bms-rs control-flow semantics)

// #SWITCH with only #DEF — the simplest switch block.
#[test]
fn switch_only_def_selected_when_no_case() -> TestResult {
    let items = build_doc(
        "#SWITCH 3\n\
         #DEF\n\
         #WAV01 a.wav\n\
         #SKIP 0\n\
         #ENDSW",
    )?;

    let seed = find_seed(3, 3).ok_or("no seed found for value 3")?;
    let mut rng = DeterministicRng::new(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions[0].selected_index, 0);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav"]);
    Ok(())
}

// Empty #RANDOM block (no branches at all) produces no output.
#[test]
fn empty_random_block_select_yields_nothing() -> TestResult {
    let items = build_doc("#RANDOM 2\n#ENDRANDOM")?;
    let mut rng = DeterministicRng::new(0);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert!(tokens.is_empty());
    assert_eq!(decisions.decisions[0].selected_index, 0);
    Ok(())
}

// #RANDOM with a single #IF branch.
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
    let mut rng = DeterministicRng::new(seed);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(wav_filenames(&tokens), vec!["a.wav"]);
    assert_eq!(decisions.decisions[0].selected_index, 0);
    Ok(())
}

// Three-level nesting: RANDOM inside SWITCH inside RANDOM.
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
         #SKIP 0\n\
         #ENDSW\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // Outer → 2 (switch branch), Switch → 2 (case2 w/ inner random),
    // Inner → 1 (innermost.wav)
    // DeterministicRng seed 3 produces sequence [2, 2, 1]
    let mut rng = DeterministicRng::new(3);
    let (tokens, decisions) = items.select_branches(&mut rng);

    assert_eq!(decisions.decisions.len(), 3);
    assert!(wav_filenames(&tokens).contains(&"innermost.wav"));
    Ok(())
}

// Headers and messages around control flow are preserved in selection output.
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
    let mut rng = DeterministicRng::new(seed);
    let (tokens, _) = items.select_branches(&mut rng);

    assert_eq!(tokens.len(), 3);
    Ok(())
}

// Nested control-flow tests ported from bms-rs (nested_switch.rs)

/// RNG with pre-determined values (ignores max).
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

// Ported from nested_switch.rs → nested_switch_simpler / nested_switch
// SWITCH → CASE 1 → SWITCH → CASE 1, with note content.
#[test]
fn switch_in_switch_select() -> TestResult {
    let items = build_doc(
        "#SWITCH 2\n\
         #CASE 1\n\
         #WAV01 a.wav\n\
         #SWITCH 2\n\
         #CASE 1\n\
         #WAV02 b.wav\n\
         #SKIP 0\n\
         #CASE 2\n\
         #WAV03 c.wav\n\
         #SKIP 0\n\
         #ENDSW\n\
         #SKIP 0\n\
         #CASE 2\n\
         #WAV04 d.wav\n\
         #SKIP 0\n\
         #ENDSW",
    )?;

    // RNG [1, 1]: outer→1 (CASE 1), inner→1 (CASE 1) → a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]: outer→1, inner→2 (CASE 2) → a.wav, c.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "c.wav"]);

    // RNG [2]: outer→2 (CASE 2) → d.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(wav_filenames(&tokens), vec!["d.wav"]);
    Ok(())
}

// Ported from nested_switch.rs → nested_random_in_switch
// SWITCH → CASE 1 → RANDOM 2 (IF 1 / ELSEIF 2).
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
         #SKIP 0\n\
         #CASE 2\n\
         #WAV04 d.wav\n\
         #SKIP 0\n\
         #ENDSW",
    )?;

    // RNG [1, 1]: SWITCH→1, RANDOM→1 (IF 1) → a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]: SWITCH→1, RANDOM→2 (ELSEIF 2) → a.wav, c.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "c.wav"]);

    // RNG [2]: SWITCH→2 (CASE 2) → d.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(wav_filenames(&tokens), vec!["d.wav"]);
    Ok(())
}

// Ported from nested_switch.rs → nested_switch_in_random
// RANDOM 2 → IF 1 → SWITCH 2, with ELSE fallback.
#[test]
fn nested_switch_in_random_select() -> TestResult {
    let items = build_doc(
        "#RANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #SWITCH 2\n\
         #CASE 1\n\
         #WAV02 b.wav\n\
         #SKIP 0\n\
         #CASE 2\n\
         #WAV03 c.wav\n\
         #SKIP 0\n\
         #ENDSW\n\
         #ELSE\n\
         #WAV04 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM",
    )?;

    // RNG [1, 1]: RANDOM→1 (IF 1), SWITCH→1 (CASE 1) → a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]: RANDOM→1, SWITCH→2 (CASE 2) → a.wav, c.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "c.wav"]);

    // RNG [2]: RANDOM→2 (ELSE) → d.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(wav_filenames(&tokens), vec!["d.wav"]);
    Ok(())
}

// Ported from nested_switch.rs → test_switch_insane (BMIIDXView2010 test case).
// Complex multi-level nesting with 5-level SWITCH, RANDOM, and inner SWITCH.
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
         #SKIP 0\n\
         #CASE 2\n\
         #WAV04 d.wav\n\
         #SKIP 0\n\
         #CASE 3\n\
         #WAV05 e.wav\n\
         #SWITCH 2\n\
         #CASE 1\n\
         #WAV06 f.wav\n\
         #SKIP 0\n\
         #CASE 2\n\
         #WAV07 g.wav\n\
         #SKIP 0\n\
         #ENDSW\n\
         #SKIP 0\n\
         #ENDSW",
    )?;

    // RNG [1, 1]: SWITCH→1 (CASE 1), RANDOM→1 (IF 1) → a.wav, b.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "b.wav"]);

    // RNG [1, 2]: SWITCH→1, RANDOM→2 (ELSE) → a.wav, c.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[1, 2]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["a.wav", "c.wav"]);

    // RNG [2]: SWITCH→2 (CASE 2) → d.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[2]));
    assert_eq!(decisions.decisions.len(), 1);
    assert_eq!(wav_filenames(&tokens), vec!["d.wav"]);

    // RNG [3, 1]: SWITCH→3 (CASE 3), inner SWITCH→1 → e.wav, f.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[3, 1]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["e.wav", "f.wav"]);

    // RNG [3, 2]: SWITCH→3, inner SWITCH→2 → e.wav, g.wav
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[3, 2]));
    assert_eq!(decisions.decisions.len(), 2);
    assert_eq!(wav_filenames(&tokens), vec!["e.wav", "g.wav"]);

    // RNG [4]: SWITCH→4 (no CASE 4, nothing selected) → empty
    let (tokens, decisions) = items.select_branches(&mut SequenceRng::new(&[4]));
    assert_eq!(decisions.decisions.len(), 1);
    assert!(wav_filenames(&tokens).is_empty());
    Ok(())
}
