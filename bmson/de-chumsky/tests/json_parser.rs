//! 底层 chumsky JSON 解析器的集成测试。

use bmson_de_chumsky::json;

#[test]
fn parse_json_primitive_string() {
    let (value, errs) = json::parse_json(r#""hello""#);
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_number_integer() {
    let (value, errs) = json::parse_json("42");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_number_float() {
    let (value, errs) = json::parse_json("3.14");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_boolean_true() {
    let (value, errs) = json::parse_json("true");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_boolean_false() {
    let (value, errs) = json::parse_json("false");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_null() {
    let (value, errs) = json::parse_json("null");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_empty_object() {
    let (value, errs) = json::parse_json("{}");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_empty_array() {
    let (value, errs) = json::parse_json("[]");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_object() {
    let json = r#"{"key": "value", "num": 42}"#;
    let (value, errs) = json::parse_json(json);
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_nested_object() {
    let json = r#"{"outer": {"inner": true}}"#;
    let (value, errs) = json::parse_json(json);
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_array() {
    let json = r"[1, 2, 3]";
    let (value, errs) = json::parse_json(json);
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_negative_number() {
    let (value, errs) = json::parse_json("-42");
    assert!(value.is_some());
    assert!(errs.is_empty());
}

#[test]
fn parse_json_trailing_comma_recovered() {
    let json = r#"{"key": "value",}"#;
    let (value, _) = json::parse_json(json);
    assert!(value.is_some());
}

#[test]
fn parse_json_completely_invalid_returns_none() {
    let (value, _) = json::parse_json("not json at all !!!");
    assert!(value.is_none());
}

#[test]
fn parse_json_empty_input_returns_none() {
    let (value, _) = json::parse_json("");
    assert!(value.is_none());
}

#[test]
fn parse_json_classify_errors_recovered() {
    let json = r#"{"valid": true,}"#;
    let (value, errs) = json::parse_json(json);
    assert!(value.is_some());
    let (_warnings, _recovered, fatal) = json::classify_errors(errs, false);
    assert!(
        !fatal.iter().any(|e| format!("{e}").contains("trailing")),
        "trailing comma should not be fatal"
    );
}

#[test]
fn parse_json_missing_brace_recovered() {
    let json = r#"{"key": "value""#;
    let (value, _) = json::parse_json(json);
    assert!(value.is_some());
}

#[test]
fn parse_json_classify_no_errors_idle() {
    let (value, errs) = json::parse_json(r#"{"a":1}"#);
    assert!(value.is_some());
    let (warnings, _recovered, fatal) = json::classify_errors(errs, true);
    assert!(warnings.is_empty());
    assert!(fatal.is_empty());
}

#[test]
fn parse_json_classify_with_fatal() {
    let (value, errs) = json::parse_json("{invalid");
    let (_warnings, _recovered, fatal) = json::classify_errors(errs, false);
    if value.is_none() {
        assert!(!fatal.is_empty(), "expected at least one fatal error");
    }
}
