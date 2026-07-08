//! bmson-def 类型的序列化 / 反序列化往返测试。
//!
//! 验证 serde JSON 往返转换产生语义等价的结果。

use std::path::Path;

use bmson_def::*;

/// 构建一个简单的 v2 谱面供测试使用。
fn make_test_bmson() -> Bmson<'static> {
    Bmson {
        version: "2.0.0",
        song_info: SongInfo {
            title: "Test Song",
            artist: "Test Artist",
            genre: "Test Genre",
        },
        chart_info: ChartInfo {
            subtitle: "(short ver.)",
            subartists: vec!["music:composer", "chart:charter"],
            chart_name: "HYPER",
            level: 8,
            back_image: Some(Path::new("bg.png")),
            eyecatch_image: None,
            banner_image: Some(Path::new("banner.png")),
            preview_music: None,
            title_image: None,
            bga: BGA::default(),
        },
        chart_data: ChartData {
            mode_hint: ModeHint::Beat7k,
            ln_type_hint: LnType::Ln,
            ln_judge_hint: LnJudge::Normal,
            ln_life_hint: LnLife::Normal,
            init_bpm: 140.0,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            resolution: 240,
            lines: None,
            bpm_events: vec![BpmEvent { y: 960, bpm: 180.0 }],
            stop_events: vec![StopEvent {
                y: 480,
                duration: 192,
            }],
            sound_channels: vec![],
            judge_deltas: None,
            life_deltas: None,
        },
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    }
}

#[test]
fn bmson_serde_roundtrip() {
    let bmson = make_test_bmson();
    let json = serde_json::to_string(&bmson).unwrap();
    let deserialized: Bmson = serde_json::from_str(&json).unwrap();

    assert_eq!(bmson.version, deserialized.version);
    assert_eq!(bmson.song_info.title, deserialized.song_info.title);
    assert_eq!(bmson.song_info.artist, deserialized.song_info.artist);
    assert_eq!(bmson.song_info.genre, deserialized.song_info.genre);
    assert_eq!(bmson.chart_info.subtitle, deserialized.chart_info.subtitle);
    assert_eq!(
        bmson.chart_info.chart_name,
        deserialized.chart_info.chart_name
    );
    assert_eq!(bmson.chart_info.level, deserialized.chart_info.level);
    assert!((bmson.chart_data.init_bpm - deserialized.chart_data.init_bpm).abs() < f64::EPSILON);
    assert_eq!(
        bmson.chart_data.bpm_events.len(),
        deserialized.chart_data.bpm_events.len()
    );
    assert_eq!(
        bmson.chart_data.stop_events.len(),
        deserialized.chart_data.stop_events.len()
    );
}

#[test]
fn bmson_minimal_roundtrip() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "init_bpm": 120.0, "lines": null, "bpm_events": [], "stop_events": [], "sound_channels": []
        }
    }"#;
    let bmson: Bmson = serde_json::from_str(json).unwrap();
    assert_eq!(bmson.version, "2.0.0");
    assert_eq!(bmson.song_info.title, "T");
}

#[test]
fn bmson_null_lines_is_some_none() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "init_bpm": 120.0, "lines": null, "bpm_events": null, "stop_events": null, "sound_channels": []
        }
    }"#;
    let bmson: Bmson = serde_json::from_str(json).unwrap();
    assert!(bmson.chart_data.lines.is_none());
    assert!(bmson.chart_data.bpm_events.is_empty());
    assert!(bmson.chart_data.stop_events.is_empty());
}

#[test]
fn sound_channel_roundtrip() {
    let json = r#"{
        "name": "kick.wav",
        "note_events": [
            { "x": 1, "y": 0, "l": 0, "c": false }
        ]
    }"#;
    let ch: SoundChannel = serde_json::from_str(json).unwrap();
    assert_eq!(ch.name, Path::new("kick.wav"));
    assert_eq!(ch.note_events.len(), 1);
    assert_eq!(ch.note_events[0].x, 1);
    assert_eq!(ch.note_events[0].y, 0);
}

#[test]
fn note_event_roundtrip() {
    let json = r#"{"x": 5, "y": 480, "l": 240, "c": true}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.x, 5);
    assert_eq!(note.y, 480);
    assert_eq!(note.l, 240);
    assert!(note.c);
}

#[test]
fn note_event_default_x_is_zero_bgm() {
    let json = r#"{"y": 0, "l": 0, "c": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.x, 0);
    assert!(note.is_bgm());
}

#[test]
fn note_event_x_null_means_bgm() {
    let json = r#"{"x": null, "y": 0, "l": 0, "c": false}"#;
    let note: NoteEvent = serde_json::from_str(json).unwrap();
    assert_eq!(note.x, 0);
}

#[test]
fn bpm_event_roundtrip() {
    let e = BpmEvent { y: 960, bpm: 180.0 };
    let json = serde_json::to_string(&e).unwrap();
    let back: BpmEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.y, 960);
    assert!((back.bpm - 180.0).abs() < f64::EPSILON);
}

#[test]
fn stop_event_roundtrip() {
    let e = StopEvent {
        y: 480,
        duration: 192,
    };
    let json = serde_json::to_string(&e).unwrap();
    let back: StopEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.y, 480);
    assert_eq!(back.duration, 192);
}

#[test]
fn bga_header_roundtrip() {
    let json = r#"{"id": 1, "name": "bg.png"}"#;
    let header: BGAHeader = serde_json::from_str(json).unwrap();
    assert_eq!(header.id, 1);
    assert_eq!(header.name, Path::new("bg.png"));
}

#[test]
fn bga_header_alias_id() {
    // v0 使用大写 "ID"，应被 alias 处理。
    let json = r#"{"ID": 2, "name": "layer.png"}"#;
    let header: BGAHeader = serde_json::from_str(json).unwrap();
    assert_eq!(header.id, 2);
}

#[test]
fn bga_event_roundtrip() {
    let e = BGAEvent { y: 480, id: 3 };
    let json = serde_json::to_string(&e).unwrap();
    let back: BGAEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.y, 480);
    assert_eq!(back.id, 3);
}

#[test]
fn mode_hint_roundtrip() {
    let cases = [
        (ModeHint::Beat5k, "beat-5k"),
        (ModeHint::Beat7k, "beat-7k"),
        (ModeHint::Popn9k, "popn-9k"),
        (ModeHint::DjAndromeda, "dj-andromeda"),
        (ModeHint::Generic(5), "generic-5keys"),
    ];
    for (hint, expected) in cases {
        let json = serde_json::to_string(&hint).unwrap();
        assert_eq!(json, format!("\"{expected}\""));
        let back: ModeHint = serde_json::from_str(&json).unwrap();
        assert_eq!(back, hint);
    }
}

#[test]
fn mode_hint_generic_k_alias() {
    // 遗留 "generic-5k" 格式（而非 "generic-5keys"）也应被接受。
    let json = "\"generic-5k\"";
    let hint: ModeHint = serde_json::from_str(json).unwrap();
    assert_eq!(hint, ModeHint::Generic(5));
}

#[test]
fn scroll_event_roundtrip() {
    let e = ScrollEvent { y: 1920, rate: 1.5 };
    let json = serde_json::to_string(&e).unwrap();
    let back: ScrollEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.y, 1920);
    assert!((back.rate - 1.5).abs() < f64::EPSILON);
}

#[test]
fn mine_channel_roundtrip() {
    let json = r#"{"name": "mine.wav", "notes": [{"x": 1, "y": 240, "damage": 25.0}]}"#;
    let mc: MineChannel = serde_json::from_str(json).unwrap();
    assert_eq!(mc.notes.len(), 1);
    assert!((mc.notes[0].damage - 25.0).abs() < f64::EPSILON);
}

#[test]
fn key_channel_roundtrip() {
    let json = r#"{"name": "key.wav", "notes": [{"x": 3, "y": 480}]}"#;
    let kc: KeyChannel = serde_json::from_str(json).unwrap();
    assert_eq!(kc.notes.len(), 1);
    assert_eq!(kc.notes[0].x, 3);
}

#[test]
fn chart_data_ln_hints_roundtrip() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": { "title": "T", "artist": "A", "genre": "G" },
        "chart_info": {
            "subtitle": "", "subartists": [], "chart_name": "", "level": 1,
            "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
        },
        "chart_data": {
            "init_bpm": 140.0, "lines": null, "bpm_events": [], "stop_events": [], "sound_channels": [],
            "ln_type_hint": "cn", "ln_judge_hint": "ticks", "ln_life_hint": "ticks"
        }
    }"#;
    let bmson: Bmson = serde_json::from_str(json).unwrap();
    assert_eq!(bmson.chart_data.ln_type_hint, LnType::Cn);
    assert_eq!(bmson.chart_data.ln_judge_hint, LnJudge::Ticks);
    assert_eq!(bmson.chart_data.ln_life_hint, LnLife::Ticks);

    // 往返序列化
    let json_out = serde_json::to_string(&bmson).unwrap();
    let back: Bmson = serde_json::from_str(&json_out).unwrap();
    assert_eq!(back.chart_data.ln_type_hint, LnType::Cn);
    assert_eq!(back.chart_data.ln_judge_hint, LnJudge::Ticks);
    assert_eq!(back.chart_data.ln_life_hint, LnLife::Ticks);
}

#[test]
fn bga_default_is_empty() {
    let bga = BGA::default();
    assert!(bga.bga_header.is_empty());
    assert!(bga.bga_events.is_empty());
    assert!(bga.layer_events.is_empty());
    assert!(bga.poor_events.is_empty());
}

#[test]
fn default_multiplier_is_one() {
    assert!((default_multiplier() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn default_resolution_is_240() {
    assert_eq!(default_resolution(), 240);
}

#[test]
fn resolution_zero_becomes_default() {
    let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":120.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[],"resolution":0}}"#;
    let bmson: Bmson = serde_json::from_str(json).unwrap();
    assert_eq!(bmson.chart_data.resolution, 240);
}
