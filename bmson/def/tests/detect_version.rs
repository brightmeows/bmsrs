//! `detect_version` 函数的集成测试。
//!
//! 验证版本检测函数能正确处理 v0/v1/v2 JSON 以及各种边缘情况。

use bmson_def::DetectedVersion;

#[test]
fn detect_v2_from_full_json() {
    let json = r#"{"version":"2.0.0","song_info":{},"chart_info":{},"chart_data":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V2);
}

#[test]
fn detect_v2_without_patch() {
    let json = r#"{"version":"2","song_info":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V2);
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
fn detect_v1_without_patch() {
    let json = r#"{"version":"1","info":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V1);
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
    // version 可能作为值出现在"title"或"chart_name"中——只匹配键。
    let json = r#"{"title":"version 2","song_info":{},"chart_info":{},"chart_data":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V0);
}

#[test]
fn detect_multiline_json() {
    let json = "{\n  \"version\": \"2.0.0\",\n  \"song_info\": {}\n}";
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V2);
}

#[test]
fn detect_first_version_wins() {
    // 当多个 "version" 键存在时，取第一个。
    // 当前实现使用 `find` 扫描，因此第一处匹配胜出。
    let json = r#"{"song_info":{"version":"1.0.0"},"chart_info":{},"chart_data":{}}"#;
    assert_eq!(DetectedVersion::detect(json).unwrap(), DetectedVersion::V1);
}
