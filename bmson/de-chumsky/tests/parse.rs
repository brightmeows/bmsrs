//! bmson-de-chumsky 的集成测试。

use bmson_de_chumsky::BmsonDeError;
use std::error::Error;

/// 一个最小的有效 v2 BMSON JSON 字符串。
const fn minimal_v2_json() -> &'static str {
    r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "init_bpm": 140.0, "lines": null, "bpm_events": [], "stop_events": [], "sound_channels": []
        }
    }"#
}

#[test]
fn parse_valid_v2() {
    let bmson = bmson_de_chumsky::BmsonParser::parse(minimal_v2_json()).unwrap();
    assert_eq!(bmson.version, "2.0.0");
    assert_eq!(bmson.song_info.title, "T");
    assert_eq!(bmson.song_info.artist, "A");
    assert_eq!(bmson.song_info.genre, "G");
    assert!((bmson.chart_data.init_bpm - 140.0).abs() < f64::EPSILON);
}

#[test]
fn parse_v1_auto_converts_to_v2() {
    let json = r#"{
        "version": "1.0.0",
        "info": { "title": "V1 Song", "artist": "V1 Artist", "genre": "V1 Genre", "level": 1, "init_bpm": 120.0 },
        "bpm_events": [],
        "stop_events": [],
        "sound_channels": [],
        "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
    }"#;
    let bmson = bmson_de_chumsky::BmsonParser::parse(json).unwrap();
    assert_eq!(bmson.song_info.title, "V1 Song");
    assert_eq!(bmson.song_info.artist, "V1 Artist");
    assert!((bmson.chart_data.init_bpm - 120.0).abs() < f64::EPSILON);
}

#[test]
fn parse_v0_auto_converts_to_v2() {
    let json = r#"{
        "info": { "title": "V0 Song", "artist": "V0 Artist", "genre": "V0 Genre", "level": 1, "initBPM": 130.0, "judgeRank": 100 },
        "lines": null,
        "bpmNotes": [],
        "stopEvents": [],
        "soundChannel": [],
        "bga": { "bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": [] }
    }"#;
    let bmson = bmson_de_chumsky::BmsonParser::parse(json).unwrap();
    assert_eq!(bmson.song_info.title, "V0 Song");
    assert_eq!(bmson.song_info.artist, "V0 Artist");
    assert!((bmson.chart_data.init_bpm - 130.0).abs() < f64::EPSILON);
}

#[test]
fn parse_with_bpm_and_stop_events() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "init_bpm": 140.0, "lines": null,
            "bpm_events": [{"y": 960, "bpm": 180.0}],
            "stop_events": [{"y": 480, "duration": 192}],
            "sound_channels": []
        }
    }"#;
    let bmson = bmson_de_chumsky::BmsonParser::parse(json).unwrap();
    assert_eq!(bmson.chart_data.bpm_events.len(), 1);
    assert!((bmson.chart_data.bpm_events[0].bpm - 180.0).abs() < f64::EPSILON);
    assert_eq!(bmson.chart_data.stop_events.len(), 1);
}

#[test]
fn parse_with_sound_channels() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "init_bpm": 140.0, "lines": null, "bpm_events": [], "stop_events": [],
            "sound_channels": [
                {"name": "kick.wav", "note_events": [{"x": 1, "y": 0, "l": 0, "c": false}]}
            ]
        }
    }"#;
    let bmson = bmson_de_chumsky::BmsonParser::parse(json).unwrap();
    assert_eq!(bmson.chart_data.sound_channels.len(), 1);
    assert_eq!(bmson.chart_data.sound_channels[0].note_events.len(), 1);
}

#[test]
fn parse_invalid_json_returns_error() {
    let json = r"{not valid json";
    let result = bmson_de_chumsky::BmsonParser::parse(json);
    assert!(result.is_err());
}

#[test]
fn parse_unknown_version_returns_error() {
    let json = r#"{"version":"3.0.0","song_info":{},"chart_info":{"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":120.0}}"#;
    let result = bmson_de_chumsky::BmsonParser::parse(json);
    assert!(result.is_err());
    match result {
        Err(BmsonDeError::UnknownVersion(_)) => {}
        _ => panic!("expected UnknownVersion error"),
    }
}

#[test]
fn parse_missing_required_field_returns_error() {
    // 缺少必填的 init_bpm 字段
    let json = r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "lines": null, "bpm_events": [], "stop_events": [], "sound_channels": []
        }
    }"#;
    let result = bmson_de_chumsky::BmsonParser::parse(json);
    assert!(result.is_err());
}

#[test]
fn parse_with_trailing_comma_produces_deserialize_error() {
    // chumsky 能恢复 trailing comma，但 serde_json 拒绝它。
    // 结果是 Deserialize 错误。
    let json = r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "init_bpm": 140.0, "lines": null, "bpm_events": [], "stop_events": [], "sound_channels": [],
        },
    }"#;
    let result = bmson_de_chumsky::BmsonParser::parse(json);
    // 预期为 Deserialize 错误（因为 serde_json 拒绝 trailing comma）
    // 或 JsonParse 错误（取决于 chumsky 的分类）。
    assert!(result.is_err());
}

#[test]
fn parse_error_display() {
    let err = BmsonDeError::JsonParse("unexpected token".into());
    let display = err.to_string();
    assert!(!display.is_empty());
}

#[test]
fn parse_error_debug() {
    let err = BmsonDeError::Deserialize {
        version: "v2.0.0",
        message: "missing field".into(),
    };
    let debug = format!("{err:?}");
    assert!(!debug.is_empty());
}

#[test]
fn parse_error_impl_std_error() {
    fn assert_error<T: Error>() {}
    assert_error::<BmsonDeError>();
}
