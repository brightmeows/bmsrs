//! Public API tests for the pipeline result and diagnostic types.

use bmsrs_pipeline::{Diagnostic, DiagnosticCode, Outcome, Severity, SourceSpan};

#[test]
fn outcome_into_parts_keeps_value_and_warnings() {
    let outcome = Outcome::new("chart", vec!["warning"]);
    let (value, warnings) = outcome.into_parts();
    assert_eq!(value, "chart");
    assert_eq!(warnings, vec!["warning"]);
}

#[test]
fn outcome_maps_value_and_warnings_independently() {
    let outcome = Outcome::new(2u32, vec![1u32, 2u32]);
    let mapped = outcome.map(|value| value * 10);
    assert_eq!(mapped.value, 20);
    assert_eq!(mapped.warnings, vec![1, 2]);

    let remapped = mapped.map_warnings(|warning| warning * 100);
    assert_eq!(remapped.value, 20);
    assert_eq!(remapped.warnings, vec![100, 200]);
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
}

#[test]
fn diagnostic_keeps_fields() {
    let diagnostic = Diagnostic::new(
        Severity::Warning,
        Some(SourceSpan::new(1, 3)),
        "message",
        Some(DiagnosticCode::DuplicateDefinition),
    );
    assert!(!diagnostic.is_error());
    assert_eq!(diagnostic.span, Some(SourceSpan::new(1, 3)));
    assert_eq!(diagnostic.message, "message");
    assert_eq!(diagnostic.code, Some(DiagnosticCode::DuplicateDefinition));
}
