//! Public API tests for the pipeline result and diagnostic types.

use bmsrs_pipeline::{Diagnostic, DiagnosticCode, HasSeverity as _, Outcome, Severity, SourceSpan};

#[test]
fn outcome_into_parts_keeps_value_and_diagnostics() {
    let outcome = Outcome::new("chart", vec!["diagnostic"]);
    let (value, diagnostics) = outcome.into_parts();
    assert_eq!(value, "chart");
    assert_eq!(diagnostics, vec!["diagnostic"]);
}

#[test]
fn outcome_maps_value_and_diagnostics_independently() {
    let outcome = Outcome::new(2u32, vec![1u32, 2u32]);
    let mapped = outcome.map(|value| value * 10);
    assert_eq!(mapped.value, 20);
    assert_eq!(mapped.diagnostics, vec![1, 2]);

    let remapped = mapped.map_diagnostics(|diagnostic| diagnostic * 100);
    assert_eq!(remapped.value, 20);
    assert_eq!(remapped.diagnostics, vec![100, 200]);
}

#[test]
fn outcome_has_errors_reads_diagnostic_severity() {
    let outcome = Outcome::new(
        "chart",
        vec![
            Diagnostic::new(Severity::Warning, "warning"),
            Diagnostic::new(Severity::Error, "error"),
            Diagnostic::new(Severity::Information, "information"),
        ],
    );
    assert!(outcome.has_errors());
    assert_eq!(outcome.errors().count(), 1);
    assert_eq!(
        outcome
            .errors()
            .next()
            .map(|diagnostic| diagnostic.message.as_str()),
        Some("error")
    );
}

#[test]
fn outcome_into_result_keeps_all_diagnostics_on_error() {
    let diagnostics = vec![
        Diagnostic::new(Severity::Warning, "warning"),
        Diagnostic::new(Severity::Error, "error"),
        Diagnostic::new(Severity::Information, "information"),
    ];
    let outcome = Outcome::new("chart", diagnostics);
    let returned = outcome.into_result().expect_err("error severity fails");
    assert_eq!(returned.len(), 3);
    assert_eq!(returned[0].message, "warning");
    assert_eq!(returned[1].message, "error");
    assert_eq!(returned[2].message, "information");
}

#[test]
fn outcome_into_result_returns_value_without_errors() {
    let outcome = Outcome::new("chart", vec![Diagnostic::new(Severity::Warning, "warning")]);
    assert_eq!(outcome.into_result(), Ok("chart"));
}

#[test]
fn pipeline_diagnostic_exposes_its_severity() {
    let diagnostic = Diagnostic::new(Severity::Warning, "message");
    assert_eq!(diagnostic.severity(), Severity::Warning);
}

#[test]
fn source_span_uses_half_open_ranges() {
    let span = SourceSpan::new(2, 5);
    assert_eq!(span.len(), 3);
    assert!(span.contains(2));
    assert!(span.contains(4));
    assert!(!span.contains(5));
    assert!(!span.is_empty());
    assert!(SourceSpan::empty(3).is_empty());
}

#[test]
fn diagnostic_code_strings_are_stable() {
    assert_eq!(DiagnosticCode::Syntax.as_str(), "syntax");
    assert_eq!(
        DiagnosticCode::DuplicateDefinition.as_str(),
        "duplicate-definition"
    );
    assert_eq!(
        DiagnosticCode::UndefinedReference.as_str(),
        "undefined-reference"
    );
    assert_eq!(DiagnosticCode::Conditional.as_str(), "conditional");
    assert_eq!(DiagnosticCode::Extension.as_str(), "extension");
}

#[test]
fn diagnostic_keeps_fields() {
    let diagnostic = Diagnostic::new(Severity::Warning, "message")
        .with_span(SourceSpan::new(1, 3))
        .with_code(DiagnosticCode::DuplicateDefinition);
    assert!(!diagnostic.is_error());
    assert_eq!(diagnostic.span, Some(SourceSpan::new(1, 3)));
    assert_eq!(diagnostic.message, "message");
    assert_eq!(diagnostic.code, Some(DiagnosticCode::DuplicateDefinition));
}
