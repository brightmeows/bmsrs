//! `Messages` 的集成测试（通过公开 API 进行事件解析）。
//!
//! 这些测试仅通过公开 API 消费 `Messages` 结构体，使用
//! `concat_raw` + `finalize`。

use bms_parser::*;
use bms_tokenizer::{BmsBase, BmsChannel, BmsToken, BmsTokenizer, BpmIndex, WavIndex};
use bmsrs_chart::BgaLayer;

/// 辅助函数：通过 Messages 解析单条标准消息行（`#xxxYY:body`）。
fn parse_one(line: &str) -> Messages {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, &str>(line)
        .into_iter()
        .filter_map(|(_, res)| res.ok())
        .collect();
    let mut msgs = Messages::default();
    for token in &tokens {
        if let BmsToken::Message(msg) = token {
            msgs.concat_raw(msg);
        }
    }
    msgs.finalize(BmsBase::Base36);
    msgs
}

#[test]
fn position_new_and_fraction() {
    let pos = Position::new(1, 1, 4);
    assert_eq!(pos.measure, 1);
    assert_eq!(pos.numer, 1);
    assert_eq!(pos.denom, 4);
    assert!((pos.fraction() - 0.25).abs() < f64::EPSILON);
}

#[test]
fn position_denom_zero_returns_zero() {
    let pos = Position::new(0, 5, 0);
    assert!(pos.fraction().abs() < f64::EPSILON);
}

#[test]
fn position_at_measure_start() {
    let pos = Position::new(2, 0, 3);
    assert!(pos.fraction().abs() < f64::EPSILON);
}

#[test]
fn position_at_measure_end() {
    // 3/3 位于小节边界上；fraction 为 1.0
    let pos = Position::new(1, 3, 3);
    assert!((pos.fraction() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn bgm_event_fields() {
    let pos = Position::new(1, 0, 1);
    let id: WavIndex = "01".try_into().unwrap();
    let ev = BgmEvent {
        position: pos,
        wav_id: id,
    };
    assert_eq!(ev.position, pos);
    assert_eq!(ev.wav_id, id);
}

#[test]
fn note_event_visible() {
    let pos = Position::new(1, 0, 1);
    let id: WavIndex = "AA".try_into().unwrap();
    let ev = NoteEvent {
        position: pos,
        player: 1,
        lane: 3,
        key_type: KeyType::Visible,
        wav_id: id,
    };
    assert_eq!(ev.player, 1);
    assert_eq!(ev.lane, 3);
    assert!(matches!(ev.key_type, KeyType::Visible));
}

#[test]
fn bpm_change_absolute() {
    let pos = Position::new(0, 0, 1);
    let ev = BpmChange {
        position: pos,
        value: BpmValue::Absolute(180.0),
    };
    assert!(
        (match ev.value {
            BpmValue::Absolute(v) => v,
            BpmValue::Reference(_) => 0.0,
        } - 180.0)
            .abs()
            < f64::EPSILON
    );
}

#[test]
fn bpm_change_reference() {
    let pos = Position::new(0, 0, 1);
    let id: BpmIndex = "05".try_into().unwrap();
    let ev = BpmChange {
        position: pos,
        value: BpmValue::Reference(id),
    };
    assert!(matches!(ev.value, BpmValue::Reference(_)));
}

#[test]
fn bga_layer_variants() {
    assert!(matches!(BgaLayer::Base, BgaLayer::Base));
    assert!(matches!(BgaLayer::Poor, BgaLayer::Poor));
    assert!(matches!(BgaLayer::Layer, BgaLayer::Layer));
    assert!(matches!(BgaLayer::Layer2, BgaLayer::Layer2));
}

#[test]
fn key_type_variants() {
    assert!(matches!(KeyType::Visible, KeyType::Visible));
    assert!(matches!(KeyType::Invisible, KeyType::Invisible));
}

#[test]
fn mine_event_fields() {
    let pos = Position::new(2, 1, 4);
    let ev = MineEvent {
        position: pos,
        player: 2,
        lane: 5,
        damage: 2.5,
    };
    assert_eq!(ev.player, 2);
    assert_eq!(ev.lane, 5);
    assert!((ev.damage - 2.5).abs() < f64::EPSILON);
}

#[test]
fn stp_event_fields() {
    let pos = Position::new(1, 128, 1000);
    let ev = StpEvent {
        position: pos,
        duration_ms: 500.0,
    };
    assert!((ev.duration_ms - 500.0).abs() < f64::EPSILON);
    assert_eq!(ev.position.numer, 128);
    assert_eq!(ev.position.denom, 1000);
}

#[test]
fn measure_length_fields() {
    let ml = MeasureLength {
        measure: 1,
        length_ratio: 2.0,
    };
    assert_eq!(ml.measure, 1);
    assert!((ml.length_ratio - 2.0).abs() < f64::EPSILON);
}

// 事件解析集成测试

#[test]
fn bgm_events_parsed() {
    let msgs = parse_one("#00101:AABBCC");
    assert_eq!(msgs.bgm_events.len(), 3);
    assert_eq!(msgs.bgm_events[0].wav_id, "AA".try_into().unwrap());
    assert_eq!(msgs.bgm_events[1].wav_id, "BB".try_into().unwrap());
    assert_eq!(msgs.bgm_events[2].wav_id, "CC".try_into().unwrap());
    assert_eq!(msgs.bgm_events[0].position.numer, 0);
    assert_eq!(msgs.bgm_events[0].position.denom, 3);
}

#[test]
fn note_events_parsed() {
    let msgs = parse_one("#00111:1122");
    assert_eq!(msgs.note_events.len(), 2);
    assert_eq!(msgs.note_events[0].player, 1);
    assert_eq!(msgs.note_events[0].lane, 1);
    assert_eq!(msgs.note_events[0].key_type, KeyType::Visible);
    assert_eq!(msgs.note_events[1].player, 1);
    assert_eq!(msgs.note_events[1].lane, 1);
}

#[test]
fn long_note_events_parsed() {
    let msgs = parse_one("#00151:0102");
    assert_eq!(msgs.long_note_events.len(), 2);
    assert_eq!(msgs.long_note_events[0].player, 1);
    assert_eq!(msgs.long_note_events[0].lane, 1);
    assert_eq!(msgs.long_note_events[1].lane, 1); // 同一轨道，不同位置
    assert_eq!(msgs.long_note_events[0].position.numer, 0);
    assert_eq!(msgs.long_note_events[1].position.numer, 1);
}

#[test]
fn mine_events_parsed() {
    let msgs = parse_one("#001D1:01");
    assert_eq!(msgs.mine_events.len(), 1);
    assert_eq!(msgs.mine_events[0].player, 1);
    assert_eq!(msgs.mine_events[0].lane, 1);
}

#[test]
fn mine_damage_decoded() {
    // #001D1:01 → damage = 1/2 = 0.5
    let msgs = parse_one("#001D1:01");
    assert!((msgs.mine_events[0].damage - 0.5).abs() < f64::EPSILON);
}

#[test]
fn mine_damage_half_health() {
    // 1E (36进制) = 50 → damage = 50/2 = 25.0
    let msgs = parse_one("#001D1:1E");
    assert!((msgs.mine_events[0].damage - 25.0).abs() < f64::EPSILON);
}

#[test]
fn mine_damage_instant_kill() {
    // ZZ = 1295 → damage = inf（即死）
    let msgs = parse_one("#001D1:ZZ");
    assert!(msgs.mine_events[0].damage.is_infinite());
}

#[test]
fn mine_zero_entries_filtered() {
    // "00" entries are filtered out (no mine at that position).
    let msgs = parse_one("#001D1:0000");
    assert_eq!(msgs.mine_events.len(), 0);
}

#[test]
fn mine_damage_mixed_zero_and_real() {
    // 0A(=10) 00 ZZ(=1295) → first: 10/2=5, second: inf, 00 skipped.
    let msgs = parse_one("#001D1:0A00ZZ");
    assert_eq!(msgs.mine_events.len(), 2);
    assert!((msgs.mine_events[0].damage - 5.0).abs() < f64::EPSILON);
    assert!(msgs.mine_events[1].damage.is_infinite());
}

#[test]
fn bpm_absolute_parsed() {
    let msgs = parse_one("#00103:7F");
    assert_eq!(msgs.bpm_changes.len(), 1);
    assert_eq!(msgs.bpm_changes[0].value, BpmValue::Absolute(127.0));
}

#[test]
fn bpm_reference_parsed() {
    let msgs = parse_one("#00108:05");
    assert_eq!(msgs.bpm_changes.len(), 1);
    assert_eq!(
        msgs.bpm_changes[0].value,
        BpmValue::Reference("05".try_into().unwrap())
    );
}

#[test]
fn stop_event_parsed() {
    let msgs = parse_one("#00109:01");
    assert_eq!(msgs.stop_events.len(), 1);
    assert_eq!(msgs.stop_events[0].stop_id, "01".try_into().unwrap());
}

#[test]
fn scroll_event_parsed() {
    let msgs = parse_one("#000SC:ZZ");
    assert_eq!(msgs.scroll_events.len(), 1);
    assert_eq!(msgs.scroll_events[0].scroll_id, "ZZ".try_into().unwrap());
}

#[test]
fn speed_event_parsed() {
    let msgs = parse_one("#001SP:01");
    assert_eq!(msgs.speed_events.len(), 1);
    assert_eq!(msgs.speed_events[0].speed_id, "01".try_into().unwrap());
}

#[test]
fn speed_event_multiple_values() {
    let msgs = parse_one("#001SP:010203");
    assert_eq!(msgs.speed_events.len(), 3);
    assert_eq!(msgs.speed_events[0].position.numer, 0);
    assert_eq!(msgs.speed_events[1].position.numer, 1);
    assert_eq!(msgs.speed_events[2].position.numer, 2);
}

#[test]
fn speed_event_zero_values_preserved() {
    // SPEED 通道不过滤 "00" —— 它遵循非 BGM 的合并语义。
    let msgs = parse_one("#001SP:AA00BB");
    assert_eq!(msgs.speed_events.len(), 3);
    assert_eq!(msgs.speed_events[0].speed_id, "AA".try_into().unwrap());
    assert_eq!(msgs.speed_events[1].speed_id, "00".try_into().unwrap());
    assert_eq!(msgs.speed_events[2].speed_id, "BB".try_into().unwrap());
}

#[test]
fn bga_event_layer2_parsed() {
    let msgs = parse_one("#0010A:03");
    assert_eq!(msgs.bga_events.len(), 1);
    assert_eq!(msgs.bga_events[0].layer, BgaLayer::Layer2);
    assert_eq!(msgs.bga_events[0].bmp_id, "03".try_into().unwrap());
}

#[test]
fn bga_event_base_parsed() {
    let msgs = parse_one("#00104:03");
    assert_eq!(msgs.bga_events.len(), 1);
    assert_eq!(msgs.bga_events[0].layer, BgaLayer::Base);
    assert_eq!(msgs.bga_events[0].bmp_id, "03".try_into().unwrap());
}

#[test]
fn bga_event_poor_parsed() {
    let msgs = parse_one("#00106:AA");
    assert_eq!(msgs.bga_events.len(), 1);
    assert_eq!(msgs.bga_events[0].layer, BgaLayer::Poor);
}

#[test]
fn bga_event_layer_parsed() {
    let msgs = parse_one("#00107:BB");
    assert_eq!(msgs.bga_events.len(), 1);
    assert_eq!(msgs.bga_events[0].layer, BgaLayer::Layer);
}

#[test]
fn measure_length_parsed() {
    // #00102:2 means 8/4 (2x a standard 4/4 measure).
    let msgs = parse_one("#00102:2");
    assert_eq!(msgs.measure_lengths.len(), 1);
    assert_eq!(msgs.measure_lengths[0].measure, 1);
    assert!((msgs.measure_lengths[0].length_ratio - 2.0).abs() < f64::EPSILON);
}

#[test]
fn measure_length_fractional() {
    // #00102:0.75 means 3/4.
    let msgs = parse_one("#00102:0.75");
    assert_eq!(msgs.measure_lengths.len(), 1);
    assert!((msgs.measure_lengths[0].length_ratio - 0.75).abs() < f64::EPSILON);
}

#[test]
fn measure_length_default_on_invalid() {
    // 无效值被静默忽略。
    let msgs = parse_one("#00102:notanumber");
    assert!(msgs.measure_lengths.is_empty());
}

#[test]
fn empty_values_no_events() {
    let msgs = parse_one("#00111:");
    assert_eq!(msgs.note_events.len(), 0);
    assert!(msgs.raw.contains_key(&1));
}

#[test]
fn unknown_channel_raw_only() {
    let msgs = parse_one("#0010F:AA");
    assert_eq!(msgs.bgm_events.len(), 0);
    assert_eq!(msgs.note_events.len(), 0);
    let ch = BmsChannel::from_raw("0F").unwrap();
    assert_eq!(
        msgs.raw
            .get(&1)
            .and_then(|m| m.get(&ch))
            .and_then(|v| v.first().map(String::as_str)),
        Some("AA")
    );
    // 单行存储。
    assert_eq!(
        msgs.raw.get(&1).and_then(|m| m.get(&ch).map(Vec::len)),
        Some(1)
    );
}

#[test]
fn note_events_filter_zero_entries() {
    // "00" entries should not produce NoteEvent.
    let msgs = parse_one("#00111:AA00BB");
    assert_eq!(msgs.note_events.len(), 2);
    assert_eq!(msgs.note_events[0].wav_id, "AA".try_into().unwrap());
    assert_eq!(msgs.note_events[1].wav_id, "BB".try_into().unwrap());
}

#[test]
fn note_events_only_zero_entries_produces_nothing() {
    let msgs = parse_one("#00111:0000");
    assert_eq!(msgs.note_events.len(), 0);
}

#[test]
fn two_player_notes_parsed() {
    // 通道 21 = 2P 可见按键 1
    let msgs = parse_one("#00121:1122");
    assert_eq!(msgs.note_events.len(), 2);
    assert_eq!(msgs.note_events[0].player, 2);
    assert_eq!(msgs.note_events[0].lane, 1);
    assert_eq!(msgs.note_events[0].key_type, KeyType::Visible);

    // 通道 41 = 2P 不可见按键 1
    let msgs_41 = parse_one("#00141:3344");
    assert_eq!(msgs_41.note_events.len(), 2);
    assert_eq!(msgs_41.note_events[0].player, 2);
    assert_eq!(msgs_41.note_events[0].lane, 1);
    assert_eq!(msgs_41.note_events[0].key_type, KeyType::Invisible);
}

// BPM 00 过滤（通道 03）

#[test]
fn bpm_00_rest_skipped() {
    // "00" in channel 03 = rest — no BPM change emitted.
    let msgs = parse_one("#00103:00");
    assert_eq!(msgs.bpm_changes.len(), 0);
}

#[test]
fn bpm_00_only_all_filtered() {
    // 全部为 "00" 值 → 无 BPM 变更。
    let msgs = parse_one("#00103:00000000");
    assert_eq!(msgs.bpm_changes.len(), 0);
}

#[test]
fn bpm_00_mixed_with_real() {
    // "00" in between: 7F (127) 00 AA (170) → only 2 BPM changes.
    let msgs = parse_one("#00103:7F00AA");
    assert_eq!(msgs.bpm_changes.len(), 2);
    assert_eq!(msgs.bpm_changes[0].value, BpmValue::Absolute(127.0));
    assert_eq!(msgs.bpm_changes[1].value, BpmValue::Absolute(170.0));
}
