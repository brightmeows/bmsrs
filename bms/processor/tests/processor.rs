#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;

use bms_parser::{BgmEvent, Bms, BpmChange, BpmValue, KeyType, LongNoteEvent, NoteEvent, Position};
use bms_processor::BmsProcessor;
use bms_processor::layout::{Bme, BmsChannel, BmsLayout as _, DscOctFp, Nanasi, Pms, PmsBme};
use bms_tokenizer::{BpmIndex, LnObjIndex};
use bmsrs_chart::mode::{Lane, NoteSide};
use bmsrs_chart::{EventKind, NoteKind};

/// 在测试中构造有效 [`BmsChannel`] 的简写。
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
    // 回归：旧的 Beat7k 匹配 `(1, 1..=8)` 静默丢弃了通道 19（解码轨道 9，
    // 即 KEY7）。Bme 必须将其映射到 Key(7)。
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
    // 1P 通道 11-15 上的 PMS KEY1-5 → Player1
    assert_eq!(Pms::map_channel(ch(1, 1)), Some((NoteSide::P1, key(1))));
    assert_eq!(Pms::map_channel(ch(1, 5)), Some((NoteSide::P1, key(5))));
    // 2P 通道 22-25 上的 PMS KEY6-9 → 仍为 Player1（单人模式）
    assert_eq!(Pms::map_channel(ch(2, 2)), Some((NoteSide::P1, key(6))));
    assert_eq!(Pms::map_channel(ch(2, 5)), Some((NoteSide::P1, key(9))));
    // 通道 21（2P 轨道 1）PMS 不使用。
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
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                side,
                lane,
                kind,
                ext: (),
                ..
            } = &e.kind
            {
                Some((e.tick(), *side, *lane, *kind))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0], (0, NoteSide::P1, key(1), NoteKind::Normal));
    assert_eq!(chart.data.audio_assets.len(), 1);
}

#[test]
fn process_key7_note_lands_on_key_seven() {
    // KEY7 通道 19 丢弃 bug 的端到端回归。
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "k7.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 9, // 通道 19 → KEY7
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let notes: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note { lane, ext: (), .. } = &e.kind {
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
fn process_negative_bpm_is_accepted() {
    // 负 BPM 用于逆向滚动（逆走），时序上使用 |bpm| 计算。
    let mut bms = Bms::default();
    bms.timing.bpm = Some(-120.0);
    let result = BmsProcessor::process::<Bme>(&bms);
    assert!(result.is_ok());
    let chart = result.unwrap();
    // BPM 绝对值 = 120，因此 240 ticks = 0.5s。
    assert_eq!(
        chart.data.timing.tick_to_duration(240),
        std::time::Duration::from_millis(500)
    );
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
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Bgm { .. } = &e.kind {
                Some(e.tick())
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

    assert_eq!(chart.song.title, "Test Song");
    assert_eq!(chart.song.artist, "Test Artist");
    assert_eq!(chart.song.genre, "Test Genre");
}

#[test]
fn process_default_uses_bme_and_maps_both_sides() {
    // process_default 无条件使用 Bme（#PLAYER 头部命令不影响映射）。
    // 2P 音符报告 NoteSide::P2。
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
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note { side, lane, .. } = &e.kind {
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
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Long { .. },
                ..
            } = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(lns.len(), 1);
    assert_eq!(lns[0], 0);
}

/// LNOBJ 模式下 ch51-69 长音通道事件与 LNOBJ 互斥（memo/10 规范未定义），
/// 但不应静默丢弃——作为普通可见音符保留。
#[test]
fn lnobj_preserves_long_note_channel_events_as_normal() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("02".parse().unwrap(), "b.wav".to_owned());
    bms.gameplay.ln_obj = Some("02".parse::<LnObjIndex>().unwrap());
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 4),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let normals: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Normal,
                ..
            } = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(normals, vec![0u64], "ch51-69 事件应作为普通音符保留");
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

    assert_eq!(chart.data.timing.bpm_changes().len(), 1);
    assert!((chart.data.timing.bpm_changes()[0].bpm - 200.0).abs() < 1e-9);
}

#[test]
fn process_bar_lines_generated() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let bar_lines: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| matches!(&e.kind, EventKind::Bar).then_some(e.tick()))
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
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note { kind, ext: (), .. } = &e.kind {
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
    // P1 Scratch
    assert_eq!(DscOctFp::map_channel(ch(1, 6)), Some((NoteSide::P1, sc(1))));
    // P2 脚踏板
    assert_eq!(DscOctFp::map_channel(ch(2, 1)), Some((NoteSide::P2, PEDAL)));
    // P2 第二 Scratch
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

#[test]
fn lnobj_end_marker_plays_bgm() {
    // LNOBJ 终点标记应生成 BGM 事件（按规范）。
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    // 注册起始 WAV 与 LNOBJ WAV。
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.audio
        .wav_files
        .insert("FF".parse().unwrap(), "ln_end.wav".to_owned());
    let ln_obj: LnObjIndex = "FF".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);
    // 长音起点（带 WAV 01 的可见音符）
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    // 长音终点标记（小节 1 处的 LNOBJ WAV FF）
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "FF".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    // 从谱面收集 BGM 事件。
    let bgm_ticks: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Bgm { audio_index } = &e.kind {
                Some((e.tick(), *audio_index))
            } else {
                None
            }
        })
        .collect();

    // 小节 1 处的 LNOBJ 终点标记 → 脉冲 960（240 节拍分辨率 × 4 拍）。
    // 音频索引应为 1（第二个注册的 WAV：ln_end.wav）。
    assert_eq!(bgm_ticks, vec![(960, 1)]);
}

#[test]
fn lnobj_bgm_no_event_for_unmatched_lno() {
    // 当 LNOBJ 标记在同一轨道上没有前导音符时，不形成配对，因此不应
    // 生成 LNOBJ BGM。
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("FF".parse().unwrap(), "orphan.wav".to_owned());
    let ln_obj: LnObjIndex = "FF".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);
    // 同一 (player, lane) 上无前导的 LNOBJ 标记。
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "FF".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let bgm_count = chart
        .data
        .events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Bgm { .. }))
        .count();
    assert_eq!(bgm_count, 0, "orphan LNOBJ marker should not emit BGM");
}
