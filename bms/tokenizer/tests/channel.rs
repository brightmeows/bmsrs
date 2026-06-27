//! [`BmsChannel`] 分类与公共 API 的集成测试。
//!
//! 这些测试仅通过公共 API 消费 crate，与
//! 外部消费方的使用方式完全一致。

use bms_tokenizer::{BmsChannel, ChannelIndex};

// 辅助函数

#[expect(clippy::expect_used, reason = "test helper panics on invalid input")]
fn ch(s: &str) -> BmsChannel {
    BmsChannel::from_raw(s).expect("valid channel string")
}

#[expect(clippy::unwrap_used, reason = "test helper panics on invalid input")]
fn idx(s: &str) -> ChannelIndex {
    let upper = s.to_ascii_uppercase();
    upper.as_str().try_into().unwrap()
}

// 已知十六进制通道

#[test]
fn bgm() {
    assert_eq!(ch("01"), BmsChannel::Bgm);
    assert_eq!(ch("1"), BmsChannel::Bgm);
}

#[test]
fn measure_length() {
    assert_eq!(ch("02"), BmsChannel::MeasureLength);
    assert_eq!(ch("2"), BmsChannel::MeasureLength);
}

#[test]
fn bpm_change() {
    assert_eq!(ch("03"), BmsChannel::BpmChange);
}

#[test]
fn bga_base() {
    assert_eq!(ch("04"), BmsChannel::BgaBase);
}

#[test]
fn seek() {
    assert_eq!(ch("05"), BmsChannel::Seek);
}

#[test]
fn bga_poor() {
    assert_eq!(ch("06"), BmsChannel::BgaPoor);
}

#[test]
fn bga_layer() {
    assert_eq!(ch("07"), BmsChannel::BgaLayer);
}

#[test]
fn extended_bpm() {
    assert_eq!(ch("08"), BmsChannel::ExtendedBpm);
}

#[test]
fn stop() {
    assert_eq!(ch("09"), BmsChannel::Stop);
}

#[test]
fn bga_layer2() {
    assert_eq!(ch("0A"), BmsChannel::BgaLayer2);
    assert_eq!(ch("0a"), BmsChannel::BgaLayer2);
    assert_eq!(ch("A"), BmsChannel::BgaLayer2);
    assert_eq!(ch("a"), BmsChannel::BgaLayer2);
}

#[test]
fn bga_base_opacity() {
    assert_eq!(ch("0B"), BmsChannel::BgaBaseOpacity);
    assert_eq!(ch("B"), BmsChannel::BgaBaseOpacity);
}

#[test]
fn bga_layer_opacity() {
    assert_eq!(ch("0C"), BmsChannel::BgaLayerOpacity);
}

#[test]
fn bga_layer2_opacity() {
    assert_eq!(ch("0D"), BmsChannel::BgaLayer2Opacity);
    assert_eq!(ch("d"), BmsChannel::BgaLayer2Opacity);
}

#[test]
fn bga_poor_opacity() {
    assert_eq!(ch("0E"), BmsChannel::BgaPoorOpacity);
}

#[test]
fn bgm_volume() {
    assert_eq!(ch("97"), BmsChannel::BgmVolume);
}

#[test]
fn key_volume() {
    assert_eq!(ch("98"), BmsChannel::KeyVolume);
}

#[test]
fn text() {
    assert_eq!(ch("99"), BmsChannel::Text);
}

#[test]
fn judge() {
    assert_eq!(ch("A0"), BmsChannel::Judge);
    assert_eq!(ch("a0"), BmsChannel::Judge);
}

#[test]
fn bga_argb_base() {
    assert_eq!(ch("A1"), BmsChannel::BgaArgbBase);
}

#[test]
fn bga_argb_layer() {
    assert_eq!(ch("A2"), BmsChannel::BgaArgbLayer);
}

#[test]
fn bga_argb_layer2() {
    assert_eq!(ch("A3"), BmsChannel::BgaArgbLayer2);
}

#[test]
fn bga_argb_poor() {
    assert_eq!(ch("A4"), BmsChannel::BgaArgbPoor);
}

#[test]
fn bga_keybound() {
    assert_eq!(ch("A5"), BmsChannel::BgaKeyBound);
}

#[test]
fn option() {
    assert_eq!(ch("A6"), BmsChannel::Option);
    assert_eq!(ch("a6"), BmsChannel::Option);
}

// 扩展通道

#[test]
fn scroll() {
    assert_eq!(ch("SC"), BmsChannel::Scroll);
    assert_eq!(ch("sc"), BmsChannel::Scroll);
    assert_eq!(ch("Sc"), BmsChannel::Scroll);
}

#[test]
fn speed() {
    assert_eq!(ch("SP"), BmsChannel::Speed);
    assert_eq!(ch("sp"), BmsChannel::Speed);
}

// 音符通道

#[test]
fn note_1p_visible() {
    let expected = BmsChannel::Note(idx("11"));
    assert_eq!(ch("11"), expected);
}

#[test]
fn note_1p_visible_key9() {
    let expected = BmsChannel::Note(idx("19"));
    assert_eq!(ch("19"), expected);
}

#[test]
fn note_2p_visible() {
    let expected = BmsChannel::Note(idx("21"));
    assert_eq!(ch("21"), expected);
}

#[test]
fn note_1p_invisible() {
    let expected = BmsChannel::Note(idx("31"));
    assert_eq!(ch("31"), expected);
}

#[test]
fn note_2p_invisible() {
    let expected = BmsChannel::Note(idx("41"));
    assert_eq!(ch("41"), expected);
}

#[test]
fn note_1p_long() {
    let expected = BmsChannel::Note(idx("51"));
    assert_eq!(ch("51"), expected);
}

#[test]
fn note_2p_long() {
    let expected = BmsChannel::Note(idx("61"));
    assert_eq!(ch("61"), expected);
}

#[test]
fn note_1p_landmine() {
    let expected = BmsChannel::Note(idx("D1"));
    assert_eq!(ch("D1"), expected);
    // 小写输入在内部被规范化为大写。
    assert_eq!(ch("d1"), expected);
}

#[test]
fn note_2p_landmine() {
    let expected = BmsChannel::Note(idx("E9"));
    assert_eq!(ch("E9"), expected);
    assert_eq!(ch("e9"), expected);
}

#[test]
fn note_mgq_ext() {
    // MGQ 将音符通道扩展到 1A–1F、2A–2F 等。
    let expected = BmsChannel::Note(idx("1A"));
    assert_eq!(ch("1A"), expected);
    assert_eq!(ch("1a"), expected);
}

#[test]
fn note_pomu_ext() {
    // pomu 将音符通道扩展到 1G–1Z、2G–2Z 等。
    let expected = BmsChannel::Note(idx("1G"));
    assert_eq!(ch("1G"), expected);
}

#[test]
fn note_5f() {
    // 5F 是一个长音 MGQ 扩展。
    let expected = BmsChannel::Note(idx("5F"));
    assert_eq!(ch("5F"), expected);
}

// 保留通道（非音符）

#[test]
fn track_10_is_unknown_not_note() {
    assert_eq!(ch("10"), BmsChannel::Unknown(idx("10")));
}

#[test]
fn track_20_is_unknown_not_note() {
    assert_eq!(ch("20"), BmsChannel::Unknown(idx("20")));
}

#[test]
fn track_30_is_unknown_not_note() {
    assert_eq!(ch("30"), BmsChannel::Unknown(idx("30")));
}

#[test]
fn track_40_is_unknown_not_note() {
    assert_eq!(ch("40"), BmsChannel::Unknown(idx("40")));
}

#[test]
fn track_50_is_unknown_not_note() {
    assert_eq!(ch("50"), BmsChannel::Unknown(idx("50")));
}

#[test]
fn track_60_is_unknown_not_note() {
    assert_eq!(ch("60"), BmsChannel::Unknown(idx("60")));
}

#[test]
fn d0_is_unknown_not_note() {
    assert_eq!(ch("D0"), BmsChannel::Unknown(idx("D0")));
}

#[test]
fn e0_is_unknown_not_note() {
    assert_eq!(ch("E0"), BmsChannel::Unknown(idx("E0")));
}

// 未知通道

#[test]
fn channel_00_is_unknown() {
    assert_eq!(ch("00"), BmsChannel::Unknown(idx("00")));
}

#[test]
fn channel_0f_is_unknown() {
    let expected = BmsChannel::Unknown(idx("0F"));
    assert_eq!(ch("0F"), expected);
    assert_eq!(ch("0f"), expected);
}

#[test]
fn channel_zz_is_unknown() {
    let expected = BmsChannel::Unknown(idx("ZZ"));
    assert_eq!(ch("ZZ"), expected);
    assert_eq!(ch("zz"), expected);
}

#[test]
fn channel_70_is_unknown() {
    assert_eq!(ch("70"), BmsChannel::Unknown(idx("70")));
}

#[test]
fn channel_96_is_unknown() {
    assert_eq!(ch("96"), BmsChannel::Unknown(idx("96")));
}

#[test]
fn single_char_f_is_unknown() {
    let expected = BmsChannel::Unknown(idx("F"));
    assert_eq!(ch("F"), expected);
    assert_eq!(ch("f"), expected);
}

#[test]
fn single_char_0_is_unknown() {
    assert_eq!(ch("0"), BmsChannel::Unknown(idx("0")));
}

// from_raw 边界情况

#[test]
fn from_raw_invalid_string_returns_none() {
    assert_eq!(BmsChannel::from_raw(""), None);
    assert_eq!(BmsChannel::from_raw("!!"), None);
    assert_eq!(BmsChannel::from_raw("ABC"), None);
}

// as_u8_hex

#[test]
fn as_u8_hex_known_channels() {
    assert_eq!(BmsChannel::Bgm.as_u8_hex(), Some(0x01));
    assert_eq!(BmsChannel::MeasureLength.as_u8_hex(), Some(0x02));
    assert_eq!(BmsChannel::BpmChange.as_u8_hex(), Some(0x03));
    assert_eq!(BmsChannel::BgaBase.as_u8_hex(), Some(0x04));
    assert_eq!(BmsChannel::Seek.as_u8_hex(), Some(0x05));
    assert_eq!(BmsChannel::BgaPoor.as_u8_hex(), Some(0x06));
    assert_eq!(BmsChannel::BgaLayer.as_u8_hex(), Some(0x07));
    assert_eq!(BmsChannel::ExtendedBpm.as_u8_hex(), Some(0x08));
    assert_eq!(BmsChannel::Stop.as_u8_hex(), Some(0x09));
    assert_eq!(BmsChannel::BgaLayer2.as_u8_hex(), Some(0x0A));
    assert_eq!(BmsChannel::BgaBaseOpacity.as_u8_hex(), Some(0x0B));
    assert_eq!(BmsChannel::BgaLayerOpacity.as_u8_hex(), Some(0x0C));
    assert_eq!(BmsChannel::BgaLayer2Opacity.as_u8_hex(), Some(0x0D));
    assert_eq!(BmsChannel::BgaPoorOpacity.as_u8_hex(), Some(0x0E));
    assert_eq!(BmsChannel::BgmVolume.as_u8_hex(), Some(0x97));
    assert_eq!(BmsChannel::KeyVolume.as_u8_hex(), Some(0x98));
    assert_eq!(BmsChannel::Text.as_u8_hex(), Some(0x99));
    assert_eq!(BmsChannel::Judge.as_u8_hex(), Some(0xA0));
    assert_eq!(BmsChannel::BgaArgbBase.as_u8_hex(), Some(0xA1));
    assert_eq!(BmsChannel::BgaArgbLayer.as_u8_hex(), Some(0xA2));
    assert_eq!(BmsChannel::BgaArgbLayer2.as_u8_hex(), Some(0xA3));
    assert_eq!(BmsChannel::BgaArgbPoor.as_u8_hex(), Some(0xA4));
    assert_eq!(BmsChannel::BgaKeyBound.as_u8_hex(), Some(0xA5));
    assert_eq!(BmsChannel::Option.as_u8_hex(), Some(0xA6));
}

#[test]
fn as_u8_hex_scroll_speed_none() {
    assert_eq!(BmsChannel::Scroll.as_u8_hex(), None);
    assert_eq!(BmsChannel::Speed.as_u8_hex(), None);
}

#[test]
fn as_u8_hex_note_hex_channel() {
    let n = BmsChannel::Note(idx("11"));
    assert_eq!(n.as_u8_hex(), Some(0x11));
}

#[test]
fn as_u8_hex_note_non_hex_channel() {
    // 1G 是有效的 Base36 但非十六进制 → None
    let n = BmsChannel::Note(idx("1G"));
    assert_eq!(n.as_u8_hex(), None);
}

#[test]
fn as_u8_hex_unknown_hex() {
    let u = BmsChannel::Unknown(idx("FF"));
    assert_eq!(u.as_u8_hex(), Some(0xFF));
}

// Display

#[test]
fn display_known_channels() {
    assert_eq!(BmsChannel::Bgm.to_string(), "01");
    assert_eq!(BmsChannel::BgaLayer2.to_string(), "0A");
    assert_eq!(BmsChannel::Judge.to_string(), "A0");
    assert_eq!(BmsChannel::Scroll.to_string(), "SC");
    assert_eq!(BmsChannel::Speed.to_string(), "SP");
}

#[test]
fn display_note() {
    let n = BmsChannel::Note(idx("11"));
    assert_eq!(n.to_string(), "11");
}

#[test]
fn display_unknown() {
    let u = BmsChannel::Unknown(idx("ZZ"));
    assert_eq!(u.to_string(), "ZZ");
    // 小写输入在规范化后以大写存储。
    let u2 = ch("zz");
    assert_eq!(u2.to_string(), "ZZ");
}

// Ord / Eq（BTreeMap 键所需）

#[test]
fn note_channels_ordered_by_raw_index() {
    let n1 = BmsChannel::Note(idx("11"));
    let n2 = BmsChannel::Note(idx("12"));
    assert!(n1 < n2);
    assert_ne!(n1, n2);
}

#[test]
fn unit_variants_ordered_by_declaration() {
    assert!(BmsChannel::Bgm < BmsChannel::MeasureLength);
    assert!(BmsChannel::Stop < BmsChannel::BgaLayer2);
    assert!(BmsChannel::BgaLayer2 < BmsChannel::BgaBaseOpacity);
}

#[test]
fn note_greater_than_unit_variants() {
    // 单元变体在声明顺序上先于元组变体。
    assert!(BmsChannel::Option < BmsChannel::Note(idx("11")));
    assert!(BmsChannel::Speed < BmsChannel::Note(idx("11")));
}

#[test]
fn unknown_greater_than_note() {
    assert!(BmsChannel::Note(idx("11")) < BmsChannel::Unknown(idx("00")));
}
