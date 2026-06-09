#![expect(missing_docs, reason = "integration tests")]

mod common;

use bmson_de_chumsky::{BmsonDeError, from_str};

#[test]
fn v2_roundtrip_parses_successfully() {
    let bmson = from_str(common::minimal_v2_json()).unwrap();
    assert_eq!(bmson.song_info.title, "T");
    assert_eq!(bmson.song_info.artist, "A");
    assert_eq!(bmson.song_info.genre, "G");
    assert_eq!(bmson.chart_info.level, 1);
}

#[test]
fn v1_is_converted_to_v2() {
    let bmson = from_str(common::minimal_v1_json()).unwrap();
    assert_eq!(bmson.song_info.title, "T");
    assert_eq!(bmson.song_info.artist, "A");
    assert_eq!(bmson.chart_info.level, 1);
}

#[test]
fn v0_is_converted_to_v2() {
    let bmson = from_str(common::minimal_v0_json()).unwrap();
    assert_eq!(bmson.song_info.title, "T");
    assert_eq!(bmson.song_info.artist, "A");
    assert_eq!(bmson.chart_info.level, 1);
}

#[test]
fn unknown_version_returns_error() {
    let json = r#"{"version":"3.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]}}"#;
    let result = from_str(json);
    assert!(result.is_err());
    match result {
        Err(BmsonDeError::UnknownVersion(_)) => {}
        Err(other) => panic!("expected UnknownVersion, got: {other}"),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn malformed_json_returns_parse_error() {
    let result = from_str("not json at all");
    assert!(result.is_err());
}

#[test]
fn empty_input_returns_parse_error() {
    let result = from_str("");
    assert!(result.is_err());
}

#[test]
fn v0_negative_bpm_returns_conversion_error() {
    let json = r#"{"info":{"title":"T","artist":"A","genre":"G","initBPM":-1.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let result = from_str(json);
    assert!(result.is_err());
    match result {
        Err(BmsonDeError::V0Conversion(_)) => {}
        Err(other) => panic!("expected V0Conversion, got: {other}"),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn v1_missing_required_field_returns_deserialize_error() {
    let json = r#"{"version":"1.0.0","info":{"title":"T","genre":"G","init_bpm":140.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let result = from_str(json);
    assert!(result.is_err());
    match result {
        Err(BmsonDeError::Deserialize {
            version: "v1.0.0", ..
        }) => {}
        Err(other) => panic!("expected Deserialize(v1), got: {other}"),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn trailing_comma_in_v2_returns_deserialize_with_diagnostics() {
    let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},}"#;
    let result = from_str(json);
    assert!(result.is_err());
    match &result {
        Err(BmsonDeError::Deserialize { message, .. }) => {
            assert!(
                message.contains("trailing comma") || message.contains("diagnostics"),
                "expected chumsky diagnostic in error message, got: {message}"
            );
        }
        Err(other) => panic!("expected Deserialize, got: {other}"),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn v2_with_sound_channels_roundtrip() {
    let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":5,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":180.0,"lines":null,"bpm_events":[{"y":0,"bpm":180.0}],"stop_events":[],"sound_channels":[{"name":"kick.wav","note_events":[{"x":1,"y":0,"l":0,"c":false}]},{"name":"snare.wav","note_events":[{"x":3,"y":240,"l":0,"c":false}]}]}}"#;
    let bmson = from_str(json).unwrap();
    assert_eq!(bmson.chart_info.level, 5);
    assert_eq!(bmson.chart_data.bpm_events.len(), 1);
    assert_eq!(bmson.chart_data.sound_channels.len(), 2);
    let Some(sc) = bmson.chart_data.sound_channels.first() else {
        panic!("expected at least one sound channel");
    };
    assert_eq!(sc.name, std::path::Path::new("kick.wav"));
}

#[test]
fn v1_with_stop_events_roundtrip() {
    let json = r#"{"version":"1.0.0","info":{"title":"T","artist":"A","genre":"G","init_bpm":140.0,"level":1},"bpm_events":[{"y":0,"bpm":140.0}],"stop_events":[{"y":480,"duration":240}],"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let bmson = from_str(json).unwrap();
    assert_eq!(bmson.chart_data.bpm_events.len(), 1);
    assert_eq!(bmson.chart_data.stop_events.len(), 1);
    let Some(stop) = bmson.chart_data.stop_events.first() else {
        panic!("expected at least one stop event");
    };
    assert_eq!(stop.duration, 240);
}

#[test]
fn v0_with_sound_channel_roundtrip() {
    let json = r#"{"info":{"title":"T","artist":"A","genre":"G","initBPM":140.0,"level":1},"soundChannel":[{"name":"hat.wav","notes":[{"x":2,"y":120,"l":0,"c":false}]}],"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let bmson = from_str(json).unwrap();
    assert_eq!(bmson.chart_data.sound_channels.len(), 1);
    let Some(sc) = bmson.chart_data.sound_channels.first() else {
        panic!("expected at least one sound channel");
    };
    let Some(note) = sc.note_events.first() else {
        panic!("expected at least one note");
    };
    assert_eq!(note.x, 2);
}

#[test]
fn v2_scroll_events_roundtrip() {
    let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]},"scroll_events":[{"y":0,"rate":1.0},{"y":960,"rate":2.0}]}"#;
    let bmson = from_str(json).unwrap();
    assert_eq!(bmson.scroll_events.len(), 2);
    let Some(se) = bmson.scroll_events.get(1) else {
        panic!("expected second scroll event");
    };
    assert!((se.rate - 2.0).abs() < 1e-10);
}
