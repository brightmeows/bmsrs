//! `bmsrs-bms-source` 与 `bmsrs-bms-syntax` 的行边界一致性。
//!
//! `SourceFile::parse_document` 会先调用 `Document::to_text()` 再重新
//! 切行。两层必须对行数与每行文本达成一致，否则 `Analysis` 的
//! `TextRange` 会与文档坐标错位。

use bmsrs_bms_source::Document;
use bmsrs_bms_syntax::SourceFile;

/// 断言文档行与 CST 行的文本逐一相等。
fn assert_document_boundaries(document: &Document) {
    let parsed = SourceFile::parse_document(document);
    let document_lines: Vec<String> = (0..document.len())
        .filter_map(|index| document.line_text(index))
        .collect();
    let syntax_lines: Vec<String> = parsed
        .source_file()
        .lines()
        .map(|line| {
            line.syntax()
                .text()
                .to_string()
                .trim_end_matches(['\r', '\n'])
                .to_owned()
        })
        .collect();
    assert_eq!(
        document_lines, syntax_lines,
        "document 与 CST 的行边界不一致"
    );
}

/// 断言 UTF-8 文本的行边界一致。
fn assert_text_boundaries(text: &str) {
    assert_document_boundaries(&Document::from_text(text));
}

#[test]
fn lf_boundaries_match() {
    assert_text_boundaries("#TITLE A\n#ARTIST B\n");
}

#[test]
fn crlf_boundaries_match() {
    assert_text_boundaries("#TITLE A\r\n#ARTIST B\r\n");
}

#[test]
fn cr_boundaries_match() {
    assert_text_boundaries("#TITLE A\r#ARTIST B\r");
}

#[test]
fn missing_trailing_newline_matches() {
    assert_text_boundaries("#TITLE A\n#ARTIST B");
}

#[test]
fn blank_and_comment_lines_match() {
    assert_text_boundaries("// comment\n\n#TITLE A\n");
}

#[test]
fn utf16le_boundaries_match() {
    let text = "#TITLE A\r\n#ARTIST B\r\n";
    let mut bytes = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    assert_document_boundaries(&Document::from_bytes(bytes));
}
