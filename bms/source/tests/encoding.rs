//! 编码检测与支持范围测试。

use bms_source::{DetectionMethod, Document, SourceEncoding};

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
