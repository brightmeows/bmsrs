//! Source-layer line classification tests.

use bmsrs_bms_source::{Document, LineKind};

#[test]
fn blank_line_is_blank() {
    let document = Document::from_text("\n");
    assert!(matches!(
        document.line(0).map(bmsrs_bms_source::Line::kind),
        Some(LineKind::Blank)
    ));
}

#[test]
fn line_comment_is_comment() {
    let document = Document::from_text("// note\n");
    assert!(matches!(
        document.line(0).map(bmsrs_bms_source::Line::kind),
        Some(LineKind::Comment)
    ));
}

#[test]
fn semicolon_line_is_comment() {
    let document = Document::from_text("  ; debug\n");
    assert!(matches!(
        document.line(0).map(bmsrs_bms_source::Line::kind),
        Some(LineKind::Comment)
    ));
}

#[test]
fn multiline_block_comment_lines_are_trivia() {
    let document = Document::from_text("/* note\n   still note */\n#TITLE x\n");
    let is_trivia: Vec<_> = document
        .lines()
        .iter()
        .map(bmsrs_bms_source::Line::is_trivia)
        .collect();
    assert_eq!(is_trivia, vec![true, true, false]);
}

#[test]
fn header_line_is_text_at_source_layer() {
    let document = Document::from_text("#TITLE x\n");
    assert!(matches!(
        document.line(0).map(bmsrs_bms_source::Line::kind),
        Some(LineKind::Text)
    ));
}

#[test]
fn bare_text_is_text() {
    let document = Document::from_text("hello world\n");
    assert!(matches!(
        document.line(0).map(bmsrs_bms_source::Line::kind),
        Some(LineKind::Text)
    ));
}
