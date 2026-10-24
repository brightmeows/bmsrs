//! 注释状态机与行分类测试。

use bms_document::{Document, LineKind};
use bms_tokenizer::{BmsHeader, BmsHeaderMetadata, BmsToken};

/// 提取文档中第一个 `#TITLE` 的值。
fn title(document: &Document) -> Option<String> {
    document.tokens().find_map(|(_, token)| match token {
        BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title(title))) => Some(title),
        _ => None,
    })
}

#[test]
fn inline_line_comment_is_not_part_of_value() {
    let document = Document::from_text("#TITLE Song // note\n");
    assert_eq!(title(&document), Some("Song".to_owned()));
}

#[test]
fn block_comment_before_header_is_skipped() {
    let document = Document::from_text("/* c */#TITLE x\n");
    assert_eq!(title(&document), Some("x".to_owned()));
}

#[test]
fn block_comment_after_header_is_not_part_of_value() {
    let document = Document::from_text("#TITLE x /* c */\n");
    assert_eq!(title(&document), Some("x".to_owned()));
}

#[test]
fn multiline_block_comment_lines_are_comments() {
    let document = Document::from_text("/* note\n   still note */\n#TITLE x\n");
    let kinds: Vec<_> = document
        .lines()
        .iter()
        .map(bms_document::Line::is_trivia)
        .collect();
    assert_eq!(kinds, vec![true, true, false]);
}

#[test]
fn multiline_block_comment_does_not_hide_following_header() {
    let document = Document::from_text("/* note\n   still note */#TITLE x\n");
    assert_eq!(title(&document), Some("x".to_owned()));
}

#[test]
fn semicolon_at_line_start_is_comment() {
    let document = Document::from_text("  ; debug\n#TITLE x\n");
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Comment))
    );
}

#[test]
fn semicolon_mid_line_is_part_of_value() {
    let document = Document::from_text("#TITLE foo;bar\n");
    assert_eq!(title(&document), Some("foo;bar".to_owned()));
}

#[test]
fn url_double_slash_is_not_comment() {
    let document = Document::from_text("#URL http://example.com\n");
    let token = document.tokens().next().map(|(_, token)| token);
    assert!(matches!(
        token,
        Some(BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Url(value))))
            if value == "http://example.com"
    ));
}

#[test]
fn quoted_double_slash_is_not_comment() {
    let document = Document::from_text("#URL \"http://example.com\"\n");
    let token = document.tokens().next().map(|(_, token)| token);
    assert!(matches!(
        token,
        Some(BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Url(value))))
            if value == "\"http://example.com\""
    ));
}

#[test]
fn escaped_quote_keeps_comment_marker_out_of_string() {
    let document = Document::from_text("#TITLE \"a\\\"b\" // tail\n");
    assert_eq!(title(&document), Some("\"a\\\"b\"".to_owned()));
}

#[test]
fn double_hash_line_is_comment() {
    let document = Document::from_text("## note\n#TITLE x\n");
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Comment))
    );
}

#[test]
fn bare_text_line_is_classified_as_text() {
    let document = Document::from_text("hello world\n");
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Text))
    );
}

#[test]
fn invalid_header_value_is_classified_as_error() {
    let document = Document::from_text("#BPM abc\n");
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Error(_)))
    );
}
