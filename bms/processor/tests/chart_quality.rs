#![expect(missing_docs, reason = "integration test")]

use bmsrs_bms_model::{BgmEvent, Bms, KeyType, MineEvent, NoteEvent, Position};
use bmsrs_bms_processor::layout::Bme;
use bmsrs_bms_processor::{BmsProcessor, ProcessWarningKind};
use bmsrs_bms_values::PlayerMode;

#[expect(
    clippy::unwrap_used,
    reason = "static fixture construction; a parse failure is a test bug"
)]
fn chart_with_first_side_note() -> Bms {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(60.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "a.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    bms
}

#[test]
fn missing_total_produces_warning_without_span() {
    let bms = chart_with_first_side_note();
    let outcome = BmsProcessor::process::<Bme>(&bms).unwrap();
    let warning = outcome
        .warnings
        .iter()
        .find(|warning| matches!(warning.kind, ProcessWarningKind::MissingTotal))
        .unwrap();
    assert!(warning.span.is_none());
}

#[test]
fn insufficient_total_produces_warning() {
    let mut bms = chart_with_first_side_note();
    bms.gameplay.total = Some(50.0);
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::InsufficientTotal { value } if (value - 50.0).abs() < f64::EPSILON
    )));
}

#[test]
fn healthy_total_produces_no_total_warning() {
    let mut bms = chart_with_first_side_note();
    bms.gameplay.total = Some(100.0);
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(
        !warnings
            .iter()
            .any(|warning| matches!(warning.kind, ProcessWarningKind::MissingTotal))
    );
    assert!(
        !warnings
            .iter()
            .any(|warning| matches!(warning.kind, ProcessWarningKind::InsufficientTotal { .. }))
    );
}

#[test]
fn tail_padding_over_30s_produces_warning() {
    let mut bms = chart_with_first_side_note();
    // At BPM 60 a measure lasts 4 seconds; a BGM event at measure 8 extends
    // the chart end to 36 seconds, so the tail after the note at tick 0 is
    // 36 seconds.
    bms.messages.bgm_events.push(BgmEvent {
        position: Position::new(8, 0, 1),
        wav_id: "01".parse().unwrap(),
    });
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::TailPaddingTooLong {
            last_note_tick: 0,
            ..
        }
    )));
}

#[test]
fn tail_padding_under_30s_produces_no_warning() {
    let mut bms = chart_with_first_side_note();
    bms.messages.bgm_events.push(BgmEvent {
        position: Position::new(2, 0, 1),
        wav_id: "01".parse().unwrap(),
    });
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(
        !warnings
            .iter()
            .any(|warning| matches!(warning.kind, ProcessWarningKind::TailPaddingTooLong { .. }))
    );
}

#[test]
fn mine_only_chart_produces_no_tail_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(60.0);
    bms.messages.mine_events.push(MineEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        raw_value: Some(1),
    });
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(
        !warnings
            .iter()
            .any(|warning| matches!(warning.kind, ProcessWarningKind::TailPaddingTooLong { .. }))
    );
}

#[test]
fn single_player_with_second_side_notes_produces_warning() {
    let mut bms = chart_with_first_side_note();
    bms.gameplay.player = Some(PlayerMode::Single);
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 2,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::PlayerNoteSideMismatch {
            declared_player: 1,
            second_side_present: true,
        }
    )));
}

#[test]
fn couple_player_without_second_side_notes_produces_warning() {
    let mut bms = chart_with_first_side_note();
    bms.gameplay.player = Some(PlayerMode::Couple);
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::PlayerNoteSideMismatch {
            declared_player: 2,
            second_side_present: false,
        }
    )));
}

#[test]
fn player_declaration_matching_notes_produces_no_mismatch_warning() {
    let mut bms = chart_with_first_side_note();
    bms.gameplay.player = Some(PlayerMode::Single);
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(!warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::PlayerNoteSideMismatch { .. }
    )));
}
