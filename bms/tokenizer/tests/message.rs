//! [`parse_message_line`] 与 [`BmsMessage`] 解析的集成测试。

use bms_tokenizer::{BmsChannel, BmsMessage, BmsTokenizeError, ChannelIndex, parse_message_line};

/// 避免测试调用中 turbofish 的辅助函数。
fn parse_msg(s: &str) -> Result<Option<BmsMessage>, BmsTokenizeError> {
    parse_message_line(s)
}

#[expect(clippy::unwrap_used, reason = "test helper panics on invalid input")]
fn idx(s: &str) -> ChannelIndex {
    let upper = s.to_ascii_uppercase();
    upper.as_str().try_into().unwrap()
}

// 基础解析

#[test]
fn parse_basic_message() {
    let result = parse_msg("#00111:11223344").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.addr, "00111");
    assert_eq!(msg.body, "11223344");
    assert_eq!(msg.track(), 1);
    assert_eq!(msg.channel(), BmsChannel::Note(idx("11")));
}

#[test]
fn parse_high_track() {
    let result = parse_msg("#99908:FF").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.track(), 999);
    assert_eq!(msg.channel(), BmsChannel::ExtendedBpm);
    assert_eq!(msg.body, "FF");
}

#[test]
fn parse_zero_track() {
    let result = parse_msg("#00051:A0B0").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.track(), 0);
    assert_eq!(msg.channel(), BmsChannel::Note(idx("51")));
}

#[test]
fn parse_channel_0a() {
    let result = parse_msg("#0010A:01").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.track(), 1);
    assert_eq!(msg.channel(), BmsChannel::BgaLayer2);
    assert_eq!(msg.channel().as_u8_hex(), Some(10));
}

#[test]
fn parse_channel_d1() {
    let result = parse_msg("#001D1:01").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.channel(), BmsChannel::Note(idx("D1")));
    assert_eq!(msg.channel().as_u8_hex(), Some(209));
}

#[test]
fn parse_channel_e9() {
    let result = parse_msg("#000E9:AA").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.channel(), BmsChannel::Note(idx("E9")));
}

#[test]
fn parse_channel_ff() {
    let result = parse_msg("#000FF:01").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.channel(), BmsChannel::Unknown(idx("FF")));
    assert_eq!(msg.channel().as_u8_hex(), Some(255));
}

#[test]
fn parse_extended_channel_sc() {
    let result = parse_msg("#000SC:1122").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.track(), 0);
    assert_eq!(msg.channel(), BmsChannel::Scroll);
    // SC 非十六进制 → as_u8_hex 返回 None
    assert_eq!(msg.channel().as_u8_hex(), None);
}

#[test]
fn parse_extended_channel_sp() {
    let result = parse_msg("#001SP:AA").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.track(), 1);
    assert_eq!(msg.channel(), BmsChannel::Speed);
    assert_eq!(msg.channel().as_u8_hex(), None);
}

// 正文与小节边界情况

#[test]
fn track_empty_prefix_is_zero() {
    let result = parse_msg("#01:1122").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.track(), 0); // 通道 "01" 前无 prefix
    assert_eq!(msg.channel(), BmsChannel::Bgm);
}

#[test]
fn track_single_char_addr() {
    let result = parse_msg("#1:1122").unwrap();
    let msg = result.expect("should parse");
    assert_eq!(msg.track(), 0); // prefix 为空
    assert_eq!(msg.channel(), BmsChannel::Bgm);
}

#[test]
fn track_uses_only_digits_from_prefix() {
    let result = parse_msg("#0A01:1122").unwrap();
    let msg = result.expect("should parse");
    // addr="0A01"，最后 2 位="01"=通道，prefix="0A" → 数字="0" → 0
    assert_eq!(msg.track(), 0);
    assert_eq!(msg.channel(), BmsChannel::Bgm);
}

// 返回 Ok(None) 的情况

#[test]
fn empty_line_returns_none() {
    assert_eq!(parse_msg("").unwrap(), None);
}

#[test]
fn no_hash_returns_none() {
    assert_eq!(parse_msg("hello world").unwrap(), None);
}

#[test]
fn header_line_returns_none() {
    assert_eq!(parse_msg("#TITLE test").unwrap(), None);
}

#[test]
fn comment_line_returns_none() {
    assert_eq!(parse_msg("// comment").unwrap(), None);
}

#[test]
fn too_short_line_returns_none() {
    assert_eq!(parse_msg("#a").unwrap(), None);
    assert_eq!(parse_msg("#:").unwrap(), None);
}

#[test]
fn addr_empty_returns_none() {
    assert_eq!(parse_msg("#:1122").unwrap(), None);
}

#[test]
fn no_colon_returns_none() {
    assert_eq!(parse_msg("#00101").unwrap(), None);
}

// 错误情况

#[test]
fn non_base62_last_char_returns_err() {
    // addr="001.."——最后一个字符 '.' 不是有效的 Base62
    let result = parse_msg("#001..:1122");
    assert!(result.is_err());
}

#[test]
fn invalid_channel_at_end_returns_err() {
    let result = parse_msg("#001!!:1122");
    assert!(result.is_err());
}
