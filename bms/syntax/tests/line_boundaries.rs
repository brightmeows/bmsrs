//! Line-boundary consistency between `bmsrs-bms-source` and
//! `bmsrs-bms-syntax`.
//!
//! `SourceFile::parse_document` calls `Document::to_text()` and then splits
//! lines again. The two layers must agree on the line count and each line's
//! text; otherwise the `TextRange` values in `Analysis` will be misaligned with
//! document coordinates.

use bmsrs_bms_source::Document;
use bmsrs_bms_syntax::SourceFile;

/// Assert that document lines and CST lines match text for text.
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
        "document and CST line boundaries disagree"
    );
}

/// Assert line-boundary consistency for UTF-8 text.
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
