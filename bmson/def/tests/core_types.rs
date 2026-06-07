#![expect(missing_docs, reason = "integration tests")]

use bmson_def::{BGA, ChartData, ChartInfo, NoteEvent, V0LnType};

#[test]
fn note_x_null_becomes_zero() {
    let json = r#"{"y": 240, "l": 0, "c": false, "x": null}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.x, 0);
    assert!(note.is_bgm());
}

#[test]
fn note_x_missing_defaults_zero() {
    let json = r#"{"y": 240, "l": 0, "c": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.x, 0);
    assert!(note.is_bgm());
}

#[test]
fn note_x_numeric() {
    let json = r#"{"x": 5, "y": 240, "l": 0, "c": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.x, 5);
    assert!(!note.is_bgm());
}

#[test]
fn resolution_zero_becomes_default() {
    let cd: ChartData =
        serde_json::from_str(r#"{"init_bpm": 120, "resolution": 0}"#).unwrap();
    assert_eq!(cd.resolution, 240);

    let cd: ChartData =
        serde_json::from_str(r#"{"init_bpm": 120, "resolution": 480}"#).unwrap();
    assert_eq!(cd.resolution, 480);

    let cd: ChartData = serde_json::from_str(r#"{"init_bpm": 120}"#).unwrap();
    assert_eq!(cd.resolution, 240);
}

#[test]
fn bga_v0_camelcase_aliases() {
    let json = r#"{
        "bgaHeader": [{"ID": 1, "name": "bg.png"}],
        "bgaNotes": [{"y": 0, "id": 1}],
        "layerNotes": [],
        "poorNotes": []
    }"#;
    let bga: BGA = serde_json::from_str(json).unwrap();
    assert_eq!(bga.bga_header.len(), 1);
    assert_eq!(bga.bga_header[0].id, 1);
    assert_eq!(bga.bga_header[0].name, "bg.png");
    assert_eq!(bga.bga_events.len(), 1);
    assert_eq!(bga.bga_events[0].y, 0);
    assert!(bga.layer_events.is_empty());
    assert!(bga.poor_events.is_empty());
}

#[test]
fn bga_v1_snake_case() {
    let json = r#"{
        "bga_header": [{"id": 2, "name": "layer.png"}],
        "bga_events": [],
        "layer_events": [],
        "poor_events": []
    }"#;
    let bga: BGA = serde_json::from_str(json).unwrap();
    assert_eq!(bga.bga_header[0].id, 2);
}

#[test]
fn note_t_beatoraja_v0_extension() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false, "t": 2}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.t, Some(V0LnType::Cn));
}

#[test]
fn note_t_absent_is_none() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.t, None);
}

#[test]
fn note_t_round_trip() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false, "t": 3}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    let out = serde_json::to_string(&note).unwrap();
    assert!(out.contains(r#""t":3"#), "output: {out}");
    assert_eq!(note.t, Some(V0LnType::Hcn));
}

#[test]
fn v0_ln_type_serde() {
    assert_eq!(serde_json::to_string(&V0LnType::Ln).unwrap(), "1");
    assert_eq!(serde_json::to_string(&V0LnType::Cn).unwrap(), "2");
    assert_eq!(serde_json::to_string(&V0LnType::Hcn).unwrap(), "3");

    assert_eq!(serde_json::from_str::<V0LnType>("1").unwrap(), V0LnType::Ln);
    assert_eq!(serde_json::from_str::<V0LnType>("2").unwrap(), V0LnType::Cn);
    assert_eq!(serde_json::from_str::<V0LnType>("3").unwrap(), V0LnType::Hcn);

    let err = serde_json::from_str::<V0LnType>("0");
    assert!(err.is_err());
}

#[test]
fn null_arrays_become_empty() {
    let cd: ChartData = serde_json::from_str(
        r#"{"init_bpm": 120, "bpm_events": null, "stop_events": null}"#,
    )
    .unwrap();
    assert!(cd.bpm_events.is_empty());
    assert!(cd.stop_events.is_empty());

    let cd: ChartData = serde_json::from_str(r#"{"init_bpm": 120}"#).unwrap();
    assert!(cd.bpm_events.is_empty());
    assert!(cd.stop_events.is_empty());
}

#[test]
fn title_image_chart_info() {
    let json = r#"{
        "level": 1,
        "title_image": "title.png",
        "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}
    }"#;
    let info: ChartInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.title_image, Some("title.png".to_owned()));
}

#[test]
fn title_image_absent_is_none() {
    let json = r#"{
        "level": 1,
        "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}
    }"#;
    let info: ChartInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.title_image, None);
}

#[test]
fn lines_none_is_auto() {
    let cd: ChartData = serde_json::from_str(r#"{"init_bpm": 120}"#).unwrap();
    assert_eq!(cd.lines, None);

    let cd: ChartData =
        serde_json::from_str(r#"{"init_bpm": 120, "lines": null}"#).unwrap();
    assert_eq!(cd.lines, None);
}

#[test]
fn lines_empty_array_no_bars() {
    let cd: ChartData =
        serde_json::from_str(r#"{"init_bpm": 120, "lines": []}"#).unwrap();
    assert_eq!(cd.lines, Some(vec![]));
}
