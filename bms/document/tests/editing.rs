//! 行级编辑测试。

use bms_document::{Document, DocumentOptions, EditError, LineKind, SourceEncoding};

/// 返回 `Shift_JIS` 编码。
fn shift_jis_encoding() -> SourceEncoding {
    let Some(encoding) = SourceEncoding::from_label(b"shift_jis") else {
        panic!("Shift_JIS 必须受支持");
    };
    encoding
}

/// 构造 `Shift_JIS` 文档。
fn shift_jis_document(source: &str) -> Document {
    let (bytes, _, had_unmappable) = encoding_rs::SHIFT_JIS.encode(source);
    assert!(!had_unmappable, "测试夹具必须可编码");
    Document::from_bytes_with(
        bytes.into_owned(),
        &DocumentOptions {
            encoding: Some(shift_jis_encoding()),
            header_prefixes: vec!['#', '%'],
        },
    )
}

#[test]
fn replace_line_preserves_other_lines() {
    let source = "#TITLE A\n#ARTIST B\n#00111:0102\n";
    let mut document = Document::from_text(source);
    document
        .replace_line(1, "#ARTIST C")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render(), b"#TITLE A\n#ARTIST C\n#00111:0102\n");
}

#[test]
fn shift_jis_edit_preserves_other_lines() {
    let source = "#CHARSET Shift_JIS\n#TITLE テスト\n// 注釈\n#00111:0102\n";
    let mut document = shift_jis_document(source);
    let before = document.render();
    document
        .replace_line(1, "#TITLE 変更")
        .unwrap_or_else(|error| panic!("{error}"));
    let after = document.render();
    let before_lines: Vec<&[u8]> = before.split(|byte| *byte == b'\n').collect();
    let after_lines: Vec<&[u8]> = after.split(|byte| *byte == b'\n').collect();
    assert_eq!(
        (before_lines[0], before_lines[2], before_lines[3]),
        (after_lines[0], after_lines[2], after_lines[3])
    );
}

#[test]
fn shift_jis_replace_content_keeps_comment() {
    let source = "  #TITLE テスト // 注釈
";
    let mut document = shift_jis_document(source);
    document
        .replace_content(0, "#TITLE 変更")
        .unwrap_or_else(|error| panic!("{error}"));
    let expected = "  #TITLE 変更 // 注釈\n";
    let (expected_bytes, _, _) = encoding_rs::SHIFT_JIS.encode(expected);
    assert_eq!(document.render(), expected_bytes.into_owned());
}

#[test]
fn replace_line_keeps_terminator() {
    let mut document = Document::from_text("#TITLE A\r\n#ARTIST B\r\n");
    document
        .replace_line(0, "#TITLE C")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render(), b"#TITLE C\r\n#ARTIST B\r\n");
}

#[test]
fn replace_content_preserves_indent_and_comment() {
    let mut document = Document::from_text("  #TITLE old // note\n");
    document
        .replace_content(0, "#TITLE new")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render(), b"  #TITLE new // note\n");
}

#[test]
fn replace_content_after_replace_line_replaces_whole_line() {
    let mut document = Document::from_text("  #TITLE old // note\n");
    document
        .replace_line(0, "#TITLE new")
        .unwrap_or_else(|error| panic!("{error}"));
    document
        .replace_content(0, "#TITLE newer")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render(), b"#TITLE newer\n");
}

#[test]
fn insert_line_inherits_crlf() {
    let mut document = Document::from_text("#TITLE A\r\n#ARTIST B\r\n");
    document
        .insert_line(1, "#BPM 150")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render(), b"#TITLE A\r\n#BPM 150\r\n#ARTIST B\r\n");
}

#[test]
fn insert_line_after_unterminated_line_adds_newline() {
    let mut document = Document::from_text("#TITLE A");
    document
        .insert_line(1, "#BPM 150")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render(), b"#TITLE A\n#BPM 150\n");
}

#[test]
fn remove_line_drops_only_that_line() {
    let mut document = Document::from_text("#TITLE A\n#ARTIST B\n#BPM 120\n");
    document
        .remove_line(1)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.render(), b"#TITLE A\n#BPM 120\n");
}

#[test]
fn out_of_bounds_replace_is_rejected() {
    let mut document = Document::from_text("#TITLE A\n");
    let result = document.replace_line(3, "#TITLE B");
    assert!(matches!(result, Err(EditError::OutOfBounds { .. })));
}

#[test]
fn out_of_bounds_insert_is_rejected() {
    let mut document = Document::from_text("#TITLE A\n");
    let result = document.insert_line(3, "#TITLE B");
    assert!(matches!(result, Err(EditError::OutOfBounds { .. })));
}

#[test]
fn unmappable_edit_is_rejected() {
    let mut document = shift_jis_document("#TITLE A\n");
    let result = document.replace_line(0, "#TITLE 简体中文");
    assert!(matches!(result, Err(EditError::Unmappable(_))));
}

#[test]
fn unmappable_edit_leaves_document_unchanged() {
    let mut document = shift_jis_document("#TITLE A\n");
    let before = document.render();
    let _result: Result<(), EditError> = document.replace_line(0, "#TITLE 简体中文");
    assert_eq!(document.render(), before);
}

#[test]
fn invalid_edit_is_accepted_as_error() {
    let mut document = Document::from_text("#BPM 120\n");
    document
        .replace_line(0, "#BPM abc")
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Error(_)))
    );
}

#[test]
fn invalid_edit_text_remains_readable() {
    let mut document = Document::from_text("#BPM 120\n");
    document
        .replace_line(0, "#BPM abc")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(document.line_text(0).as_deref(), Some("#BPM abc"));
}
