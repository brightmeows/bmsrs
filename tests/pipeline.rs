//! Integration tests for the `bmsrs` whole-pipeline entry points.
//!
//! The `bms::parse` family assembles CST parsing, control-flow selection, and
//! single-line parsing; these tests verify that both error strategies behave
//! as external consumers expect.

use bmsrs::bms::{ErrorStrategy, syntax::ParseOptions};

#[test]
fn parse_with_collect_all_keeps_tokens_after_error() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let outcome = bmsrs::bms::parse_with(
        text,
        &ParseOptions::default(),
        |_| 1,
        ErrorStrategy::CollectAll,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(outcome.value.metadata.artist.as_deref(), Some("B"));
}

#[test]
fn parse_with_collect_all_reports_tokenize_warnings() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let outcome = bmsrs::bms::parse_with(
        text,
        &ParseOptions::default(),
        |_| 1,
        ErrorStrategy::CollectAll,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(
        outcome.warnings.as_slice(),
        [bmsrs::bms::FromTextWarning::Tokenize { range, .. }] if !range.is_empty()
    ));
}

#[test]
fn diagnostic_converts_tokenize_warning() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let outcome = bmsrs::bms::parse_with(
        text,
        &ParseOptions::default(),
        |_| 1,
        ErrorStrategy::CollectAll,
    )
    .unwrap_or_else(|error| panic!("{error}"));

    let warning = outcome
        .warnings
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("expected a tokenize warning"));
    let diagnostic: bmsrs::Diagnostic = warning.into();

    assert_eq!(
        diagnostic.code,
        Some(bmsrs::pipeline::DiagnosticCode::Tokenize)
    );
    assert!(diagnostic.span.is_some());
}

#[test]
fn parse_with_fail_fast_returns_tokenize_error() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let result = bmsrs::bms::parse_with(
        text,
        &ParseOptions::default(),
        |_| 1,
        ErrorStrategy::FailFast,
    );
    assert!(matches!(
        result,
        Err(bmsrs::bms::FromTextError::Tokenize { range, .. }) if !range.is_empty()
    ));
}

#[test]
fn parse_with_collect_all_reports_unselected_branch_error() {
    let text = "#RANDOM 2\n#IF 1\n#BPM 120\n#ENDIF\n#IF 2\n#BPM abc\n#ENDIF\n#ENDRANDOM\n";
    let outcome = bmsrs::bms::parse_with(
        text,
        &ParseOptions::default(),
        |_| 1,
        ErrorStrategy::CollectAll,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(
        outcome.warnings.as_slice(),
        [bmsrs::bms::FromTextWarning::Tokenize { .. }]
    ));
}

#[test]
fn parse_with_accepts_custom_header_prefix() {
    let options = ParseOptions {
        header_prefixes: vec!['@'],
    };
    let outcome = bmsrs::bms::parse_with("@TITLE A\n", &options, |_| 1, ErrorStrategy::CollectAll)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(outcome.value.metadata.title.as_deref(), Some("A"));
}

#[test]
fn parse_bytes_detects_shift_jis() {
    let text = "#TITLE テスト\n#BPM 120\n";
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(text);

    let bms = bmsrs::bms::parse_bytes(bytes.into_owned()).unwrap().value;

    assert_eq!(bms.metadata.title.as_deref(), Some("テスト"));
}
