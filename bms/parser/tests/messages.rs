//! `Messages` 的集成测试（通过公开 API 进行事件解析）。
//!
//! 这些测试仅通过公开 API 消费 `Messages` 结构体，使用
//! `concat_raw` + `finalize`。

use bms_parser::*;
use bms_tokenizer::{BmsBase, BmsChannel, BmsToken, BmsTokenizer, BpmIndex, WavIndex};
use bmsrs_chart::BgaLayer;

/// 辅助函数：通过 Messages 解析单条标准消息行（`#xxxYY:body`），使用 Base36。
fn parse_one(line: &str) -> Messages {
    parse_one_with_base(line, BmsBase::Base36)
}

/// 辅助函数：通过 Messages 解析单条标准消息行，并指定进制。
fn parse_one_with_base(line: &str, base: BmsBase) -> Messages {
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
    msgs.finalize(base);
    msgs
}

#[test]
fn position_new_and_fraction() {
    let pos = Position::new(1, 1, 4);
    assert_eq!(pos.measure(), 1);
    assert_eq!(pos.numer(), 1);
    assert_eq!(pos.denom(), 4);
    assert!((pos.fraction() - 0.25).abs() < f64::EPSILON);
}

#[test]
#[should_panic(expected = "Position denom must be non-zero")]
fn position_zero_denom_panics() {
    let _position = Position::new(0, 5, 0);
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
        raw_value: Some(5),
    };
    assert_eq!(ev.player, 2);
    assert_eq!(ev.lane, 5);
    assert_eq!(ev.raw_value, Some(5));
}

#[test]
fn stp_event_fields() {
    let pos = Position::new(1, 128, 1000);
    let ev = StpEvent {
        position: pos,
        duration_ms: 500.0,
    };
    assert!((ev.duration_ms - 500.0).abs() < f64::EPSILON);
    assert_eq!(ev.position.numer(), 128);
    assert_eq!(ev.position.denom(), 1000);
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
    assert_eq!(msgs.bgm_events[0].position.numer(), 0);
    assert_eq!(msgs.bgm_events[0].position.denom(), 3);
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
    assert_eq!(msgs.long_note_events[0].position.numer(), 0);
    assert_eq!(msgs.long_note_events[1].position.numer(), 1);
}

#[test]
fn mine_events_parsed() {
    let msgs = parse_one("#001D1:01");
    assert_eq!(msgs.mine_events.len(), 1);
    assert_eq!(msgs.mine_events[0].player, 1);
    assert_eq!(msgs.mine_events[0].lane, 1);
}

#[test]
fn mine_raw_value_decoded() {
    // #001D1:01 → base36 "01" = 1
    let msgs = parse_one("#001D1:01");
    assert_eq!(msgs.mine_events[0].raw_value, Some(1));
}

#[test]
fn mine_raw_value_half_health() {
    // 1E (36进制) = 50
    let msgs = parse_one("#001D1:1E");
    assert_eq!(msgs.mine_events[0].raw_value, Some(50));
}

#[test]
fn mine_raw_value_instant_kill() {
    // ZZ = 1295（即死标记）
    let msgs = parse_one("#001D1:ZZ");
    assert_eq!(msgs.mine_events[0].raw_value, Some(1295));
}

#[test]
fn mine_zero_entries_filtered() {
    // "00" entries are filtered out (no mine at that position).
    let msgs = parse_one("#001D1:0000");
    assert_eq!(msgs.mine_events.len(), 0);
}

#[test]
fn mine_raw_value_mixed_zero_and_real() {
    // 0A(=10) 00 ZZ(=1295) → first: 10, second: 1295, 00 skipped.
    let msgs = parse_one("#001D1:0A00ZZ");
    assert_eq!(msgs.mine_events.len(), 2);
    assert_eq!(msgs.mine_events[0].raw_value, Some(10));
    assert_eq!(msgs.mine_events[1].raw_value, Some(1295));
}

/// 回归：Base62 模式下地雷伤害值仍按 base36 解码——
/// `"10"` 应为 base36 = 36，而非十进制 10。
/// 见 `bms/ext/base62-format.md` L127-129：地雷值不受 `#BASE 62` 影响。
#[test]
fn mine_raw_value_base62_mode_still_base36() {
    let msgs = parse_one_with_base("#001D1:10", BmsBase::Base62);
    assert_eq!(msgs.mine_events[0].raw_value, Some(36));
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
    assert_eq!(msgs.speed_events[0].position.numer(), 0);
    assert_eq!(msgs.speed_events[1].position.numer(), 1);
    assert_eq!(msgs.speed_events[2].position.numer(), 2);
}

#[test]
fn speed_event_zero_values_preserved() {
    // SPEED 通道过滤 "00"（无操作位置），仅保留有效索引。
    let msgs = parse_one("#001SP:AA00BB");
    assert_eq!(msgs.speed_events.len(), 2);
    assert_eq!(msgs.speed_events[0].speed_id, "AA".try_into().unwrap());
    assert_eq!(msgs.speed_events[1].speed_id, "BB".try_into().unwrap());
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

#[test]
fn finalize_idempotent_does_not_duplicate_non_event_data() {
    // chA6（选项）走 finalize_merged，产生 non_event_data 条目。
    let mut msgs = parse_one("#001A6:0100000000000000");
    let first_count = msgs.non_event_data.len();
    assert!(first_count > 0, "chA6 应产生 non_event_data 条目");
    // 重复调用 finalize 不应追加重复条目
    msgs.finalize(BmsBase::Base36);
    assert_eq!(msgs.non_event_data.len(), first_count);
}

#[test]
fn base62_note_lowercase_wav_index_preserved() {
    let msgs = parse_one_with_base("#00111:aa", BmsBase::Base62);
    assert_eq!(
        msgs.note_events[0].wav_id,
        WavIndex::try_from("aa").unwrap()
    );
}

#[test]
fn base62_note_uppercase_wav_index_preserved() {
    let msgs = parse_one_with_base("#00111:AA", BmsBase::Base62);
    assert_eq!(
        msgs.note_events[0].wav_id,
        WavIndex::try_from("AA").unwrap()
    );
}

#[test]
fn base62_lowercase_and_uppercase_wav_indices_are_distinct() {
    let msgs_lower = parse_one_with_base("#00111:aa", BmsBase::Base62);
    let msgs_upper = parse_one_with_base("#00111:AA", BmsBase::Base62);
    assert_ne!(
        msgs_lower.note_events[0].wav_id,
        msgs_upper.note_events[0].wav_id
    );
}

// 扩展音符通道（pomu2 系 `1A`–`1Z` 等）

/// 扩展可见通道 `1A` 解码出 lane 10。
#[test]
fn extended_visible_channel_1a_decodes_lane_10() {
    let msgs = parse_one("#0011A:01");
    assert_eq!(msgs.note_events.len(), 1);
    assert_eq!(msgs.note_events[0].player, 1);
    assert_eq!(msgs.note_events[0].lane, 10);
    assert_eq!(msgs.note_events[0].key_type, KeyType::Visible);
}

/// 扩展可见通道 `1Z` 解码出 lane 35。
#[test]
fn extended_visible_channel_1z_decodes_lane_35() {
    let msgs = parse_one("#0011Z:01");
    assert_eq!(msgs.note_events.len(), 1);
    assert_eq!(msgs.note_events[0].player, 1);
    assert_eq!(msgs.note_events[0].lane, 35);
}

/// 2P 扩展可见通道 `2A` 解码出 player 2, lane 10。
#[test]
fn extended_visible_channel_2a_decodes_player_2_lane_10() {
    let msgs = parse_one("#0012A:01");
    assert_eq!(msgs.note_events.len(), 1);
    assert_eq!(msgs.note_events[0].player, 2);
    assert_eq!(msgs.note_events[0].lane, 10);
    assert_eq!(msgs.note_events[0].key_type, KeyType::Visible);
}

/// 扩展不可见通道 `3A`/`4A` 解码出 lane 10 的不可见音符。
#[test]
fn extended_invisible_channel_3a_decodes_lane_10() {
    let msgs = parse_one("#0013A:01");
    assert_eq!(msgs.note_events.len(), 1);
    assert_eq!(msgs.note_events[0].player, 1);
    assert_eq!(msgs.note_events[0].lane, 10);
    assert_eq!(msgs.note_events[0].key_type, KeyType::Invisible);

    let msgs_4a = parse_one("#0014A:01");
    assert_eq!(msgs_4a.note_events[0].player, 2);
    assert_eq!(msgs_4a.note_events[0].key_type, KeyType::Invisible);
}

/// 扩展长音通道 `5A`/`6A` 解码出 lane 10 的长音。
#[test]
fn extended_long_note_channel_5a_decodes_lane_10() {
    let msgs = parse_one("#0015A:01");
    assert_eq!(msgs.long_note_events.len(), 1);
    assert_eq!(msgs.long_note_events[0].player, 1);
    assert_eq!(msgs.long_note_events[0].lane, 10);

    let msgs_6a = parse_one("#0016A:01");
    assert_eq!(msgs_6a.long_note_events[0].player, 2);
    assert_eq!(msgs_6a.long_note_events[0].lane, 10);
}

/// 预留通道 `"10"`/`"20"`（lane 0）不产生 `NoteEvent`。
#[test]
fn reserved_channel_10_produces_no_note() {
    let msgs_10 = parse_one("#00110:01");
    assert_eq!(msgs_10.note_events.len(), 0);
    let msgs_20 = parse_one("#00120:01");
    assert_eq!(msgs_20.note_events.len(), 0);
}

/// 回归：地雷通道 `D0`（lane 0）不产生 `MineEvent`。
#[test]
fn mine_channel_d0_produces_no_event() {
    let msgs = parse_one("#001D0:01");
    assert_eq!(msgs.mine_events.len(), 0);
}

/// 回归：假设性的 `DA`（lane 10）不产生 `MineEvent`——
/// `classify_channel` 已将其归为 `Unknown`，仅 `D1`–`D9` 有效。
#[test]
fn mine_channel_da_produces_no_event() {
    let msgs = parse_one("#001DA:01");
    assert_eq!(msgs.mine_events.len(), 0);
}

/// 回归：标准地雷 `D1`–`D9` 仍正常产生 `MineEvent`（lane 1–9）。
#[test]
fn mine_channel_d1_still_works() {
    let msgs = parse_one("#001D1:01");
    assert_eq!(msgs.mine_events.len(), 1);
    assert_eq!(msgs.mine_events[0].player, 1);
    assert_eq!(msgs.mine_events[0].lane, 1);
}
