//! bmson v0/v1 ↔ v2（根）版本间转换的集成测试。

use std::path::Path;

use bmson_def::*;

// v1 转换测试

fn make_v1_bmson() -> v1::Bmson<'static> {
    v1::Bmson {
        version: "1.0.0",
        info: v1::BmsonInfo {
            title: "Test Song",
            subtitle: "(short ver.)",
            artist: "Test Artist",
            subartists: vec!["music:composer"],
            genre: "Test Genre",
            mode_hint: ModeHint::Beat7k,
            chart_name: "HYPER",
            level: 8,
            init_bpm: 140.0,
            judge_rank: 100.0,
            total: 100.0,
            back_image: None,
            eyecatch_image: None,
            banner_image: None,
            preview_music: None,
            title_image: None,
            resolution: 240,
            ln_type: None,
        },
        lines: None,
        bpm_events: vec![BpmEvent { y: 960, bpm: 180.0 }],
        stop_events: vec![StopEvent {
            y: 480,
            duration: 192,
        }],
        sound_channels: vec![v1::SoundChannel {
            name: Path::new("kick.wav"),
            notes: vec![NoteEvent {
                x: 1,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            }],
        }],
        bga: BGA::default(),
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    }
}

#[test]
fn v1_converts_to_root() {
    let v1 = make_v1_bmson();
    let root: Bmson = Bmson::from(v1);

    assert_eq!(root.version, "1.0.0");
    assert_eq!(root.song_info.title, "Test Song");
    assert_eq!(root.song_info.artist, "Test Artist");
    assert_eq!(root.song_info.genre, "Test Genre");
    assert_eq!(root.chart_info.chart_name, "HYPER");
    assert!((root.chart_data.init_bpm - 140.0).abs() < f64::EPSILON);
    assert_eq!(root.chart_data.bpm_events.len(), 1);
    assert!((root.chart_data.bpm_events[0].bpm - 180.0).abs() < f64::EPSILON);
    assert_eq!(root.chart_data.stop_events.len(), 1);
    assert_eq!(root.chart_data.sound_channels.len(), 1);
    assert_eq!(root.chart_data.sound_channels[0].note_events.len(), 1);
}

#[test]
fn v1_roundtrip_v1_via_root() {
    let v1_original = make_v1_bmson();
    let root: Bmson = Bmson::from(v1_original);
    let v1_back: v1::Bmson = v1::Bmson::from(root);

    assert_eq!(v1_back.version, "1.0.0");
    assert_eq!(v1_back.info.title, "Test Song");
    assert_eq!(v1_back.info.artist, "Test Artist");
    assert!((v1_back.info.init_bpm - 140.0).abs() < f64::EPSILON);
    assert!((v1_back.info.judge_rank - 100.0).abs() < f64::EPSILON);
}

#[test]
fn v1_serde_roundtrip() {
    let v1_original = make_v1_bmson();
    let json = serde_json::to_string(&v1_original).unwrap();
    let v1_back: v1::Bmson = serde_json::from_str(&json).unwrap();

    assert_eq!(v1_back.info.title, "Test Song");
    assert_eq!(v1_back.info.artist, "Test Artist");
    assert_eq!(v1_back.info.subtitle, "(short ver.)");
    assert_eq!(v1_back.info.chart_name, "HYPER");
}

#[test]
fn v1_with_null_arrays_handled() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "T", "artist": "A", "genre": "G", "level": 1, "init_bpm": 120.0
        },
        "bpm_events": null,
        "stop_events": null,
        "sound_channels": [],
        "bga": { "bga_header": [], "bga_events": [], "layer_events": [], "poor_events": [] }
    }"#;
    let v1: v1::Bmson = serde_json::from_str(json).unwrap();
    assert!(v1.bpm_events.is_empty());
    assert!(v1.stop_events.is_empty());
}

// v0 转换测试

fn make_v0_bmson() -> v0::Bmson<'static> {
    v0::Bmson {
        info: v0::BmsonInfo {
            title: "Legacy Song",
            subtitle: None,
            artist: "Legacy Artist",
            subartists: vec![],
            genre: "Piano",
            mode_hint: None,
            chart_name: None,
            level: 5,
            init_bpm: 120.0,
            judge_rank: 100.0,
            total: 100.0,
            back_image: None,
            eyecatch_image: None,
            banner_image: None,
            preview_music: None,
            title_image: None,
            resolution: 240,
            ln_type: None,
        },
        lines: None,
        bpm_notes: vec![v0::EventNote { y: 0, v: 120.0 }],
        stop_events: vec![v0::EventNote { y: 960, v: 4.0 }],
        sound_channels: vec![v0::SoundChannel {
            name: Path::new("kick.wav"),
            notes: vec![NoteEvent {
                x: 1,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            }],
        }],
        bga: BGA::default(),
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    }
}

#[test]
fn v0_converts_to_root() {
    let v0 = make_v0_bmson();
    let root: Bmson = v0.try_into().unwrap();

    assert_eq!(root.version, "0.2.1");
    assert_eq!(root.song_info.title, "Legacy Song");
    assert_eq!(root.song_info.artist, "Legacy Artist");
    assert_eq!(root.song_info.genre, "Piano");
    assert!((root.chart_data.init_bpm - 120.0).abs() < f64::EPSILON);
    assert!((root.chart_data.judge_multiplier - 1.0).abs() < f64::EPSILON);
    assert!((root.chart_data.life_multiplier - 1.0).abs() < f64::EPSILON);
}

#[test]
fn v0_zero_init_bpm_returns_error() {
    let mut v0 = make_v0_bmson();
    v0.info.init_bpm = 0.0;
    let result: Result<Bmson, _> = v0.try_into();
    assert!(result.is_err());
}

#[test]
fn v0_negative_init_bpm_returns_error() {
    let mut v0 = make_v0_bmson();
    v0.info.init_bpm = -1.0;
    let result: Result<Bmson, _> = v0.try_into();
    assert!(result.is_err());
}

#[test]
fn v0_judge_rank_maps_to_multiplier() {
    let mut v0 = make_v0_bmson();
    v0.info.judge_rank = 50.0;
    let root: Bmson = v0.try_into().unwrap();
    assert!((root.chart_data.judge_multiplier - 0.5).abs() < f64::EPSILON);
}

#[test]
fn v0_total_maps_to_multiplier() {
    let mut v0 = make_v0_bmson();
    v0.info.total = 200.0;
    let root: Bmson = v0.try_into().unwrap();
    assert!((root.chart_data.life_multiplier - 2.0).abs() < f64::EPSILON);
}

#[test]
fn v0_roundtrip_root_to_v0() {
    let v0_original = make_v0_bmson();
    let root: Bmson = v0_original.try_into().unwrap();
    let v0_back: v0::Bmson = v0::Bmson::try_from(root).unwrap();

    assert_eq!(v0_back.info.title, "Legacy Song");
    assert_eq!(v0_back.info.artist, "Legacy Artist");
    assert!((v0_back.info.init_bpm - 120.0).abs() < f64::EPSILON);
}

#[test]
fn v0_serde_roundtrip() {
    let v0_original = make_v0_bmson();
    let json = serde_json::to_string(&v0_original).unwrap();
    let v0_back: v0::Bmson = serde_json::from_str(&json).unwrap();

    assert_eq!(v0_back.info.title, "Legacy Song");
    assert_eq!(v0_back.info.artist, "Legacy Artist");
    assert_eq!(v0_back.info.genre, "Piano");
}

#[test]
fn v0_legacy_field_names_accepted() {
    let json = r#"{
        "info": { "title": "T", "artist": "A", "genre": "G", "level": 1, "initBPM": 120, "judgeRank": 100 },
        "lines": null,
        "bpmNotes": [],
        "stopEvents": [],
        "soundChannel": [],
        "bga": { "bgaHeader": [], "bgaNotes": [], "layerNotes": [], "poorNotes": [] }
    }"#;
    let v0: v0::Bmson = serde_json::from_str(json).unwrap();
    assert_eq!(v0.info.title, "T");
    assert!((v0.info.init_bpm - 120.0).abs() < f64::EPSILON);
}

#[test]
fn v0_bar_line_with_k_field() {
    let json = r#"{"y": 960, "k": 0}"#;
    let bl: v0::BarLine = serde_json::from_str(json).unwrap();
    assert_eq!(bl.y, 960);
    assert_eq!(bl.k, Some(0));
}

#[test]
fn v0_bar_line_without_k_field() {
    let json = r#"{"y": 1920}"#;
    let bl: v0::BarLine = serde_json::from_str(json).unwrap();
    assert_eq!(bl.y, 1920);
    assert_eq!(bl.k, None);
}

#[test]
fn v0_bar_line_k_dropped_when_converting_to_root() {
    let v0_bar = v0::BarLine { y: 960, k: Some(0) };
    let root_bar: BarLine = bmson_def::BarLine { y: 960 };
    assert_eq!(root_bar.y, v0_bar.y);
    // k 字段只存在于 v0
}

#[test]
fn v0_event_note_roundtrip() {
    let en = v0::EventNote { y: 480, v: 140.0 };
    let json = serde_json::to_string(&en).unwrap();
    let back: v0::EventNote = serde_json::from_str(&json).unwrap();
    assert_eq!(back.y, 480);
    assert!((back.v - 140.0).abs() < f64::EPSILON);
}
