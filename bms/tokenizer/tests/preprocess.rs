//! Integration tests for [`bms_tokenizer::preprocess`].
//!
//! All tests exercise only the public API `bms_tokenizer::preprocess()`.

use bms_tokenizer::preprocess;

#[test]
fn inline_double_slash_stripped() {
    assert_eq!(preprocess("#TITLE foo // bar"), "#TITLE foo ");
}

#[test]
fn double_slash_at_line_start_removes_line() {
    assert_eq!(
        preprocess("// this is a comment\n#TITLE real"),
        "\n#TITLE real"
    );
}

#[test]
fn double_slash_does_not_affect_other_lines() {
    assert_eq!(
        preprocess("#TITLE foo\n#ARTIST bar\n// comment\n#BPM 120"),
        "#TITLE foo\n#ARTIST bar\n\n#BPM 120"
    );
}

#[test]
fn semicolon_at_line_start_removes_line() {
    assert_eq!(preprocess("; debug\n#TITLE x"), "\n#TITLE x");
}

#[test]
fn semicolon_after_whitespace_is_line_start() {
    assert_eq!(preprocess("  ; debug\n#TITLE x"), "  \n#TITLE x");
}

#[test]
fn semicolon_in_mid_line_is_not_stripped() {
    assert_eq!(preprocess("#TITLE foo;bar"), "#TITLE foo;bar");
}

#[test]
fn url_without_quotes_gets_stripped_at_double_slash() {
    assert_eq!(preprocess("#URL http://example.com"), "#URL http:");
}

#[test]
fn block_comment_single_line() {
    assert_eq!(preprocess("/* comment */#TITLE x"), "#TITLE x");
}

#[test]
fn block_comment_multi_line() {
    assert_eq!(preprocess("/* line1\n   line2 */#TITLE x"), "#TITLE x");
}

#[test]
fn block_comment_non_nesting() {
    assert_eq!(preprocess("/* outer /* inner */#TITLE x"), "#TITLE x");
}

#[test]
fn unclosed_block_comment_strips_remainder() {
    assert_eq!(preprocess("#TITLE x\n/* no end"), "#TITLE x\n");
}

#[test]
fn string_quotes_protect_double_slash() {
    assert_eq!(
        preprocess("#URL \"http://example.com\""),
        "#URL \"http://example.com\""
    );
}

#[test]
fn string_quotes_protect_semicolon() {
    assert_eq!(preprocess("#TITLE \"foo;bar\""), "#TITLE \"foo;bar\"");
}

#[test]
fn string_quotes_protect_block_comment() {
    assert_eq!(
        preprocess("#TITLE \"/* not a comment */\""),
        "#TITLE \"/* not a comment */\""
    );
}

#[test]
fn comment_outside_string_is_stripped() {
    assert_eq!(
        preprocess("#URL \"http://safe.com\" // real comment"),
        "#URL \"http://safe.com\" "
    );
}

#[test]
fn backslash_escape_in_string() {
    assert_eq!(preprocess("#TITLE \"foo\\\"bar\""), "#TITLE \"foo\\\"bar\"");
}

#[test]
fn mixed_comments() {
    let input = "#TITLE Song\n// meta\n; debug\n/* block */\n#BPM 120 // tempo";
    let expected = "#TITLE Song\n\n\n\n#BPM 120 ";
    assert_eq!(preprocess(input), expected);
}

#[test]
fn empty_input() {
    assert_eq!(preprocess(""), "");
}

#[test]
fn no_comments_passthrough() {
    let input = "#TITLE Song\n#BPM 120\n#00111:1122";
    assert_eq!(preprocess(input), input);
}
