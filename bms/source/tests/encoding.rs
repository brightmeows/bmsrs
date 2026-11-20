//! Encoding detection and support-range tests.

use bmsrs_bms_source::{DetectionMethod, Document, SourceEncoding};

#[test]
fn bom_selects_utf8() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"#TITLE A\n");
    let document = Document::from_bytes(bytes);
    assert_eq!(document.detection().method, DetectionMethod::Bom);
}

#[test]
fn charset_declaration_selects_shift_jis() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("#CHARSET Shift_JIS\n#TITLE テスト\n");
    let document = Document::from_bytes(bytes.into_owned());
    assert_eq!(document.encoding().name(), "Shift_JIS");
}

#[test]
fn charset_declaration_selects_euc_kr() {
    let (bytes, _, _) = encoding_rs::EUC_KR.encode("#CHARSET EUC-KR\n#TITLE 테스트\n");
    let document = Document::from_bytes(bytes.into_owned());
    assert_eq!(document.encoding().name(), "EUC-KR");
}

#[test]
fn shift_jis_without_declaration_is_detected_heuristically() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("#TITLE テスト\n");
    let document = Document::from_bytes(bytes.into_owned());
    assert_eq!(document.detection().method, DetectionMethod::Heuristic);
}

#[test]
fn declared_utf8_with_shift_jis_bytes_falls_back() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("#CHARSET UTF-8\n#TITLE テスト\n");
    let document = Document::from_bytes(bytes.into_owned());
    assert_eq!(
        document.detection().method,
        DetectionMethod::DeclaredFallback
    );
}

#[test]
fn declared_utf8_with_shift_jis_bytes_reports_declaration() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("#CHARSET UTF-8\n#TITLE テスト\n");
    let document = Document::from_bytes(bytes.into_owned());
    assert_eq!(
        document.detection().declared.map(SourceEncoding::name),
        Some("UTF-8")
    );
}

#[test]
fn iso_2022_jp_is_rejected() {
    assert!(SourceEncoding::new(encoding_rs::ISO_2022_JP).is_err());
}

#[test]
fn replacement_encoding_is_rejected() {
    assert!(SourceEncoding::new(encoding_rs::REPLACEMENT).is_err());
}

#[test]
fn unknown_charset_label_falls_back_to_detection() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("#CHARSET X-UNKNOWN\n#TITLE テスト\n");
    let document = Document::from_bytes(bytes.into_owned());
    assert_eq!(document.encoding().name(), "Shift_JIS");
}

#[test]
fn utf32le_bom_is_detected_and_decoded() {
    let body = "#TITLE テスト\n#00111:01\n";
    let mut bytes = vec![0xFF, 0xFE, 0x00, 0x00];
    for character in body.chars() {
        bytes.extend_from_slice(&u32::from(character).to_le_bytes());
    }
    let document = Document::from_bytes(bytes.clone());
    assert_eq!(document.encoding().name(), "UTF-32LE");
    assert_eq!(document.detection().method, DetectionMethod::Bom);
    assert_eq!(document.to_text(), body);
}

#[test]
fn utf32be_bom_is_detected_and_decoded() {
    let body = "#TITLE テスト\n#00111:01\n";
    let mut bytes = vec![0x00, 0x00, 0xFE, 0xFF];
    for character in body.chars() {
        bytes.extend_from_slice(&u32::from(character).to_be_bytes());
    }
    let document = Document::from_bytes(bytes);
    assert_eq!(document.encoding().name(), "UTF-32BE");
    assert_eq!(document.detection().method, DetectionMethod::Bom);
    assert_eq!(document.to_text(), body);
}

#[test]
fn bomless_utf16_with_nul_signal_is_detected() {
    let body = "#TITLE test chart 001 #ARTIST alice\n#00111:01\n#00211:02\n";
    let mut bytes: Vec<u8> = Vec::new();
    for character in body.chars() {
        let unit = u16::try_from(u32::from(character)).unwrap_or(0xFFFD);
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let document = Document::from_bytes(bytes);
    assert_eq!(document.encoding().name(), "UTF-16LE");
    assert_eq!(document.to_text(), body);
}

#[test]
fn utf16_binary_noise_is_not_claimed() {
    // NUL-bearing bytes that do not decode as a BMS-shaped UTF-16 stream fall
    // through to the usual paths.
    let bytes: Vec<u8> = std::iter::repeat_n(0u8, 64).collect();
    let document = Document::from_bytes(bytes);
    assert_ne!(document.encoding().name(), "UTF-16LE");
    assert_ne!(document.encoding().name(), "UTF-16BE");
}

#[test]
fn gb18030_text_is_claimed_by_the_gbk_candidate() {
    // GB18030 (which `encoding_rs` names GBK) covers all of Unicode, so even
    // four-byte extension characters decode in the candidate pass.
    let text = "#TITLE \u{20000}\u{20001}\u{20002}\n#00111:01\n";
    let (bytes, _, had_errors) = encoding_rs::GB18030.encode(text);
    assert!(!had_errors);
    let document = Document::from_bytes(bytes.into_owned());
    let name = document.encoding().name();
    assert_eq!(name, "GBK", "method {:?}", document.detection().method);
    assert_eq!(document.to_text(), text);
}
