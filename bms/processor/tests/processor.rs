#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;

use bms_parser::{BgmEvent, Bms, BpmChange, BpmValue, KeyType, NoteEvent, Position};
use bms_processor::BmsProcessor;
use bms_processor::layout::{Bme, BmsChannel, BmsLayout as _, DscOctFp, Nanasi, Pms, PmsBme};
use bms_tokenizer::{BpmIndex, LnObjIndex};
use bmsrs_chart::mode::{Lane, PlayerSide};
use bmsrs_chart::{Note, NoteData, NoteDataLike as _, NoteKind};

/// Shorthand to construct a valid [`BmsChannel`] in tests.
fn ch(player: u8, lane: u8) -> BmsChannel {
    BmsChannel::new(player, lane).unwrap_or_else(|| panic!("invalid BMS channel"))
}

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

const fn sc(n: u8) -> Lane {
    Lane::Scratch(nz(n))
}

const PEDAL: Lane = Lane::FootPedal;

#[test]
fn bme_maps_key7_channel_19() {
    // Regression: the old Beat7k match `(1, 1..=8)` silently dropped
    // channel 19 (decoded lane 9, i.e. KEY7). Bme must map it to Key(7).
    assert_eq!(
        Bme::map_channel(ch(1, 9)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(7),
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn bme_maps_both_player_sides() {
    assert_eq!(
        Bme::map_channel(ch(2, 6)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: sc(1),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Bme::map_channel(ch(2, 6)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: Lane::Scratch(NonZeroU8::new(1).unwrap()),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Bme::map_channel(ch(2, 9)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(7),
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn pms_maps_cross_side_channels_to_single_player() {
    // PMS KEY1-5 on 1P channels 11-15 → Player1
    assert_eq!(
        Pms::map_channel(ch(1, 1)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(1),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Pms::map_channel(ch(1, 5)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(5),
            kind: NoteKind::Normal
        })
    );
    // PMS KEY6-9 on 2P channels 22-25 → still Player1 (single-player mode)
    assert_eq!(
        Pms::map_channel(ch(2, 2)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(6),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Pms::map_channel(ch(2, 5)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(9),
            kind: NoteKind::Normal
        })
    );
    // Channel 21 (2P lane 1) is unused by PMS.
    assert_eq!(Pms::map_channel(ch(2, 1)), None);
}

#[test]
fn nanasi_maps_foot_pedal_channel_17() {
    assert_eq!(
        Nanasi::map_channel(ch(1, 7)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: PEDAL,
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        Nanasi::map_channel(ch(2, 7)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: PEDAL,
            kind: NoteKind::Normal
        })
    );
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

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    assert_eq!(chart.notes.len(), 1);
    assert_eq!(chart.notes[0].tick, 0);
    assert_eq!(chart.notes[0].data.side(), PlayerSide::Player1);
    assert_eq!(chart.notes[0].data.lane(), key(1));
    assert_eq!(chart.notes[0].data.kind(), NoteKind::Normal);
    assert_eq!(chart.audio_assets.len(), 1);
}

#[test]
fn process_key7_note_lands_on_key_seven() {
    // End-to-end regression for the KEY7 channel-19 drop bug.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "k7.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 9, // channel 19 → KEY7
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    assert_eq!(chart.notes[0].data.lane(), key(7));
}

#[test]
fn process_zero_bpm_returns_error() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(0.0);
    let result = BmsProcessor::process::<Bme>(&bms);
    assert!(result.is_err());
}

#[test]
fn process_negative_bpm_returns_error() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(-10.0);
    let result = BmsProcessor::process::<Bme>(&bms);
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

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

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

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    assert_eq!(chart.metadata.title, "Test Song");
    assert_eq!(chart.metadata.artist, "Test Artist");
    assert_eq!(chart.metadata.genre, "Test Genre");
}

#[test]
fn process_default_uses_bme_and_maps_both_sides() {
    // process_default is unconditional Bme (the #PLAYER header does not
    // affect mapping). A 2P note reports PlayerSide::Player2.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
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
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 2,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process_default(&bms).unwrap();

    let positions: Vec<(PlayerSide, Lane)> = chart
        .notes
        .iter()
        .map(|n| (n.data.side(), n.data.lane()))
        .collect();
    assert!(positions.contains(&(PlayerSide::Player1, key(1))));
    assert!(positions.contains(&(PlayerSide::Player2, key(1))));
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
    let ln_obj: LnObjIndex = "02".parse().unwrap();
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

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

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
    let bpm_id: BpmIndex = "01".parse().unwrap();
    bms.timing.bpm_defs.insert(bpm_id, 200.0);
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 8),
        value: BpmValue::Reference(bpm_id),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    assert_eq!(chart.timing.bpm_changes.len(), 1);
    assert!((chart.timing.bpm_changes[0].bpm - 200.0).abs() < 1e-9);
}

#[test]
fn process_bar_lines_generated() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

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

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    assert_eq!(chart.notes.len(), 1);
    assert_eq!(chart.notes[0].data.kind(), NoteKind::Invisible);
}

#[test]
fn pms_bme_reinterprets_16_17_as_keys() {
    assert_eq!(
        PmsBme::map_channel(ch(1, 8)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(6),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        PmsBme::map_channel(ch(1, 6)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key(8),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        PmsBme::map_channel(ch(2, 7)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(9),
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn dsc_oct_fp_maps_dual_scratch_and_pedal() {
    // P1 scratch
    assert_eq!(
        DscOctFp::map_channel(ch(1, 6)),
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: sc(1),
            kind: NoteKind::Normal
        })
    );
    // P2 foot pedal
    assert_eq!(
        DscOctFp::map_channel(ch(2, 1)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: PEDAL,
            kind: NoteKind::Normal
        })
    );
    // P2 second scratch
    assert_eq!(
        DscOctFp::map_channel(ch(2, 6)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: sc(2),
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn pms_bme_maps_second_player_side() {
    assert_eq!(
        PmsBme::map_channel(ch(2, 1)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(1),
            kind: NoteKind::Normal
        })
    );
    assert_eq!(
        PmsBme::map_channel(ch(2, 9)),
        Some(NoteData {
            side: PlayerSide::Player2,
            lane: key(7),
            kind: NoteKind::Normal
        })
    );
}

#[test]
fn bms_channel_rejects_invalid_input() {
    assert_eq!(BmsChannel::new(3, 1), None);
    assert_eq!(BmsChannel::new(0, 1), None);
    assert_eq!(BmsChannel::new(1, 0), None);
    assert_eq!(BmsChannel::new(1, 10), None);
}
