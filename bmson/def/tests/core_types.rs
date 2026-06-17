#![expect(missing_docs, reason = "integration tests")]

use std::path::Path;

use bmson_def::{
    BGA, ChartData, ChartInfo, JudgementDeltas, LifeDeltas, LnJudge, LnLife, LnMode, LnType,
    NoteEvent,
};

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
    let cd: ChartData = serde_json::from_str(r#"{"init_bpm": 120, "resolution": 0}"#).unwrap();
    assert_eq!(cd.resolution, 240);

    let cd_480: ChartData =
        serde_json::from_str(r#"{"init_bpm": 120, "resolution": 480}"#).unwrap();
    assert_eq!(cd_480.resolution, 480);

    let cd_default: ChartData = serde_json::from_str(r#"{"init_bpm": 120}"#).unwrap();
    assert_eq!(cd_default.resolution, 240);
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
    assert_eq!(bga.bga_header[0].name, Path::new("bg.png"));
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
    assert_eq!(note.t, Some(LnMode::Cn));
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
    assert_eq!(note.t, Some(LnMode::Hcn));
}

#[test]
fn ln_mode_serde() {
    assert_eq!(serde_json::to_string(&LnMode::Ln).unwrap(), "1");
    assert_eq!(serde_json::to_string(&LnMode::Cn).unwrap(), "2");
    assert_eq!(serde_json::to_string(&LnMode::Hcn).unwrap(), "3");

    assert_eq!(serde_json::from_str::<LnMode>("1").unwrap(), LnMode::Ln);
    assert_eq!(serde_json::from_str::<LnMode>("2").unwrap(), LnMode::Cn);
    assert_eq!(serde_json::from_str::<LnMode>("3").unwrap(), LnMode::Hcn);

    let err = serde_json::from_str::<LnMode>("0");
    assert!(err.is_err());
}

#[test]
fn null_arrays_become_empty() {
    let cd: ChartData =
        serde_json::from_str(r#"{"init_bpm": 120, "bpm_events": null, "stop_events": null}"#)
            .unwrap();
    assert!(cd.bpm_events.is_empty());
    assert!(cd.stop_events.is_empty());

    let cd_no_events: ChartData = serde_json::from_str(r#"{"init_bpm": 120}"#).unwrap();
    assert!(cd_no_events.bpm_events.is_empty());
    assert!(cd_no_events.stop_events.is_empty());
}

#[test]
fn title_image_chart_info() {
    let json = r#"{
        "level": 1,
        "title_image": "title.png",
        "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}
    }"#;
    let info: ChartInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.title_image, Some(Path::new("title.png")));
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

    let cd_null: ChartData = serde_json::from_str(r#"{"init_bpm": 120, "lines": null}"#).unwrap();
    assert_eq!(cd_null.lines, None);
}

#[test]
fn lines_empty_array_no_bars() {
    let cd: ChartData = serde_json::from_str(r#"{"init_bpm": 120, "lines": []}"#).unwrap();
    assert_eq!(cd.lines, Some(vec![]));
}

#[test]
fn note_v2_up_true() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false, "up": true}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.up, Some(true));
}

#[test]
fn note_v2_up_false() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false, "up": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.up, Some(false));
}

#[test]
fn note_v2_up_absent_is_none() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.up, None);
}

#[test]
fn note_v2_vol_pan() {
    let json = r#"{"x": 1, "y": 240, "l": 0, "c": false, "vol": 80, "pan": -50}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.vol, Some(80));
    assert_eq!(note.pan, Some(-50));
}

#[test]
fn note_v2_vol_pan_absent() {
    let json = r#"{"x": 1, "y": 240, "l": 0, "c": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.vol, None);
    assert_eq!(note.pan, None);
}

#[test]
fn note_v2_ln_type_hint() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false, "ln_type_hint": "cn"}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.ln_type_hint, Some(LnType::Cn));
}

#[test]
fn note_v2_ln_judge_hint() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false, "ln_judge_hint": "ticks"}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.ln_judge_hint, Some(LnJudge::Ticks));
}

#[test]
fn note_v2_ln_life_hint() {
    let json = r#"{"x": 1, "y": 240, "l": 240, "c": false, "ln_life_hint": "ticks"}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.ln_life_hint, Some(LnLife::Ticks));
}

#[test]
fn note_v2_all_optional_fields_round_trip() {
    let json = r#"{
        "x": 3, "y": 480, "l": 240, "c": false,
        "up": true,
        "vol": 75,
        "pan": 0,
        "ln_type_hint": "cn",
        "ln_judge_hint": "ticks",
        "ln_life_hint": "normal"
    }"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    let out = serde_json::to_string(&note).unwrap();
    let restored: NoteEvent = serde_json::from_str(&out).unwrap();
    assert_eq!(note, restored);
}

#[test]
fn judge_deltas_serde() {
    let d: JudgementDeltas =
        serde_json::from_str(r#"{"perfect": 10, "great": 20, "good": 50, "miss": 100}"#).unwrap();
    assert_eq!(d.perfect, 10);
    assert_eq!(d.great, 20);
    assert_eq!(d.good, 50);
    assert_eq!(d.miss, 100);
}

#[test]
fn life_deltas_serde() {
    let d: LifeDeltas =
        serde_json::from_str(r#"{"perfect": 2.0, "great": 1.0, "good": 0.0, "miss": -4.0}"#)
            .unwrap();
    assert!((d.perfect - 2.0).abs() < 1e-10);
    assert!((d.great - 1.0).abs() < 1e-10);
    assert!((d.good - 0.0).abs() < 1e-10);
    assert!((d.miss - -4.0).abs() < 1e-10);
}

#[test]
fn chart_data_judge_life_deltas() {
    let json = r#"{
        "init_bpm": 120,
        "judge_deltas": {"perfect": 5, "great": 15, "good": 40, "miss": 80},
        "life_deltas": {"perfect": 1.5, "great": 0.5, "good": -0.5, "miss": -5.0}
    }"#;
    let cd: ChartData = serde_json::from_str(json).unwrap();
    assert_eq!(cd.judge_deltas.unwrap().perfect, 5);
    assert!((cd.life_deltas.unwrap().miss - -5.0).abs() < 1e-10);
}

#[test]
fn bmson_scroll_events() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": {"title": "T", "artist": "A", "genre": "G"},
        "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
        "chart_data": {"init_bpm": 120},
        "scroll_events": [
            {"y": 0, "rate": 1.0},
            {"y": 240, "rate": 2.5},
            {"y": 480, "rate": -1.0}
        ]
    }"#;
    let root: bmson_def::Bmson = serde_json::from_str(json).unwrap();
    assert_eq!(root.scroll_events.len(), 3);
    assert!((root.scroll_events[1].rate - 2.5).abs() < 1e-10);
    assert!((root.scroll_events[2].rate - -1.0).abs() < 1e-10);
}

#[test]
fn bmson_mine_channels() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": {"title": "T", "artist": "A", "genre": "G"},
        "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
        "chart_data": {"init_bpm": 120},
        "mine_channels": [{
            "name": "mine.wav",
            "notes": [
                {"x": 1, "y": 120, "damage": 10.5},
                {"x": 3, "y": 240, "damage": 25.0}
            ]
        }]
    }"#;
    let root: bmson_def::Bmson = serde_json::from_str(json).unwrap();
    assert_eq!(root.mine_channels.len(), 1);
    assert_eq!(root.mine_channels[0].name, Path::new("mine.wav"));
    assert!((root.mine_channels[0].notes[0].damage - 10.5).abs() < 1e-10);
}

#[test]
fn bmson_key_channels() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": {"title": "T", "artist": "A", "genre": "G"},
        "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
        "chart_data": {"init_bpm": 120},
        "key_channels": [{
            "name": "ghost.wav",
            "notes": [
                {"x": 0, "y": 0},
                {"x": 2, "y": 120}
            ]
        }]
    }"#;
    let root: bmson_def::Bmson = serde_json::from_str(json).unwrap();
    assert_eq!(root.key_channels.len(), 1);
    assert_eq!(root.key_channels[0].notes[1].x, 2);
}

#[test]
fn bmson_extensions_absent_default_empty() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": {"title": "T", "artist": "A", "genre": "G"},
        "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
        "chart_data": {"init_bpm": 120}
    }"#;
    let root: bmson_def::Bmson = serde_json::from_str(json).unwrap();
    assert!(root.scroll_events.is_empty());
    assert!(root.mine_channels.is_empty());
    assert!(root.key_channels.is_empty());
}

#[test]
fn bga_header_round_trip() {
    let json = r#"{"bga_header": [{"id": 1, "name": "bg.png"}], "bga_events": [], "layer_events": [], "poor_events": []}"#;
    let bga: bmson_def::BGA = serde_json::from_str(json).unwrap();
    assert_eq!(bga.bga_header[0].name, Path::new("bg.png"));

    let serialized = serde_json::to_string(&bga).unwrap();
    let bga2: bmson_def::BGA = serde_json::from_str(&serialized).unwrap();
    assert_eq!(bga2.bga_header[0].name, Path::new("bg.png"));
    assert_eq!(bga.bga_header[0].id, bga2.bga_header[0].id);
}

#[test]
fn sound_channel_round_trip() {
    let json = r#"{"name": "kick.wav", "note_events": [{"x": 1, "y": 0, "l": 0, "c": false}]}"#;
    let ch: bmson_def::SoundChannel = serde_json::from_str(json).unwrap();
    assert_eq!(ch.name, Path::new("kick.wav"));

    let serialized = serde_json::to_string(&ch).unwrap();
    let ch2: bmson_def::SoundChannel = serde_json::from_str(&serialized).unwrap();
    assert_eq!(ch2.name, Path::new("kick.wav"));
    assert_eq!(ch.note_events, ch2.note_events);
}

#[test]
fn chart_info_image_round_trip() {
    let json = r#"{
        "subtitle": "",
        "subartists": [],
        "chart_name": "",
        "level": 1,
        "back_image": "bg.png",
        "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}
    }"#;
    let info: bmson_def::ChartInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.back_image, Some(Path::new("bg.png")));

    let serialized = serde_json::to_string(&info).unwrap();
    let info2: bmson_def::ChartInfo = serde_json::from_str(&serialized).unwrap();
    assert_eq!(info2.back_image, Some(Path::new("bg.png")));
}
