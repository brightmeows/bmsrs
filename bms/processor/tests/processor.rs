#![expect(missing_docs, reason = "integration test")]

use bms_parser::{BgmEvent, Bms, BpmChange, BpmValue, KeyType, NoteEvent, Position};
use bms_processor::{BmsMapping, BmsProcessor};
use bms_tokenizer::{BmsIndex, BpmTag, LnObjTag, PlayerMode};
use bmsrs_chart::{Beat5k, Beat7k, Beat10k, Beat14k, Note, NoteData, NoteKind};

#[test]
fn beat7k_maps_keys_and_scratch() {
    assert_eq!(Beat7k.map_lane(1, 1), Some(0));
    assert_eq!(Beat7k.map_lane(1, 8), Some(7));
    assert_eq!(Beat7k.map_lane(2, 1), None);
}

#[test]
fn beat5k_maps_five_keys_plus_scratch() {
    assert_eq!(Beat5k.map_lane(1, 1), Some(0));
    assert_eq!(Beat5k.map_lane(1, 5), Some(4));
    assert_eq!(Beat5k.map_lane(1, 8), Some(5));
}

#[test]
fn beat14k_maps_both_players() {
    assert_eq!(Beat14k.map_lane(1, 1), Some(0));
    assert_eq!(Beat14k.map_lane(1, 8), Some(7));
    assert_eq!(Beat14k.map_lane(2, 1), Some(8));
    assert_eq!(Beat14k.map_lane(2, 8), Some(15));
}

#[test]
fn beat10k_maps_split_layout() {
    assert_eq!(Beat10k.map_lane(1, 1), Some(0));
    assert_eq!(Beat10k.map_lane(1, 5), Some(4));
    assert_eq!(Beat10k.map_lane(1, 8), Some(5));
    assert_eq!(Beat10k.map_lane(2, 1), Some(6));
    assert_eq!(Beat10k.map_lane(2, 8), Some(11));
}

#[test]
fn process_basic_note() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

    assert_eq!(chart.notes.len(), 1);
    assert_eq!(chart.notes[0].tick, 0);
    assert_eq!(chart.notes[0].data.lane(), 0);
    assert_eq!(chart.notes[0].data.kind(), NoteKind::Normal);
    assert_eq!(chart.audio_assets.len(), 1);
}

#[test]
fn process_zero_bpm_returns_error() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(0.0);
    let result = BmsProcessor::process(&bms, &Beat7k);
    assert!(result.is_err());
}

#[test]
fn process_negative_bpm_returns_error() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(-10.0);
    let result = BmsProcessor::process(&bms, &Beat7k);
    assert!(result.is_err());
}

#[test]
fn process_bgm_events_mapped() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "bgm.wav".to_owned());
    bms.messages.bgm_events.push(BgmEvent {
        position: Position::new(0, 0, 8),
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

    assert_eq!(chart.bgm.len(), 1);
    assert_eq!(chart.bgm[0].tick, 0);
}

#[test]
fn process_metadata_from_headers() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.metadata.title = Some("Test Song".to_owned());
    bms.metadata.artist = Some("Test Artist".to_owned());
    bms.metadata.genre = Some("Test Genre".to_owned());

    let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

    assert_eq!(chart.metadata.title, "Test Song");
    assert_eq!(chart.metadata.artist, "Test Artist");
    assert_eq!(chart.metadata.genre, "Test Genre");
}

#[test]
fn process_default_single_player_uses_beat7k() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.player = Some(PlayerMode::Single);

    let chart = BmsProcessor::process_default(&bms).unwrap();

    assert_eq!(chart.lane_count, 8);
}

#[test]
fn process_default_double_player_uses_beat14k() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.player = Some(PlayerMode::Double);

    let chart = BmsProcessor::process_default(&bms).unwrap();

    assert_eq!(chart.lane_count, 16);
}

#[test]
fn process_lnobj_produces_long_note() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("02".parse().unwrap(), "b.wav".to_owned());
    let ln_obj: BmsIndex<LnObjTag> = "02".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "02".parse().unwrap(),
    });

    let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

    let lns: Vec<&Note> = chart
        .notes
        .iter()
        .filter(|n| matches!(n.data.kind(), NoteKind::Long { .. }))
        .collect();
    assert_eq!(lns.len(), 1);
    assert_eq!(lns[0].tick, 0);
}

#[test]
fn process_bpm_change_reference_resolved() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    let bpm_id: BmsIndex<BpmTag> = "01".parse().unwrap();
    bms.timing.bpm_defs.insert(bpm_id, 200.0);
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 8),
        value: BpmValue::Reference(bpm_id),
    });

    let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

    assert_eq!(chart.timing.bpm_changes.len(), 1);
    assert!((chart.timing.bpm_changes[0].bpm - 200.0).abs() < 1e-9);
}

#[test]
fn process_bar_lines_generated() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);

    let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

    assert!(!chart.bar_lines.is_empty());
    assert_eq!(chart.bar_lines[0].tick, 0);
    assert_eq!(chart.bar_lines[1].tick, 960);
}

#[test]
fn process_invisible_note_mapped() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "se.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Invisible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

    assert_eq!(chart.notes.len(), 1);
    assert_eq!(chart.notes[0].data.kind(), NoteKind::Invisible);
}
