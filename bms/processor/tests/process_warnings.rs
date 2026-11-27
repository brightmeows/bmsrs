#![expect(missing_docs, reason = "integration test")]

use bmsrs_bms_model::{Bms, BpmChange, BpmValue, LongNoteEvent, NoteEvent, Position};
use bmsrs_bms_parser::BmsParser;
use bmsrs_bms_processor::layout::Bme;
use bmsrs_bms_processor::{BmsProcessor, ProcessWarningKind};
use bmsrs_bms_syntax::SourceFile;
use bmsrs_bms_values::BmsBase;

#[test]
fn missing_wav_key_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    // Define WAV "01", but the note references the undefined "ZZ".
    bms.audio.wav_sources.insert(
        "01".parse().unwrap(),
        bmsrs_bms_model::WavSource::File("kick.wav".to_owned()),
    );
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bmsrs_bms_model::KeyType::Visible,
        wav_id: "ZZ".parse().unwrap(),
    });

    let warnings = BmsProcessor::process_with(&bms, Bme).unwrap().warnings;
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w.kind, ProcessWarningKind::MissingWavDefinition { .. }))
    );
}

#[test]
fn base16_missing_wav_key_uses_hex_numeric_value() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.base = Some(BmsBase::Base16);
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bmsrs_bms_model::KeyType::Visible,
        wav_id: "FF".parse().unwrap(),
    });

    let warnings = BmsProcessor::process_with(&bms, Bme).unwrap().warnings;
    assert!(warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::MissingWavDefinition { key: 255, .. }
    )));
}

#[test]
fn untermined_long_note_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio.wav_sources.insert(
        "01".parse().unwrap(),
        bmsrs_bms_model::WavSource::File("a.wav".to_owned()),
    );
    // In LNTYPE 1 mode, a single long-note event cannot be paired.
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });

    let warnings = BmsProcessor::process_with(&bms, Bme).unwrap().warnings;
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w.kind, ProcessWarningKind::UnterminatedLongNote { .. }))
    );
}

#[test]
fn normal_chart_produces_no_warnings() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.total = Some(100.0);
    bms.audio.wav_sources.insert(
        "01".parse().unwrap(),
        bmsrs_bms_model::WavSource::File("kick.wav".to_owned()),
    );
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: bmsrs_bms_model::KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let warnings = BmsProcessor::process_with(&bms, Bme).unwrap().warnings;
    assert_eq!(warnings, [] as [bmsrs_bms_processor::ProcessWarning; 0]);
}

#[test]
fn missing_bpm_definition_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    let bpm_id: bmsrs_bms_values::BpmIndex = "ZZ".parse().unwrap();
    // Reference an undefined BPM key.
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 8),
        value: BpmValue::Reference(bpm_id),
    });

    let warnings = BmsProcessor::process_with(&bms, Bme).unwrap().warnings;
    let count = warnings
        .iter()
        .filter(|w| matches!(&w.kind, ProcessWarningKind::MissingBpmDefinition { .. }))
        .count();
    assert_eq!(
        count, 1,
        "a missing BPM definition must warn exactly once, got {warnings:?}"
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
        key_type: bmsrs_bms_model::KeyType::Visible,
        wav_id: "ZZ".parse().unwrap(),
    });

    let result = BmsProcessor::process_with(&bms, Bme);
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
    let source = SourceFile::parse(text);
    let bms = BmsParser::parse(&source.source_file(), &mut |_| 1).bms;
    let warnings = BmsProcessor::process_with(&bms, Bme).unwrap().warnings;

    assert!(warnings.iter().any(|w| matches!(
        w.kind,
        ProcessWarningKind::UnparsableValue(bmsrs_bms_model::ChannelValueWarning {
            measure: 0,
            ..
        })
    )));
}

#[test]
fn negative_volwav_is_dropped_with_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.vol_wav = Some(-5.0);

    let processed = BmsProcessor::process_with(&bms, Bme).unwrap();
    // A negative value used to saturate to Some(0) and silence the chart.
    assert_eq!(processed.value.chart.chart_info().volwav_percent, None);
    assert!(processed.warnings.iter().any(|w| matches!(
        w.kind,
        ProcessWarningKind::NegativeVolWavIgnored { value } if (value - (-5.0)).abs() < f64::EPSILON
    )));
}

#[test]
fn oversized_volwav_is_clamped_with_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.vol_wav = Some(300.0);

    let processed = BmsProcessor::process_with(&bms, Bme).unwrap();
    assert_eq!(processed.value.chart.chart_info().volwav_percent, Some(255));
    assert!(processed.warnings.iter().any(|w| matches!(
        w.kind,
        ProcessWarningKind::VolWavClamped {
            value: 300.0,
            clamped: 255
        }
    )));
}

#[test]
fn in_range_volwav_produces_no_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.vol_wav = Some(120.0);

    let processed = BmsProcessor::process_with(&bms, Bme).unwrap();
    assert_eq!(processed.value.chart.chart_info().volwav_percent, Some(120));
    assert!(!processed.warnings.iter().any(|w| matches!(
        w.kind,
        ProcessWarningKind::NegativeVolWavIgnored { .. } | ProcessWarningKind::VolWavClamped { .. }
    )));
}
