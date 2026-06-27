#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;

use bms_parser::{BgmEvent, Bms, BpmChange, BpmValue, KeyType, NoteEvent, Position};
use bms_processor::BmsProcessor;
use bms_processor::layout::{Bme, BmsChannel, BmsLayout as _, DscOctFp, Nanasi, Pms, PmsBme};
use bms_tokenizer::{BpmIndex, LnObjIndex};
use bmsrs_chart::mode::{Lane, NoteSide};
use bmsrs_chart::{Event, NoteKind};

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
    assert_eq!(Bme::map_channel(ch(1, 9)), Some((NoteSide::P1, key(7))));
}

#[test]
fn bme_maps_both_player_sides() {
    assert_eq!(Bme::map_channel(ch(2, 6)), Some((NoteSide::P2, sc(1))));
    assert_eq!(
        Bme::map_channel(ch(2, 6)),
        Some((NoteSide::P2, Lane::Scratch(NonZeroU8::new(1).unwrap())))
    );
    assert_eq!(Bme::map_channel(ch(2, 9)), Some((NoteSide::P2, key(7))));
}

#[test]
fn pms_maps_cross_side_channels_to_single_player() {
    // PMS KEY1-5 on 1P channels 11-15 → Player1
    assert_eq!(Pms::map_channel(ch(1, 1)), Some((NoteSide::P1, key(1))));
    assert_eq!(Pms::map_channel(ch(1, 5)), Some((NoteSide::P1, key(5))));
    // PMS KEY6-9 on 2P channels 22-25 → still Player1 (single-player mode)
    assert_eq!(Pms::map_channel(ch(2, 2)), Some((NoteSide::P1, key(6))));
    assert_eq!(Pms::map_channel(ch(2, 5)), Some((NoteSide::P1, key(9))));
    // Channel 21 (2P lane 1) is unused by PMS.
    assert_eq!(Pms::map_channel(ch(2, 1)), None);
}

#[test]
fn nanasi_maps_foot_pedal_channel_17() {
    assert_eq!(Nanasi::map_channel(ch(1, 7)), Some((NoteSide::P1, PEDAL)));
    assert_eq!(Nanasi::map_channel(ch(2, 7)), Some((NoteSide::P2, PEDAL)));
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

    let notes: Vec<_> = chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Note {
                tick,
                side,
                lane,
                kind,
                ext: (),
                ..
            } = e
            {
                Some((*tick, *side, *lane, *kind))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0], (0, NoteSide::P1, key(1), NoteKind::Normal));
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

    let notes: Vec<_> = chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Note { lane, ext: (), .. } = e {
                Some(*lane)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes[0], key(7));
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

    let bgm: Vec<_> = chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Bgm { tick, .. } = e {
                Some(*tick)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bgm.len(), 1);
    assert_eq!(bgm[0], 0);
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
    // affect mapping). A 2P note reports NoteSide::P2.
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

    let positions: Vec<(NoteSide, Lane)> = chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Note { side, lane, .. } = e {
                Some((*side, *lane))
            } else {
                None
            }
        })
        .collect();
    assert!(positions.contains(&(NoteSide::P1, key(1))));
    assert!(positions.contains(&(NoteSide::P2, key(1))));
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

    let lns: Vec<_> = chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Note {
                tick,
                kind: NoteKind::Long { .. },
                ..
            } = e
            {
                Some(*tick)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(lns.len(), 1);
    assert_eq!(lns[0], 0);
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

    let bar_lines: Vec<_> = chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Bar { tick } = e {
                Some(*tick)
            } else {
                None
            }
        })
        .collect();
    assert!(!bar_lines.is_empty());
    assert_eq!(bar_lines[0], 0);
    assert_eq!(bar_lines[1], 960);
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

    let notes: Vec<_> = chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Note { kind, ext: (), .. } = e {
                Some(*kind)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0], NoteKind::Invisible);
}

#[test]
fn pms_bme_reinterprets_16_17_as_keys() {
    assert_eq!(PmsBme::map_channel(ch(1, 8)), Some((NoteSide::P1, key(6))));
    assert_eq!(PmsBme::map_channel(ch(1, 6)), Some((NoteSide::P1, key(8))));
    assert_eq!(PmsBme::map_channel(ch(2, 7)), Some((NoteSide::P2, key(9))));
}

#[test]
fn dsc_oct_fp_maps_dual_scratch_and_pedal() {
    // P1 scratch
    assert_eq!(DscOctFp::map_channel(ch(1, 6)), Some((NoteSide::P1, sc(1))));
    // P2 foot pedal
    assert_eq!(DscOctFp::map_channel(ch(2, 1)), Some((NoteSide::P2, PEDAL)));
    // P2 second scratch
    assert_eq!(DscOctFp::map_channel(ch(2, 6)), Some((NoteSide::P2, sc(2))));
}

#[test]
fn pms_bme_maps_second_player_side() {
    assert_eq!(PmsBme::map_channel(ch(2, 1)), Some((NoteSide::P2, key(1))));
    assert_eq!(PmsBme::map_channel(ch(2, 9)), Some((NoteSide::P2, key(7))));
}

#[test]
fn bms_channel_rejects_invalid_input() {
    assert_eq!(BmsChannel::new(3, 1), None);
    assert_eq!(BmsChannel::new(0, 1), None);
    assert_eq!(BmsChannel::new(1, 0), None);
    assert_eq!(BmsChannel::new(1, 10), None);
}
