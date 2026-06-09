#![expect(missing_docs, reason = "integration tests")]

use bmson_de_chumsky::json::{classify_errors, parse_json};
use serde_json::Value;

// --- Basic JSON types ---

#[test]
fn null_value_parses_successfully() {
    let (val, errors) = parse_json("null");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Null));
}

#[test]
fn true_value_parses_successfully() {
    let (val, errors) = parse_json("true");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Bool(true)));
}

#[test]
fn false_value_parses_successfully() {
    let (val, errors) = parse_json("false");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Bool(false)));
}

#[test]
fn integer_parses_successfully() {
    let (val, errors) = parse_json("42");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Number(serde_json::Number::from(42))));
}

#[test]
fn negative_integer_parses_successfully() {
    let (val, errors) = parse_json("-17");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Number(serde_json::Number::from(-17))));
}

#[test]
fn float_parses_successfully() {
    let (val, errors) = parse_json("2.5");
    assert!(errors.is_empty());
    match val.as_ref().and_then(Value::as_f64) {
        Some(n) => assert!((n - 2.5).abs() < 1e-10),
        None => panic!("expected float value"),
    }
}

#[test]
fn string_parses_successfully() {
    let (val, errors) = parse_json(r#""hello world""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("hello world".to_string())));
}

#[test]
fn empty_array_parses_successfully() {
    let (val, errors) = parse_json("[]");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Array(vec![])));
}

#[test]
fn array_of_numbers_parses_successfully() {
    let (val, errors) = parse_json("[1, 2, 3]");
    assert!(errors.is_empty());
    assert_eq!(
        val,
        Some(Value::Array(vec![
            Value::Number(serde_json::Number::from(1)),
            Value::Number(serde_json::Number::from(2)),
            Value::Number(serde_json::Number::from(3)),
        ]))
    );
}

#[test]
fn empty_object_parses_successfully() {
    let (val, errors) = parse_json("{}");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Object(serde_json::Map::new())));
}

#[test]
fn simple_object_parses_successfully() {
    let (val, errors) = parse_json(r#"{"key": "value", "num": 1}"#);
    assert!(errors.is_empty());
    let Some(map) = val.as_ref().and_then(Value::as_object) else {
        panic!("expected object");
    };
    assert_eq!(map.get("key").and_then(|v| v.as_str()), Some("value"));
    assert_eq!(map.get("num").and_then(Value::as_u64), Some(1));
}

// --- String escape sequences ---

#[test]
fn string_escape_newline() {
    let (val, errors) = parse_json(r#""\n""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("\n".to_string())));
}

#[test]
fn string_escape_tab() {
    let (val, errors) = parse_json(r#""\t""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("\t".to_string())));
}

#[test]
fn string_escape_quote() {
    let (val, errors) = parse_json(r#""\"hello\"""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("\"hello\"".to_string())));
}

#[test]
fn string_escape_backslash() {
    let (val, errors) = parse_json(r#""a\\b""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("a\\b".to_string())));
}

#[test]
fn string_escape_unicode() {
    let (val, errors) = parse_json(r#""\u0041""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("A".to_string())));
}

#[test]
fn string_escape_solidus() {
    let (val, errors) = parse_json(r#""\/""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("/".to_string())));
}

#[test]
fn multiple_escape_sequences_in_one_string() {
    let (val, errors) = parse_json(r#""\n\t\r\\\"""#);
    assert!(errors.is_empty());
    let Some(s) = val.as_ref().and_then(Value::as_str) else {
        panic!("expected string");
    };
    assert_eq!(s, "\n\t\r\\\"");
}

#[test]
fn mixed_escape_and_literal_in_one_string() {
    let (val, errors) = parse_json(r#""hello\nworld\t!""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("hello\nworld\t!".to_string())));
}

// --- Error recovery ---

#[test]
fn trailing_comma_in_object_parses_without_errors() {
    let (val, errors) = parse_json(r#"{"a": 1, "b": 2,}"#);
    assert!(val.is_some());
    assert!(errors.is_empty(), "trailing commas are part of the grammar");
    let Some(obj) = val.as_ref().and_then(Value::as_object) else {
        panic!("expected object");
    };
    assert_eq!(obj.len(), 2);
}

#[test]
fn trailing_comma_in_array_parses_without_errors() {
    let (val, errors) = parse_json("[1, 2, 3,]");
    assert!(val.is_some());
    assert!(errors.is_empty(), "trailing comma in array is grammar");
    let Some(arr) = val.as_ref().and_then(Value::as_array) else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 3);
}

#[test]
fn missing_comma_between_members_recovers() {
    let (val, errors) = parse_json(r#"{"a": 1 "b": 2}"#);
    assert!(val.is_some());
    assert!(!errors.is_empty());
    let (warnings, _recovered, fatal) = classify_errors(errors, val.is_some());
    assert!(!warnings.is_empty());
    assert!(fatal.is_empty());
}

#[test]
fn unterminated_array_recovers_partial_value() {
    let (val, errors) = parse_json("[1, 2, 3");
    assert!(val.is_some());
    assert!(!errors.is_empty());
    let Some(arr) = val.as_ref().and_then(Value::as_array) else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 3);
    assert_eq!(arr.first().and_then(Value::as_u64), Some(1));
}

#[test]
fn array_trailing_comma_and_missing_bracket_recovery() {
    let (val, errors) = parse_json("[1, 2,");
    assert!(val.is_some());
    assert!(!errors.is_empty());
    let Some(arr) = val.as_ref().and_then(Value::as_array) else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 2);
}

// --- Edge cases ---

#[test]
fn empty_input_errors() {
    let (val, errors) = parse_json("");
    assert!(val.is_none());
    assert!(!errors.is_empty());
}

#[test]
fn whitespace_only_input_errors() {
    let (val, errors) = parse_json("   ");
    assert!(val.is_none());
    assert!(!errors.is_empty());
}

#[test]
fn negative_number_parses() {
    let (val, errors) = parse_json("-0.5");
    assert!(errors.is_empty());
    let Some(n) = val.as_ref().and_then(Value::as_f64) else {
        panic!("expected float");
    };
    assert!((n + 0.5).abs() < 1e-10);
}

#[test]
fn exponential_notation_parses() {
    let (val, errors) = parse_json("1.5e2");
    assert!(errors.is_empty());
    let Some(n) = val.as_ref().and_then(Value::as_f64) else {
        panic!("expected float");
    };
    assert!((n - 150.0).abs() < 1e-10);
}

#[test]
fn nested_arrays_parse_successfully() {
    let (val, errors) = parse_json("[[1, 2], [3, 4]]");
    assert!(errors.is_empty());
    let Some(arr) = val.as_ref().and_then(Value::as_array) else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 2);
}

#[test]
fn nested_objects_parse_successfully() {
    let json = r#"{"outer": {"inner": 42}}"#;
    let (val, errors) = parse_json(json);
    assert!(errors.is_empty());
    let Some(obj) = val.as_ref().and_then(Value::as_object) else {
        panic!("expected object");
    };
    let inner = obj.get("outer").and_then(Value::as_object);
    assert!(inner.is_some());
    assert_eq!(
        inner.and_then(|m| m.get("inner")).and_then(Value::as_u64),
        Some(42)
    );
}

#[test]
fn json_types_in_array_parse() {
    let json = r#"[null, true, false, 42, 3.5, "s", [1], {"k": "v"}]"#;
    let (val, errors) = parse_json(json);
    assert!(errors.is_empty());
    let Some(arr) = val.as_ref().and_then(Value::as_array) else {
        panic!("expected array");
    };
    assert_eq!(arr.len(), 8);
    assert_eq!(arr.first(), Some(&Value::Null));
}

// --- Diagnostics ---

#[test]
fn number_overflow_emits_diagnostic() {
    let (val, errors) = parse_json("1e999");
    assert!(val.is_some());
    assert!(!errors.is_empty());
    let (warnings, _recovered, _fatal) = classify_errors(errors, val.is_some());
    assert!(!warnings.is_empty(), "expected overflow diagnostic warning");
}

#[test]
fn leading_zero_only_number_parses_without_warning() {
    let (val, errors) = parse_json("0");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Number(serde_json::Number::from(0))));
}

#[test]
fn control_character_string_emits_warning() {
    let (val, errors) = parse_json("\"\x01\"");
    assert!(val.is_some());
    assert!(!errors.is_empty());
    let (warnings, _recovered, _fatal) = classify_errors(errors, val.is_some());
    assert!(!warnings.is_empty(), "expected control-char warning");
}

#[test]
fn leading_comma_in_object_emits_warning() {
    let (val, errors) = parse_json(r#"{, "a": 1}"#);
    assert!(val.is_some());
    assert!(!errors.is_empty());
    let (warnings, _recovered, _fatal) = classify_errors(errors, val.is_some());
    assert!(!warnings.is_empty(), "expected leading-comma warning");
}

#[test]
fn mixed_recovery_and_warnings_classified_correctly() {
    let (val, errors) = parse_json(r#"{"a": 1 "b": 2,}"#);
    assert!(val.is_some());
    assert!(!errors.is_empty());
    let (warnings, _recovered, fatal) = classify_errors(errors, val.is_some());
    assert!(!warnings.is_empty(), "expected missing-comma warning");
    assert!(fatal.is_empty());
}

// --- classify_errors edge cases ---

#[test]
fn classify_errors_empty_input() {
    let (warnings, recovered, fatal) = classify_errors(vec![], true);
    assert!(warnings.is_empty());
    assert!(recovered.is_empty());
    assert!(fatal.is_empty());
}

#[test]
fn classify_errors_custom_is_always_warning() {
    use chumsky::error::Rich;
    let err = Rich::<'_, char>::custom((0..1).into(), "test diagnostic");
    let (warnings_no_out, _, fatal_no_out) = classify_errors(vec![err.clone()], false);
    let (warnings_with_out, _, fatal_with_out) = classify_errors(vec![err], true);
    assert_eq!(warnings_no_out.len(), 1);
    assert!(fatal_no_out.is_empty());
    assert_eq!(warnings_with_out.len(), 1);
    assert!(fatal_with_out.is_empty());
}
