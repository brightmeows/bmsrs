#![expect(missing_docs, reason = "integration test")]

use bmson_def::{
    BGA, BGAEvent, BGAHeader, BarLine, Bmson, ChartData, ChartInfo, LnJudge, LnLife, LnType,
    MineChannel, MineNote, ModeHint, NoteEvent, ScrollEvent, SongInfo, SoundChannel,
};
use bmson_processor::{BmsonMapping, BmsonProcessor};
use bmsrs_chart::{Beat5k, Beat7k, Beat10k, Beat14k, GenericLayout, NoteData, NoteKind, Popn9k};
use std::path::Path;

const TITLE: &str = "Test Song";
const ARTIST: &str = "Test Artist";
const GENRE: &str = "Test Genre";

fn make_bmson(sound_channels: Vec<SoundChannel<'_>>) -> Bmson<'_> {
    Bmson {
        version: "2.0.0",
        song_info: SongInfo {
            title: TITLE,
            artist: ARTIST,
            genre: GENRE,
        },
        chart_info: ChartInfo {
            subtitle: "",
            subartists: vec![],
            chart_name: "NORMAL",
            level: 5,
            back_image: None,
            eyecatch_image: None,
            banner_image: None,
            preview_music: None,
            title_image: None,
            bga: BGA {
                bga_header: vec![],
                bga_events: vec![],
                layer_events: vec![],
                poor_events: vec![],
            },
        },
        chart_data: ChartData {
            mode_hint: ModeHint::Beat7k,
            ln_type_hint: LnType::Ln,
            ln_judge_hint: LnJudge::Normal,
            ln_life_hint: LnLife::Normal,
            init_bpm: 120.0,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            resolution: 240,
            lines: None,
            bpm_events: vec![],
            stop_events: vec![],
            sound_channels,
            judge_deltas: None,
            life_deltas: None,
        },
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    }
}

fn ne(x: u64, y: u64, l: u64) -> NoteEvent {
    NoteEvent {
        x,
        y,
        l,
        c: false,
        t: None,
        up: None,
        ln_type_hint: None,
        ln_judge_hint: None,
        ln_life_hint: None,
        vol: None,
        pan: None,
    }
}

#[test]
fn beat7k_maps_keys_and_scratch() {
    assert_eq!(Beat7k.map_x(1), Some(0));
    assert_eq!(Beat7k.map_x(7), Some(6));
    assert_eq!(Beat7k.map_x(8), Some(7));
    assert_eq!(Beat7k.map_x(9), None);
    assert_eq!(Beat7k.map_x(0), None);
}

#[test]
fn beat5k_maps_keys_and_scratch() {
    assert_eq!(Beat5k.map_x(1), Some(0));
    assert_eq!(Beat5k.map_x(5), Some(4));
    assert_eq!(Beat5k.map_x(8), Some(5));
    assert_eq!(Beat5k.map_x(6), None);
}

#[test]
fn beat14k_maps_all_sixteen_channels() {
    assert_eq!(Beat14k.map_x(1), Some(0));
    assert_eq!(Beat14k.map_x(8), Some(7));
    assert_eq!(Beat14k.map_x(9), Some(8));
    assert_eq!(Beat14k.map_x(16), Some(15));
    assert_eq!(Beat14k.map_x(17), None);
}

#[test]
fn beat10k_maps_split_layout() {
    assert_eq!(Beat10k.map_x(1), Some(0));
    assert_eq!(Beat10k.map_x(5), Some(4));
    assert_eq!(Beat10k.map_x(8), Some(5));
    assert_eq!(Beat10k.map_x(9), Some(6));
    assert_eq!(Beat10k.map_x(13), Some(10));
    assert_eq!(Beat10k.map_x(16), Some(11));
    assert_eq!(Beat10k.map_x(6), None);
}

#[test]
fn popn9k_maps_nine_keys() {
    assert_eq!(Popn9k.map_x(1), Some(0));
    assert_eq!(Popn9k.map_x(9), Some(8));
    assert_eq!(Popn9k.map_x(10), None);
}

#[test]
fn generic_layout_maps_by_keys() {
    let layout = GenericLayout { keys: 4 };
    assert_eq!(layout.map_x(1), Some(0));
    assert_eq!(layout.map_x(4), Some(3));
    assert_eq!(layout.map_x(5), None);
}

#[test]
fn short_note_is_normal() {
    let note = ne(1, 0, 0);
    let data = Beat7k.make_note_data(&note, 0);
    assert_eq!(data.kind, NoteKind::Normal);
}

#[test]
fn long_note_preserves_duration() {
    let note = ne(1, 0, 480);
    let data = Beat7k.make_note_data(&note, 0);
    assert_eq!(data.kind, NoteKind::Long { duration: 480 });
}

#[test]
fn mine_data_has_damage() {
    let data = Beat7k.make_mine_data(3, 0.5);
    assert_eq!(data.kind, NoteKind::Mine { damage: 0.5 });
}

#[test]
fn invisible_data_is_invisible() {
    let data = Beat7k.make_invisible_data(2);
    assert_eq!(data.kind, NoteKind::Invisible);
}

#[test]
fn bgm_discarded_when_playable_at_same_pulse() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![
            ne(1, 240, 0), // playable at pulse 240
            ne(0, 240, 0), // BGM at same pulse -> discarded
        ],
    }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.bgm.len(), 0);
    assert_eq!(chart.notes.len(), 1);
}

#[test]
fn bgm_kept_when_no_playable_at_same_pulse() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![
            ne(1, 0, 0),   // playable at pulse 0
            ne(0, 240, 0), // BGM at pulse 240 -> kept
        ],
    }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.bgm.len(), 1);
    assert_eq!(chart.bgm[0].tick, 240);
}

#[test]
fn process_basic_chart() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0), ne(2, 240, 0), ne(1, 480, 0)],
    }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.notes.len(), 3);
    assert_eq!(chart.notes[0].tick, 0);
    assert_eq!(chart.notes[0].data.lane(), 0);
    assert_eq!(chart.notes[1].tick, 240);
    assert_eq!(chart.notes[1].data.lane(), 1);
    assert_eq!(chart.notes[2].tick, 480);
    assert_eq!(chart.notes[2].data.lane(), 0);
    assert_eq!(chart.audio_assets.len(), 3);
    assert_eq!(chart.audio_assets[0].path, Path::new("demo.wav"));
}

#[test]
fn process_long_note() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 480)],
    }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.notes.len(), 1);
    assert_eq!(chart.notes[0].data.kind, NoteKind::Long { duration: 480 });
}

#[test]
fn process_invalid_bpm_returns_error() {
    let mut bmson = make_bmson(vec![]);
    bmson.chart_data.init_bpm = 0.0;

    let result = BmsonProcessor::process(&bmson, &Beat7k);
    assert!(result.is_err());
}

#[test]
fn process_default_uses_beat7k() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);

    let chart = BmsonProcessor::process_default(&bmson).expect("processing succeeds");

    assert_eq!(chart.lane_count, 8);
    assert_eq!(chart.notes.len(), 1);
}

#[test]
fn process_generates_auto_bar_lines() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 960, 0)],
    }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert!(!chart.bar_lines.is_empty());
    assert_eq!(chart.bar_lines[0].tick, 0);
}

#[test]
fn process_explicit_bar_lines() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);
    bmson.chart_data.lines = Some(vec![BarLine { y: 0 }, BarLine { y: 100 }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.bar_lines.len(), 2);
    assert_eq!(chart.bar_lines[0].tick, 0);
    assert_eq!(chart.bar_lines[1].tick, 100);
}

#[test]
fn process_bga_events() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);
    bmson.chart_info.bga = BGA {
        bga_header: vec![BGAHeader {
            id: 1,
            name: Path::new("bg.png"),
        }],
        bga_events: vec![BGAEvent { y: 0, id: 1 }],
        layer_events: vec![],
        poor_events: vec![],
    };

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.bga.resources.len(), 1);
    assert_eq!(chart.bga.resources[0].path, Path::new("bg.png"));
    assert_eq!(chart.bga.events.len(), 1);
    assert_eq!(chart.bga.events[0].tick, 0);
    assert_eq!(chart.bga.events[0].resource_id, 1);
}

#[test]
fn process_mine_channel() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);
    bmson.mine_channels = vec![MineChannel {
        name: Path::new("mine.wav"),
        notes: vec![MineNote {
            x: 1,
            y: 480,
            damage: 0.5,
        }],
    }];

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    let mine_note = chart
        .notes
        .iter()
        .find(|n| n.tick == 480)
        .expect("mine note exists");
    assert_eq!(mine_note.data.kind, NoteKind::Mine { damage: 0.5 });
}

#[test]
fn process_scroll_events() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);
    bmson.scroll_events = vec![ScrollEvent { y: 240, rate: 2.0 }];

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.scroll_events.len(), 1);
    assert_eq!(chart.scroll_events[0].tick, 240);
    assert!((chart.scroll_events[0].rate - 2.0).abs() < 1e-9);
}

#[test]
fn process_metadata() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.metadata.title, TITLE);
    assert_eq!(chart.metadata.artist, ARTIST);
    assert_eq!(chart.metadata.genre, GENRE);
    assert_eq!(chart.metadata.chart_name, "NORMAL");
    assert_eq!(chart.metadata.level, 5);
}

#[test]
fn notes_sorted_by_tick() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 480, 0), ne(2, 0, 0), ne(3, 240, 0)],
    }]);

    let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

    assert_eq!(chart.notes[0].tick, 0);
    assert_eq!(chart.notes[1].tick, 240);
    assert_eq!(chart.notes[2].tick, 480);
}
