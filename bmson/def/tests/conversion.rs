#![expect(missing_docs, reason = "integration tests")]

use bmson_def::{BarLine, Bmson, LnMode, ModeHint};

#[test]
fn v1_to_root_basic() {
    let v1_json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test",
            "subtitle": "Sub",
            "artist": "Artist",
            "subartists": ["music:Composer"],
            "genre": "Test",
            "mode_hint": "beat-14k",
            "chart_name": "ANOTHER",
            "level": 12,
            "init_bpm": 170.0,
            "judge_rank": 100,
            "total": 100,
            "resolution": 240
        },
        "lines": [{"y": 960}],
        "bpm_events": [{"y": 0, "bpm": 170.0}],
        "stop_events": [],
        "sound_channels": [{
            "name": "kick.wav",
            "notes": [{"x": 1, "y": 0, "l": 0, "c": false}]
        }],
        "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}
    }"#;
    let v1: bmson_def::v1::Bmson = serde_json::from_str(v1_json).unwrap();
    let root: Bmson = v1.into();

    assert_eq!(root.song_info.title, "Test");
    assert_eq!(root.chart_info.subtitle, "Sub");
    assert_eq!(root.chart_info.level, 12);
    assert_eq!(root.chart_data.mode_hint, ModeHint::Beat14k);
    assert_eq!(root.chart_data.sound_channels.len(), 1);
    assert_eq!(root.chart_data.sound_channels[0].note_events.len(), 1);
    assert_eq!(root.chart_data.lines, Some(vec![BarLine { y: 960 }]));
}

#[test]
fn root_to_v1_round_trip() {
    let v1_json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test",
            "subtitle": "",
            "artist": "Artist",
            "subartists": [],
            "genre": "Test",
            "mode_hint": "beat-7k",
            "chart_name": "",
            "level": 5,
            "init_bpm": 140.0,
            "judge_rank": 100,
            "total": 100,
            "resolution": 240
        },
        "lines": null,
        "bpm_events": [],
        "stop_events": null,
        "sound_channels": [],
        "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}
    }"#;
    let v1: bmson_def::v1::Bmson = serde_json::from_str(v1_json).unwrap();
    let root: Bmson = v1.clone().into();
    let back: bmson_def::v1::Bmson = root.into();

    assert_eq!(back.version, v1.version);
    assert_eq!(back.info.title, v1.info.title);
    assert_eq!(back.info.init_bpm, v1.info.init_bpm);
    assert_eq!(back.info.level, v1.info.level);
    assert_eq!(back.lines, v1.lines);
}

#[test]
fn v0_to_root_camel_case() {
    let v0_json = r#"{
        "info": {
            "title": "Legacy",
            "artist": "Old Artist",
            "genre": "Old",
            "level": 10,
            "initBPM": 150.0,
            "judgeRank": 90.0,
            "total": 150.0
        },
        "lines": [{"y": 960, "k": 4}],
        "bpmNotes": [{"y": 0, "v": 150.0}],
        "stopNotes": [],
        "soundChannel": [{
            "name": "snare.wav",
            "notes": [{"x": 3, "y": 480, "l": 0, "c": false}]
        }],
        "bga": {"bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": []}
    }"#;
    let v0: bmson_def::v0::Bmson = serde_json::from_str(v0_json).unwrap();
    let root: Bmson = v0.try_into().unwrap();

    assert_eq!(root.song_info.title, "Legacy");
    assert_eq!(root.chart_data.init_bpm, 150.0);
    assert!((root.chart_data.judge_multiplier - 0.90).abs() < 1e-10);
    assert!((root.chart_data.life_multiplier - 1.50).abs() < 1e-10);
    assert_eq!(root.chart_data.lines, Some(vec![BarLine { y: 960 }]));
    assert_eq!(root.chart_data.sound_channels[0].note_events[0].x, 3);
}

#[test]
fn v0_stop_events_accepts_both_names() {
    let v0_json = r#"{
        "info": {"title": "T", "artist": "A", "genre": "G", "level": 1, "initBPM": 120, "judgeRank": 100, "total": 100},
        "bpmNotes": [],
        "stopNotes": [{"y": 480, "v": 48}],
        "soundChannel": [],
        "bga": {"bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": []}
    }"#;
    let v0: bmson_def::v0::Bmson = serde_json::from_str(v0_json).unwrap();
    assert_eq!(v0.stop_events.len(), 1);
    assert_eq!(v0.stop_events[0].y, 480);
    assert!((v0.stop_events[0].v - 48.0).abs() < 1e-10);

    let v0_json = r#"{
        "info": {"title": "T", "artist": "A", "genre": "G", "level": 1, "initBPM": 120, "judgeRank": 100, "total": 100},
        "bpmNotes": [],
        "stopEvents": [{"y": 240, "v": 24}],
        "soundChannel": [],
        "bga": {"bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": []}
    }"#;
    let v0: bmson_def::v0::Bmson = serde_json::from_str(v0_json).unwrap();
    assert_eq!(v0.stop_events[0].y, 240);
}

#[test]
fn v0_to_root_with_t_field_mapping() {
    let v0_json = r#"{
        "info": {"title": "T", "artist": "A", "genre": "G", "level": 1, "initBPM": 120, "judgeRank": 100, "total": 100},
        "bpmNotes": [],
        "stopNotes": [],
        "soundChannel": [{
            "name": "ln.wav",
            "notes": [
                {"x": 1, "y": 0, "l": 240, "c": false, "t": 2},
                {"x": 3, "y": 480, "l": 240, "c": false}
            ]
        }],
        "bga": {"bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": []}
    }"#;
    let v0: bmson_def::v0::Bmson = serde_json::from_str(v0_json).unwrap();
    let root: Bmson = v0.try_into().unwrap();

    let notes = &root.chart_data.sound_channels[0].note_events;
    assert_eq!(notes[0].t, Some(LnMode::Cn));
    assert_eq!(notes[0].ln_type_hint, Some(bmson_def::LnType::Cn));
    assert_eq!(notes[1].t, None);
    assert_eq!(notes[1].ln_type_hint, None);
}

#[test]
fn v0_init_bpm_negative_is_error() {
    let v0_json = r#"{
        "info": {"title": "T", "artist": "A", "genre": "G", "level": 1, "initBPM": -10, "judgeRank": 100, "total": 100},
        "bpmNotes": [], "stopNotes": [], "soundChannel": [],
        "bga": {"bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": []}
    }"#;
    let v0: bmson_def::v0::Bmson = serde_json::from_str(v0_json).unwrap();
    let result: Result<Bmson, _> = v0.try_into();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("init_bpm"), "error: {}", err.message);
}

#[test]
fn v0_init_bpm_zero_is_error() {
    let v0_json = r#"{
        "info": {"title": "T", "artist": "A", "genre": "G", "level": 1, "initBPM": 0, "judgeRank": 100, "total": 100},
        "bpmNotes": [], "stopNotes": [], "soundChannel": [],
        "bga": {"bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": []}
    }"#;
    let v0: bmson_def::v0::Bmson = serde_json::from_str(v0_json).unwrap();
    let result: Result<Bmson, _> = v0.try_into();
    assert!(result.is_err());
}

#[test]
fn root_to_v0_invalid_init_bpm_errors() {
    use bmson_def::Bmson;
    use bmson_def::{ChartData, ChartInfo, SongInfo};

    let root = Bmson {
        version: "2.0.0".to_owned(),
        song_info: SongInfo {
            title: "T".to_owned(),
            artist: "A".to_owned(),
            genre: "G".to_owned(),
        },
        chart_info: ChartInfo {
            subtitle: String::new(),
            subartists: vec![],
            chart_name: String::new(),
            level: 1,
            back_image: None,
            eyecatch_image: None,
            banner_image: None,
            preview_music: None,
            title_image: None,
            bga: bmson_def::BGA::default(),
        },
        chart_data: ChartData {
            mode_hint: bmson_def::ModeHint::Beat7k,
            ln_type_hint: bmson_def::LnType::Ln,
            ln_judge_hint: bmson_def::LnJudge::Normal,
            ln_life_hint: bmson_def::LnLife::Normal,
            init_bpm: 0.0,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            resolution: 240,
            lines: None,
            bpm_events: vec![],
            stop_events: vec![],
            sound_channels: vec![],
            judge_deltas: None,
            life_deltas: None,
        },
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    };
    let result: Result<bmson_def::v0::Bmson, _> = root.try_into();
    assert!(result.is_err());
    assert!(
        result.unwrap_err().message.contains("init_bpm"),
        "expected init_bpm error"
    );
}
