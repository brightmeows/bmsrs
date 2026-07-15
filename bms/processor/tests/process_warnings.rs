#![expect(missing_docs, reason = "integration test")]

use bms_parser::{Bms, BpmChange, BpmValue, LongNoteEvent, NoteEvent, Position};
use bms_processor::layout::Bme;
use bms_processor::{BmsProcessor, ProcessWarning};

#[test]
fn missing_wav_key_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    // 定义 WAV "01"，但音符引用未定义的 "ZZ"
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bms_parser::KeyType::Visible,
        wav_id: "ZZ".parse().unwrap(),
    });

    let (_chart, warnings) = BmsProcessor::process_with_warnings::<Bme>(&bms).unwrap();
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
    // LNTYPE 1 模式下，单个 long note event 无法配对。
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });

    let (_chart, warnings) = BmsProcessor::process_with_warnings::<Bme>(&bms).unwrap();
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
        key_type: bms_parser::KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let (_chart, warnings) = BmsProcessor::process_with_warnings::<Bme>(&bms).unwrap();
    assert!(warnings.is_empty());
}

#[test]
fn missing_bpm_definition_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    let bpm_id: bms_tokenizer::BpmIndex = "ZZ".parse().unwrap();
    // 引用未定义的 BPM 键
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 8),
        value: BpmValue::Reference(bpm_id),
    });

    let (_chart, warnings) = BmsProcessor::process_with_warnings::<Bme>(&bms).unwrap();
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
    // 引用未定义的 WAV
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bms_parser::KeyType::Visible,
        wav_id: "ZZ".parse().unwrap(),
    });

    let result = BmsProcessor::process_with_warnings::<Bme>(&bms);
    assert!(result.is_ok());
}
