//! Byte-fidelity roundtrip tests.

use bmsrs_bms_source::Document;
use proptest::prelude::*;

/// Encode test text with the given encoding.
fn encode(text: &str, encoding: &'static encoding_rs::Encoding) -> Vec<u8> {
    let (bytes, _, had_unmappable) = encoding.encode(text);
    assert!(!had_unmappable, "test fixture must be encodable");
    bytes.into_owned()
}

/// Build UTF-16 bytes with an optional BOM.
fn utf16_bytes(text: &str, little_endian: bool, bom: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    if bom {
        bytes.extend_from_slice(if little_endian {
            &[0xFF, 0xFE]
        } else {
            &[0xFE, 0xFF]
        });
    }
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&if little_endian {
            unit.to_le_bytes()
        } else {
            unit.to_be_bytes()
        });
    }
    bytes
}

#[test]
fn utf8_lf_roundtrip_preserves_bytes() {
    let source = "#TITLE 曲名\n// 注释\n\n#00111:0102\n";
    let document = Document::from_text(source);
    assert_eq!(document.render(), source.as_bytes());
}

#[test]
fn utf8_crlf_roundtrip_preserves_bytes() {
    let source = "#TITLE A\r\n#BPM 120\r\n";
    let document = Document::from_text(source);
    assert_eq!(document.render(), source.as_bytes());
}

#[test]
fn utf8_cr_roundtrip_preserves_bytes() {
    let source = "#TITLE A\r#BPM 120\r";
    let document = Document::from_text(source);
    assert_eq!(document.render(), source.as_bytes());
}

#[test]
fn mixed_terminators_roundtrip_preserves_bytes() {
    let source = "#TITLE A\n#BPM 120\r\n#ARTIST B\r#GENRE C";
    let document = Document::from_text(source);
    assert_eq!(document.render(), source.as_bytes());
}

#[test]
fn shift_jis_roundtrip_preserves_bytes() {
    let source = "#CHARSET Shift_JIS\n#TITLE テスト\n#ARTIST ねこ\n";
    let bytes = encode(source, encoding_rs::SHIFT_JIS);
    let document = Document::from_bytes(bytes.clone());
    assert_eq!(document.render(), bytes);
}

#[test]
fn euc_kr_roundtrip_preserves_bytes() {
    let source = "#CHARSET EUC-KR\n#TITLE 테스트\n#ARTIST 고양이\n";
    let bytes = encode(source, encoding_rs::EUC_KR);
    let document = Document::from_bytes(bytes.clone());
    assert_eq!(document.render(), bytes);
}

#[test]
fn utf16le_bom_roundtrip_preserves_bytes() {
    let bytes = utf16_bytes("#TITLE テスト\r\n#BPM 120\r\n", true, true);
    let document = Document::from_bytes(bytes.clone());
    assert_eq!(document.render(), bytes);
}

#[test]
fn utf16be_bom_roundtrip_preserves_bytes() {
    let bytes = utf16_bytes("#TITLE テスト\n#BPM 120\n", false, true);
    let document = Document::from_bytes(bytes.clone());
    assert_eq!(document.render(), bytes);
}

#[test]
fn utf8_bom_roundtrip_preserves_bytes() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"#TITLE Bom\n");
    let document = Document::from_bytes(bytes.clone());
    assert_eq!(document.render(), bytes);
}

#[test]
fn empty_file_roundtrip_is_empty() {
    let document = Document::from_bytes(Vec::new());
    assert_eq!(document.render(), Vec::<u8>::new());
}

#[test]
fn missing_trailing_newline_is_preserved() {
    let source = "#TITLE A\n#ARTIST B";
    let document = Document::from_text(source);
    assert_eq!(document.render(), source.as_bytes());
}

proptest! {
    #[test]
    fn utf8_render_is_identity(text in "\\PC{0,200}") {
        let document = Document::from_text(&text);
        prop_assert_eq!(document.render(), text.as_bytes());
    }
}
