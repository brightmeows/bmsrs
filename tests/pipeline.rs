//! Integration tests for the `bmsrs` whole-pipeline entry points.
//!
//! The `bms::parse` family assembles CST parsing, control-flow selection, and
//! single-line parsing; these tests verify that both error strategies behave
//! as external consumers expect.

use bmsrs::bms::ErrorStrategy;
use bmsrs::bms::{Input, SourceFormat};
use bmsrs::raw::bms::syntax::ParseOptions;

#[test]
fn parse_with_collect_all_keeps_tokens_after_error() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let outcome = bmsrs::bms::parse_with(
        Input::text(text, SourceFormat::Bms),
        &ParseOptions::default(),
        &mut |_| 1,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(outcome.value.metadata.artist.as_deref(), Some("B"));
}

#[test]
fn parse_with_collect_all_reports_tokenize_warnings() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let outcome = bmsrs::bms::parse_with(
        Input::text(text, SourceFormat::Bms),
        &ParseOptions::default(),
        &mut |_| 1,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(
        outcome.warnings.as_slice(),
        [bmsrs::bms::BmsDiagnostic {
            kind: bmsrs::bms::BmsDiagnosticKind::Tokenize(_),
            range,
            ..
        }] if !range.is_empty()
    ));
}

#[test]
fn colon_separated_definitions_flow_through_pipeline() {
    let text = "#BPM 120\n#STOP01:48\n#BPM0A:240\n#00109:01\n#00108:0A\n";
    let parsed = bmsrs::bms::parse(Input::text(text, SourceFormat::Bms))
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        parsed.warnings.is_empty(),
        "colon definitions must not produce tokenize warnings: {:?}",
        parsed.warnings
    );
    assert_eq!(
        parsed.value.timing.stop_defs.get(&"01".parse().unwrap()),
        Some(&48.0)
    );
    assert_eq!(
        parsed.value.timing.bpm_defs.get(&"0A".parse().unwrap()),
        Some(&240.0)
    );

    let processed = bmsrs::bms::process(&parsed.value).unwrap_or_else(|error| panic!("{error}"));
    let events = processed.value.chart.data().events();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event.kind, bmsrs::chart::EventKind::Stop { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event.kind, bmsrs::chart::EventKind::Bpm { .. }))
            .count(),
        1
    );
}

#[test]
fn diagnostic_converts_tokenize_warning() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let outcome = bmsrs::bms::parse_with(
        Input::text(text, SourceFormat::Bms),
        &ParseOptions::default(),
        &mut |_| 1,
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
    let options = ParseOptions {
        error_strategy: ErrorStrategy::FailFast,
        ..ParseOptions::default()
    };
    let result = bmsrs::bms::parse_with(Input::text(text, SourceFormat::Bms), &options, &mut |_| 1);
    assert!(matches!(
        result,
        Err(bmsrs::bms::ParseError::Diagnostic(bmsrs::bms::BmsDiagnostic {
            kind: bmsrs::bms::BmsDiagnosticKind::Tokenize(_),
            range,
            ..
        })) if !range.is_empty()
    ));
}

#[test]
fn parse_with_collect_all_reports_unselected_branch_error() {
    let text = "#RANDOM 2\n#IF 1\n#BPM 120\n#ENDIF\n#IF 2\n#BPM abc\n#ENDIF\n#ENDRANDOM\n";
    let outcome = bmsrs::bms::parse_with(
        Input::text(text, SourceFormat::Bms),
        &ParseOptions::default(),
        &mut |_| 1,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(
        outcome.warnings.as_slice(),
        [bmsrs::bms::BmsDiagnostic {
            kind: bmsrs::bms::BmsDiagnosticKind::Tokenize(_),
            ..
        }]
    ));
}

#[test]
fn parse_with_accepts_custom_header_prefix() {
    let options = ParseOptions {
        header_prefixes: vec!['@'],
        ..ParseOptions::default()
    };
    let outcome = bmsrs::bms::parse_with(
        Input::text("@TITLE A\n", SourceFormat::Bms),
        &options,
        &mut |_| 1,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(outcome.value.metadata.title.as_deref(), Some("A"));
}

#[test]
fn bytes_input_detects_shift_jis() {
    let text = "#TITLE テスト\n#BPM 120\n";
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(text);

    let bms = bmsrs::bms::parse(Input::bytes(bytes.into_owned(), SourceFormat::Bms))
        .unwrap()
        .value;

    assert_eq!(bms.metadata.title.as_deref(), Some("テスト"));
}
