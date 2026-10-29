//! Integration tests for the `detect_version` function.
//!
//! Verify that version detection handles v0/v1/v2 JSON and various edge cases.

use bmsrs_bmson_def::DetectedVersion;

#[test]
fn detect_v2_from_full_json() {
    let json = r#"{"version":"2.0.0","song_info":{},"chart_info":{},"chart_data":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V2);
}

#[test]
fn detect_v2_without_patch_is_error() {
    // A bare "2" is not a valid SemVer prefix; the format must be "2.x.y".
    let json = r#"{"version":"2","song_info":{}}"#;
    assert!(DetectedVersion::detect(json).is_err());
}

#[test]
fn detect_v2_rc_version() {
    let json = r#"{"version":"2.0.0-rc1","song_info":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V2);
}

#[test]
fn detect_v1_from_full_json() {
    let json = r#"{"version":"1.0.0","info":{},"bga":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V1);
}

#[test]
fn detect_v1_without_patch_is_error() {
    // A bare "1" is not a valid SemVer prefix; the format must be "1.x.y".
    let json = r#"{"version":"1","info":{}}"#;
    assert!(DetectedVersion::detect(json).is_err());
}

#[test]
fn detect_v0_when_no_version_field() {
    let json = r#"{"info":{"title":"Test"}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V0);
}

#[test]
fn detect_v0_when_version_starts_with_zero() {
    let json = r#"{"version":"0.2.1","info":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V0);
}

#[test]
fn detect_empty_object_is_v0() {
    let json = "{}";
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V0);
}

#[test]
fn version_with_invalid_prefix_returns_error() {
    // "2invalid" starts with '2' but not with "2.", so it should be rejected.
    let json = r#"{"version":"2invalid","song_info":{}}"#;
    assert!(DetectedVersion::detect(json).is_err());
}

#[test]
fn version_with_extra_digits_is_still_valid() {
    // "2.0.0.1" is not standard SemVer but starts with "2.", so it should be V2.
    let json = r#"{"version":"2.0.0.1","song_info":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V2);
}

#[test]
fn unknown_version_returns_error() {
    let json = r#"{"version":"3.0.0","song_info":{}}"#;
    assert!(DetectedVersion::detect(json).is_err());
}

#[test]
fn version_with_letters_returns_error() {
    let json = r#"{"version":"alpha","song_info":{}}"#;
    assert!(DetectedVersion::detect(json).is_err());
}

#[test]
fn version_field_not_a_string_returns_error() {
    let json = r#"{"version":2,"song_info":{}}"#;
    assert!(DetectedVersion::detect(json).is_err());
}

#[test]
fn version_scan_ignores_other_occurrences() {
    // "version" may appear as a value in "title" or "chart_name"; only keys
    // match.
    let json = r#"{"title":"version 2","song_info":{},"chart_info":{},"chart_data":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V0);
}

#[test]
fn detect_multiline_json() {
    let json = "{\n  \"version\": \"2.0.0\",\n  \"song_info\": {}\n}";
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V2);
}

#[test]
fn detect_ignores_nested_version_key() {
    // A "version" key inside a nested object does not participate in top-level
    // version detection.
    let json = r#"{"song_info":{"version":"1.0.0"},"chart_info":{},"chart_data":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V0);
}
