//! Regression tests for edge-case control-flow selection semantics.
//!
//! Covers selection semantics that still apply to the CST after the old
//! control-flow implementation: `#ELSEIF` first match, `#ELSE` / `#DEF`
//! fallback, `#SKIP` fall-through, multiple blocks, nesting, zero values, and
//! deep nesting.

use bmsrs_bms_syntax::{Line, SourceFile};

/// Test RNG that returns preset random values in order.
struct SequenceRng {
    /// Preset values.
    values: Vec<u64>,
    /// Current index.
    index: usize,
}

impl SequenceRng {
    /// Construct a preset sequence.
    fn new(values: &[u64]) -> Self {
        Self {
            values: values.to_vec(),
            index: 0,
        }
    }

    /// Return the next value; returns 1 when the sequence is exhausted.
    fn next(&mut self, max: u64) -> u64 {
        let value = self.values.get(self.index).copied().unwrap_or(1);
        self.index = self.index.saturating_add(1);
        value.min(max.max(1))
    }
}

/// Parse into a CST.
fn file(text: &str) -> SourceFile {
    SourceFile::parse(text).source_file()
}

/// Extract WAV file names from the selected lines.
fn wavs(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| {
            line.command_name()
                .is_some_and(|command| command.text().to_ascii_uppercase().starts_with("WAV"))
                .then(|| line.value().map(|value| value.text().to_owned()))
                .flatten()
        })
        .collect()
}

/// Return the first decision as `(value, selected_index)`.
fn first_decision(selection: &bmsrs_bms_syntax::BranchSelection) -> (u64, usize) {
    selection.decisions().first().map_or((0, 0), |decision| {
        (decision.value(), decision.selected_index())
    })
}

#[test]
fn random_selects_first_matching_branch() {
    let file = file(
        "#RANDOM 2\n#IF 1\n#WAV01 kick.wav\n#ENDIF\n#IF 2\n#WAV02 snare.wav\n#ENDIF\n#ENDRANDOM\n",
    );
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (vec!["kick.wav".to_owned()], (1, 0))
    );
}

#[test]
fn random_elseif_first_match_wins() {
    let file = file(
        "#RANDOM 5\n#IF 1\n#WAV01 a.wav\n#ELSEIF 2\n#WAV01 b.wav\n#ELSEIF 3\n#WAV01 c.wav\n#ELSE\n#WAV01 d.wav\n#ENDIF\n#ENDRANDOM\n",
    );
    let (lines, selection) = file.select_lines(|_| 2);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (vec!["b.wav".to_owned()], (2, 1))
    );
}

#[test]
fn random_elseif_no_match_falls_to_else() {
    let file = file(
        "#RANDOM 5\n#IF 1\n#WAV01 a.wav\n#ELSEIF 2\n#WAV01 b.wav\n#ELSEIF 3\n#WAV01 c.wav\n#ELSE\n#WAV01 d.wav\n#ENDIF\n#ENDRANDOM\n",
    );
    let (lines, selection) = file.select_lines(|_| 4);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (vec!["d.wav".to_owned()], (4, 3))
    );
}

#[test]
fn switch_fall_through_continues_until_end() {
    let file = file("#SWITCH 2\n#CASE 1\n#WAV01 kick.wav\n#CASE 2\n#WAV02 snare.wav\n#ENDSW\n");
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (vec!["kick.wav".to_owned(), "snare.wav".to_owned()], (1, 0))
    );
}

#[test]
fn switch_stops_at_skip() {
    let file =
        file("#SWITCH 2\n#CASE 1\n#WAV01 kick.wav\n#SKIP\n#CASE 2\n#WAV02 snare.wav\n#ENDSW\n");
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (vec!["kick.wav".to_owned()], (1, 0))
    );
}

#[test]
fn set_switch_uses_fixed_value_without_rng() {
    let file = file("#SETSWITCH 2\n#CASE 1\n#WAV01 kick.wav\n#CASE 2\n#WAV02 snare.wav\n#ENDSW\n");
    let mut called = false;
    let (lines, selection) = file.select_lines(|max| {
        called = true;
        max
    });
    assert_eq!(
        (called, wavs(&lines), first_decision(&selection)),
        (false, vec!["snare.wav".to_owned()], (2, 1))
    );
}

#[test]
fn multiple_sequential_random_blocks_select_independently() {
    let file = file(
        "#RANDOM 2\n#IF 1\n#WAV01 a.wav\n#ENDIF\n#IF 2\n#WAV01 b.wav\n#ENDIF\n#ENDRANDOM\n#RANDOM 2\n#IF 1\n#WAV02 c.wav\n#ENDIF\n#IF 2\n#WAV02 d.wav\n#ENDIF\n#ENDRANDOM\n",
    );
    let mut rng = SequenceRng::new(&[2, 1]);
    let (lines, selection) = file.select_lines(|max| rng.next(max));
    assert_eq!(
        (
            wavs(&lines),
            selection.decisions().len(),
            first_decision(&selection)
        ),
        (vec!["b.wav".to_owned(), "c.wav".to_owned()], 2, (2, 1))
    );
}

#[test]
fn switch_def_fallback_selects_default() {
    let file = file(
        "#SWITCH 3\n#CASE 1\n#WAV01 a.wav\n#SKIP\n#CASE 2\n#WAV01 b.wav\n#SKIP\n#DEF\n#WAV01 c.wav\n#SKIP\n#ENDSW\n",
    );
    let (lines, selection) = file.select_lines(|_| 3);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (vec!["c.wav".to_owned()], (3, 2))
    );
}

#[test]
fn random_no_branch_matches_yields_nothing() {
    let file = file("#RANDOM 2\n#IF 1\n#WAV01 a.wav\n#ENDIF\n#ENDRANDOM\n");
    let (lines, selection) = file.select_lines(|_| 2);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (Vec::<String>::new(), (2, 1))
    );
}

#[test]
fn empty_random_block_yields_nothing() {
    let file = file("#RANDOM 2\n#ENDRANDOM\n");
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (Vec::<String>::new(), (1, 0))
    );
}

#[test]
fn nested_random_and_switch_selects_inner_branch() {
    let file = file(
        "#RANDOM 2\n#IF 1\n#WAV01 outer_a.wav\n#ENDIF\n#IF 2\n#SWITCH 2\n#CASE 1\n#WAV02 inner_a.wav\n#CASE 2\n#WAV02 inner_b.wav\n#SKIP\n#ENDSW\n#ENDIF\n#ENDRANDOM\n",
    );
    let mut rng = SequenceRng::new(&[2, 1]);
    let (lines, selection) = file.select_lines(|max| rng.next(max));
    assert_eq!(
        (wavs(&lines), selection.decisions().len()),
        (vec!["inner_a.wav".to_owned(), "inner_b.wav".to_owned()], 2)
    );
}

#[test]
fn headers_around_control_flow_are_preserved() {
    let file = file(
        "#TITLE Demo\n#RANDOM 2\n#IF 1\n#WAV01 a.wav\n#ENDIF\n#IF 2\n#WAV01 b.wav\n#ENDIF\n#ENDRANDOM\n#BPM 120\n",
    );
    let (lines, _selection) = file.select_lines(|_| 1);
    let commands: Vec<_> = lines
        .iter()
        .filter_map(|line| line.command_name().map(|command| command.text().to_owned()))
        .collect();
    assert_eq!(
        commands,
        vec!["TITLE".to_owned(), "WAV01".to_owned(), "BPM".to_owned()]
    );
}

#[test]
fn control_flow_directives_are_not_selected() {
    let file = file("#RANDOM 2\n#IF 1\n#WAV01 a.wav\n#ENDIF\n#ENDRANDOM\n");
    let (lines, _selection) = file.select_lines(|_| 1);
    assert!(lines.iter().all(|line| !line.is_directive()));
}

#[test]
fn random_zero_yields_nothing_without_panic() {
    let file = file("#RANDOM 0\n#IF 1\n#WAV01 a.wav\n#ENDIF\n#ENDRANDOM\n");
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (Vec::<String>::new(), (0, 1))
    );
}

#[test]
fn switch_zero_yields_nothing_without_panic() {
    let file = file("#SWITCH 0\n#CASE 1\n#WAV01 a.wav\n#ENDSW\n");
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (Vec::<String>::new(), (0, 1))
    );
}

#[test]
fn multiple_chains_in_one_random_block_select_each_chain() {
    let file =
        file("#RANDOM 2\n#IF 1\n#WAV01 a.wav\n#ENDIF\n#IF 2\n#WAV02 b.wav\n#ENDIF\n#ENDRANDOM\n");
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), first_decision(&selection)),
        (vec!["a.wav".to_owned()], (1, 0))
    );
}

#[test]
fn deep_nested_random_10_levels_does_not_panic() {
    let mut text = String::new();
    for _ in 0..10 {
        text.push_str("#RANDOM 1\n#IF 1\n");
    }
    text.push_str("#WAV01 deep.wav\n");
    for _ in 0..10 {
        text.push_str("#ENDIF\n#ENDRANDOM\n");
    }
    let file = file(&text);
    let (lines, selection) = file.select_lines(|_| 1);
    assert_eq!(
        (wavs(&lines), selection.decisions().len()),
        (vec!["deep.wav".to_owned()], 10)
    );
}
