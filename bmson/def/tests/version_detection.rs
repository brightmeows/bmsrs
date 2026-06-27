#![expect(missing_docs, reason = "integration tests")]

use bmson_def::{DetectedVersion, ModeHint};

#[test]
fn detect_v2() {
    let json = r#"{"version":"2.0.0","song_info":{}}"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V2
    );
}

#[test]
fn detect_v1() {
    let json = r#"{"version":"1.0.0","info":{}}"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V1
    );
}

#[test]
fn detect_v0_no_version() {
    let json = r#"{"info":{"title":"T"}}"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V0
    );
}

#[test]
fn detect_v0_version_starts_with_zero() {
    let json = r#"{"version":"0.2.1","info":{}}"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V0
    );
}

#[test]
fn detect_unknown_version_error() {
    let json = r#"{"version":"3.0.0"}"#;
    let err = bmson_def::detect_version(json).unwrap_err();
    assert!(matches!(err, bmson_def::BmsonError::UnknownVersion(ref v) if v == "3.0.0"));
}

#[test]
fn detect_non_string_version_error() {
    let json = r#"{"version":123}"#;
    let err = bmson_def::detect_version(json).unwrap_err();
    assert!(matches!(err, bmson_def::BmsonError::UnknownVersion(_)));
}

#[test]
fn detect_empty_json_is_v0() {
    assert_eq!(
        bmson_def::detect_version("{}").unwrap(),
        DetectedVersion::V0
    );
}

const fn v2_json() -> &'static str {
    r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": { "init_bpm": 120.0 }
    }"#
}

const fn v1_json() -> &'static str {
    r#"{
        "version": "1.0.0",
        "info": {
            "title": "T", "artist": "A", "genre": "G", "level": 1,
            "init_bpm": 120.0
        },
        "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
    }"#
}

const fn v0_json() -> &'static str {
    r#"{
        "info": {
            "title": "T", "artist": "A", "genre": "G", "level": 1,
            "initBPM": 120.0
        },
        "soundChannel": [],
        "bga": { "bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": [] }
    }"#
}

#[test]
fn detect_then_parse_v2() {
    assert_eq!(
        bmson_def::detect_version(v2_json()).unwrap(),
        DetectedVersion::V2
    );
    let bmson: bmson_def::Bmson<'_> = serde_json::from_str(v2_json()).unwrap();
    assert_eq!(bmson.version, "2.0.0");
    assert_eq!(bmson.song_info.title, "T");
}

#[test]
fn detect_then_parse_v1() {
    assert_eq!(
        bmson_def::detect_version(v1_json()).unwrap(),
        DetectedVersion::V1
    );
    let v1: bmson_def::v1::Bmson<'_> = serde_json::from_str(v1_json()).unwrap();
    let bmson: bmson_def::Bmson<'_> = v1.into();
    assert_eq!(bmson.version, "1.0.0");
    assert_eq!(bmson.song_info.title, "T");
}

#[test]
fn detect_then_parse_v0() {
    assert_eq!(
        bmson_def::detect_version(v0_json()).unwrap(),
        DetectedVersion::V0
    );
    let v0: bmson_def::v0::Bmson<'_> = serde_json::from_str(v0_json()).unwrap();
    let bmson: bmson_def::Bmson<'_> = v0.try_into().unwrap();
    assert_eq!(bmson.version, "0.2.1");
    assert_eq!(bmson.song_info.title, "T");
}

#[test]
fn from_slice_v0_negative_bpm_error() {
    let json = r#"{
        "info": { "title": "T", "artist": "A", "genre": "G", "level": 1, "initBPM": -5.0 },
        "soundChannel": [],
        "bga": { "bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": [] }
    }"#;
    let v0: bmson_def::v0::Bmson<'_> = serde_json::from_str(json).unwrap();
    let err: Result<bmson_def::Bmson<'_>, _> = v0.try_into();
    assert!(err.is_err());
}

#[test]
fn serialize_v2_then_detect() {
    let v2: bmson_def::Bmson<'_> = serde_json::from_str(v2_json()).unwrap();
    let out = serde_json::to_string(&v2).unwrap();
    assert_eq!(
        bmson_def::detect_version(&out).unwrap(),
        DetectedVersion::V2
    );
}

#[test]
fn serialize_v1_then_detect() {
    let bmson: bmson_def::Bmson<'_> = {
        let v1: bmson_def::v1::Bmson<'_> = serde_json::from_str(v1_json()).unwrap();
        v1.into()
    };
    let v1_back: bmson_def::v1::Bmson<'_> = bmson.into();
    let out = serde_json::to_string(&v1_back).unwrap();
    assert_eq!(
        bmson_def::detect_version(&out).unwrap(),
        DetectedVersion::V1
    );
}

#[test]
fn detect_v1_with_mode_hint() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "T", "artist": "A", "genre": "G", "level": 1,
            "init_bpm": 140.0, "mode_hint": "popn-9k"
        },
        "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
    }"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V1
    );
    let v1: bmson_def::v1::Bmson<'_> = serde_json::from_str(json).unwrap();
    let bmson: bmson_def::Bmson<'_> = v1.into();
    assert_eq!(bmson.chart_data.mode_hint, ModeHint::Popn9k);
}

#[test]
fn detect_version_string_with_extra_whitespace() {
    // 扫描器处理 "version" 与值之间的空白。
    let json = r#"{  "version"  :  "2.0.0"  ,"song_info":{}}"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V2
    );
}

#[test]
fn detect_version_first_in_object() {
    let json = r#"{"version":"1.0.0"}"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V1
    );
}

#[test]
fn detect_version_not_first_key() {
    // version 字段可能出现在其他字段之后
    let json = r#"{"x":1,"version":"2.0.0","y":2}"#;
    assert_eq!(
        bmson_def::detect_version(json).unwrap(),
        DetectedVersion::V2
    );
}
