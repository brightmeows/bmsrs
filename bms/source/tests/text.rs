//! 完整文本与选中行渲染测试。

use bms_source::{Document, DocumentOptions, SourceEncoding, TextDecodeError};

/// 用指定编码编码文本。
fn encode(text: &str, encoding: &'static encoding_rs::Encoding) -> Vec<u8> {
    let (bytes, _, had_unmappable) = encoding.encode(text);
    assert!(!had_unmappable, "测试夹具必须可编码");
    bytes.into_owned()
}

#[test]
fn shift_jis_text_preserves_crlf() {
    let source = "#TITLE テスト\r\n#ARTIST ねこ\r\n";
    let document = Document::from_bytes(encode(source, encoding_rs::SHIFT_JIS));
    assert_eq!(document.to_text(), source);
}

#[test]
fn utf16_text_strips_bom() {
    let source = "#TITLE テスト\n";
    let mut bytes = vec![0xFF, 0xFE];
    for unit in source.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let document = Document::from_bytes(bytes);
    assert_eq!(document.to_text(), source);
}

#[test]
fn text_includes_edited_line() {
    let mut document = Document::from_text("#TITLE A\n#ARTIST B\n");
    document
        .replace_line(0, "#TITLE C")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.to_text(), "#TITLE C\n#ARTIST B\n");
}

#[test]
fn invalid_utf8_try_to_text_returns_error() {
    let Some(encoding) = SourceEncoding::from_label(b"utf-8") else {
        panic!("UTF-8 必须受支持");
    };
    let document = Document::from_bytes_with(
        vec![0xFF, 0xFE, 0x00],
        &DocumentOptions {
            encoding: Some(encoding),
        },
    );
    assert!(matches!(
        document.try_to_text(),
        Err(TextDecodeError { encoding: "UTF-8" })
    ));
}

#[test]
fn render_line_indices_keeps_bom_and_selected_bytes() {
    let mut source = vec![0xEF, 0xBB, 0xBF];
    source.extend_from_slice(b"#TITLE A\r\n#ARTIST B\r\n");
    let document = Document::from_bytes(source);
    let mut expected = vec![0xEF, 0xBB, 0xBF];
    expected.extend_from_slice(b"#TITLE A\r\n");
    assert_eq!(document.render_line_indices(&[0]), expected);
}

#[test]
fn render_line_indices_uses_replacement_bytes() {
    let mut document = Document::from_text("#TITLE A\n#ARTIST B\n");
    document
        .replace_line(1, "#ARTIST C")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render_line_indices(&[1]), b"#ARTIST C\n");
}
