//! `bms-tokenizer` 编码检测/转换的集成测试。

use bms_tokenizer::BmsEncoding;
use bms_tokenizer::{BmsHeader, BmsHeaderMetadata, BmsToken, BmsTokenizer};

// BmsEncoding::detect 集成

#[test]
fn detect_utf8_bom() {
    let bytes = &[0xEF, 0xBB, 0xBF, b'#', b'T', b'I', b'T', b'L', b'E'];
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::Utf8);
}

#[test]
fn detect_utf16le_bom() {
    let bytes = &[0xFF, 0xFE, 0x23, 0x00, 0x54, 0x00];
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::Utf16Le);
}

#[test]
fn detect_utf16be_bom() {
    let bytes = &[0xFE, 0xFF, 0x00, 0x23, 0x00, 0x54];
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::Utf16Be);
}

#[test]
fn detect_charset_shift_jis() {
    let bytes = b"#CHARSET Shift_JIS\n#TITLE test";
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::ShiftJis);
}

#[test]
fn detect_charset_euc_kr() {
    let bytes = b"#CHARSET EUC-KR\n#TITLE test";
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::EucKr);
}

#[test]
fn detect_valid_utf8_without_bom() {
    let bytes = b"#TITLE Hello\n#BPM 180";
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::Utf8);
}

#[test]
fn detect_shift_jis_bytes() {
    // "あ" (U+3042) in Shift_JIS = 0x82 0xA0
    let bytes = &[0x82, 0xA0];
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::ShiftJis);
}

#[test]
fn detect_euc_kr_bytes() {
    // 使用 #CHARSET 确保可靠检测（纯 EUC-KR 字节可能同时也是有效 Shift_JIS）
    let bytes = b"#CHARSET EUC-KR\n\xb0\xa1";
    assert_eq!(BmsEncoding::detect(bytes), BmsEncoding::EucKr);
}

// BmsEncoding::decode 集成

#[test]
fn decode_utf8_strips_bom() {
    let bytes = &[0xEF, 0xBB, 0xBF, b'#'];
    assert_eq!(BmsEncoding::Utf8.decode(bytes), "#");
}

#[test]
fn decode_utf16le_strips_bom() {
    let bytes = &[0xFF, 0xFE, 0x23, 0x00];
    assert_eq!(BmsEncoding::Utf16Le.decode(bytes), "#");
}

#[test]
fn decode_utf16be_strips_bom() {
    let bytes = &[0xFE, 0xFF, 0x00, 0x23];
    assert_eq!(BmsEncoding::Utf16Be.decode(bytes), "#");
}

// BmsTokenizer::tokenize_bytes 集成

#[test]
fn tokenize_bytes_utf8_works() {
    let bytes = b"#TITLE Hello\n#BPM 180";
    let tokens = BmsTokenizer::new().tokenize_bytes(bytes);
    assert_eq!(tokens.len(), 2);
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Title(_)
        )))
    ));
}

#[test]
fn tokenize_bytes_with_charset_header() {
    let bytes = b"#CHARSET Shift_JIS\n#TITLE Hello";
    let tokens = BmsTokenizer::new().tokenize_bytes(bytes);
    assert_eq!(tokens.len(), 2);
    assert!(matches!(
        tokens[1].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Title(_)
        )))
    ));
}

#[test]
fn tokenize_bytes_with_explicit_encoding() {
    let bytes = b"#TITLE Hello";
    let tokens = BmsTokenizer::new()
        .encoding(BmsEncoding::Utf8)
        .tokenize_bytes(bytes);
    assert_eq!(tokens.len(), 1);
    let expected = "Hello".to_owned();
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Title(ref t)
        ))) if *t == expected
    ));
}

#[test]
fn tokenize_bytes_shift_jis_content() {
    // "#TITLE " followed by "テスト" in Shift_JIS
    // テ = 0x83 0x65, ス = 0x83 0x58, ト = 0x83 0x67
    let bytes = b"#TITLE \x83\x65\x83\x58\x83\x67";
    let tokens = BmsTokenizer::new().tokenize_bytes(bytes);
    assert_eq!(tokens.len(), 1);
    if let Ok(BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title(title)))) = &tokens[0].1
    {
        assert_eq!(title, "\u{30c6}\u{30b9}\u{30c8}");
    } else {
        panic!("expected Title header");
    }
}

#[test]
fn tokenize_bytes_utf8_with_bom() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"#TITLE Hello");
    let tokens = BmsTokenizer::new().tokenize_bytes(&bytes);
    assert_eq!(tokens.len(), 1);
    let expected = "Hello".to_owned();
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Title(ref t)
        ))) if *t == expected
    ));
}

#[test]
fn tokenize_bytes_owned_via_encoding_roundtrip() {
    let tokenizer = BmsTokenizer::new().encoding(BmsEncoding::Utf8);
    let bytes = b"#TITLE Test\n#BPM 120";
    let tokens = tokenizer.tokenize_bytes(bytes);
    assert_eq!(tokens.len(), 2);
    assert!(tokens[0].1.is_ok());
    assert!(tokens[1].1.is_ok());
}
