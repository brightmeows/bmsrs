#![expect(missing_docs, reason = "integration test")]

use bmsrs_bms_parser::{Bms, BpmChange, BpmValue, LongNoteEvent, NoteEvent, Position};
use bmsrs_bms_processor::layout::Bme;
use bmsrs_bms_processor::{BmsProcessor, ProcessWarning};

#[test]
fn missing_wav_key_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    // Define WAV "01", but the note references the undefined "ZZ".
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bmsrs_bms_parser::KeyType::Visible,
        wav_id: "ZZ".parse().unwrap(),
    });

    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, ProcessWarning::MissingWavDefinition { .. }))
    );
}

#[test]
fn untermined_long_note_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "a.wav".to_owned());
    // In LNTYPE 1 mode, a single long-note event cannot be paired.
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });

    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, ProcessWarning::UnterminatedLongNote { .. }))
    );
}

#[test]
fn normal_chart_produces_no_warnings() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bmsrs_bms_parser::KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert_eq!(warnings, [] as [bmsrs_bms_processor::ProcessWarning; 0]);
}

#[test]
fn missing_bpm_definition_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    let bpm_id: bmsrs_bms_parser::BpmIndex = "ZZ".parse().unwrap();
    // Reference an undefined BPM key.
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 8),
        value: BpmValue::Reference(bpm_id),
    });

    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, ProcessWarning::MissingBpmDefinition { .. }))
    );
}

#[test]
fn process_still_returns_ok_with_warnings() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    // Reference an undefined WAV.
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bmsrs_bms_parser::KeyType::Visible,
        wav_id: "ZZ".parse().unwrap(),
    });

    let result = BmsProcessor::process::<Bme>(&bms);
    assert!(result.is_ok());
}

#[test]
fn unparsable_bpm_value_produces_warning() {
    // ch03 requires hexadecimal (01-FF); "GG" does not satisfy it. Before the
    // fix, this was dropped silently with no signal.
    let text = "\
#BPM 120
#00003:GG
";
    let (bms, _) =
        bmsrs::bms::from_text(text, |_| 1, bmsrs::bms::ErrorStrategy::CollectAll).unwrap();
    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;

    assert!(warnings.iter().any(|w| matches!(
        w,
        ProcessWarning::UnparsableValue(bmsrs_bms_parser::ChannelValueWarning { measure: 0, .. })
    )));
}
