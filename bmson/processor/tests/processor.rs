#![expect(missing_docs, reason = "integration test")]

use bmson_def::{
    BGA, BGAEvent, BGAHeader, BarLine, Bmson, ChartData, ChartInfo, MineChannel, ModeHint,
    NoteEvent, ScrollEvent, SongInfo, SoundChannel,
};
use bmson_processor::layout::*;
use bmson_processor::{BmsonNoteExt, BmsonProcessor};
use bmsrs_chart::{BgaLayer, Event, Lane, NoteKind, NoteSide};
use std::num::NonZeroU8;
use std::path::Path;

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

const fn sc(n: u8) -> Lane {
    Lane::Scratch(nz(n))
}

const TITLE: &str = "Test Song";
const ARTIST: &str = "Test Artist";
const GENRE: &str = "Test Genre";

const fn make_bmson(sound_channels: Vec<SoundChannel<'_>>) -> Bmson<'_> {
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
            ln_type_hint: bmson_def::LnType::Ln,
            ln_judge_hint: bmson_def::LnJudge::Normal,
            ln_life_hint: bmson_def::LnLife::Normal,
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

const fn ne(x: u64, y: u64, l: u64) -> NoteEvent {
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

fn notes(chart: &bmsrs_chart::Chart<BmsonNoteExt>) -> Vec<(u64, Lane, NoteKind)> {
    chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Note {
                tick, lane, kind, ..
            } = e
            {
                Some((*tick, *lane, *kind))
            } else {
                None
            }
        })
        .collect()
}

fn bgm_events(chart: &bmsrs_chart::Chart<BmsonNoteExt>) -> Vec<(u64, u32)> {
    chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Bgm { tick, audio_index } = e {
                Some((*tick, *audio_index))
            } else {
                None
            }
        })
        .collect()
}

fn bar_lines(chart: &bmsrs_chart::Chart<BmsonNoteExt>) -> Vec<u64> {
    chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Bar { tick } = e {
                Some(*tick)
            } else {
                None
            }
        })
        .collect()
}

fn scroll_events(chart: &bmsrs_chart::Chart<BmsonNoteExt>) -> Vec<(u64, f64)> {
    chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Scroll { tick, rate } = e {
                Some((*tick, *rate))
            } else {
                None
            }
        })
        .collect()
}

fn bga_events(chart: &bmsrs_chart::Chart<BmsonNoteExt>, layer: BgaLayer) -> Vec<(u64, u32)> {
    chart
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Bga {
                tick,
                layer: l,
                resource_id,
            } = e
            {
                (*l == layer).then_some((*tick, *resource_id))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn bme_bmson_x_aligns_scratch_with_keys() {
    assert_eq!(Beat::map_x(1), Some((NoteSide::P1, key(1))));
    assert_eq!(Beat::map_x(5), Some((NoteSide::P1, key(5))));
    assert_eq!(Beat::map_x(6), Some((NoteSide::P1, key(6))));
    assert_eq!(Beat::map_x(8), Some((NoteSide::P1, sc(1))));
    assert_eq!(Beat::map_x(7), Some((NoteSide::P1, key(7))));
    assert_eq!(Beat::map_x(0), None);
}

#[test]
fn pms_bmson_popn_9k_maps_nine_keys() {
    assert_eq!(Pms::map_x(1), Some((NoteSide::P1, key(1))));
    assert_eq!(Pms::map_x(9), Some((NoteSide::P1, key(9))));
    assert_eq!(Pms::map_x(10), None);
}

#[test]
fn generic_maps_by_keys() {
    let layout = GenericLayout { keys: 4 };
    assert_eq!(layout.map_x(1), Some((NoteSide::P1, key(1))));
    assert_eq!(layout.map_x(4), Some((NoteSide::P1, key(4))));
    assert_eq!(layout.map_x(5), None);
}

#[test]
fn bgm_discarded_when_playable_at_same_pulse() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 240, 0), ne(0, 240, 0)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    assert_eq!(bgm_events(&chart).len(), 0);
    assert_eq!(notes(&chart).len(), 1);
}

#[test]
fn bgm_kept_when_no_playable_at_same_pulse() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0), ne(0, 240, 0)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    assert_eq!(bgm_events(&chart).len(), 1);
    assert_eq!(bgm_events(&chart)[0], (240, 1));
}

#[test]
fn process_basic_chart() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0), ne(2, 240, 0), ne(1, 480, 0)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    let ns = notes(&chart);
    assert_eq!(ns.len(), 3);
    assert_eq!(ns[0], (0, key(1), NoteKind::Normal));
    assert_eq!(ns[1], (240, key(2), NoteKind::Normal));
    assert_eq!(ns[2], (480, key(1), NoteKind::Normal));
    assert_eq!(chart.audio_assets.len(), 3);
    assert_eq!(chart.audio_assets[0].path, Path::new("demo.wav"));
}

#[test]
fn process_long_note() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 480)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    let ns = notes(&chart);
    assert_eq!(ns.len(), 1);
    assert_eq!(ns[0].2, NoteKind::Long { duration: 480 });
}

#[test]
fn process_short_note_is_normal() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    assert_eq!(notes(&chart)[0].2, NoteKind::Normal);
}

#[test]
fn process_invalid_bpm_returns_error() {
    let mut bmson = make_bmson(vec![]);
    bmson.chart_data.init_bpm = 0.0;

    let result = BmsonProcessor::process::<Beat>(&bmson);
    assert!(result.is_err());
}

#[test]
fn process_default_beat_hint_uses_bme() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(8, 0, 0)],
    }]);

    let chart = BmsonProcessor::process_default(&bmson).expect("processing succeeds");

    let ns = notes(&chart);
    assert_eq!(ns.len(), 1);
    assert_eq!(ns[0].1, sc(1));
}

#[test]
fn process_default_popn_hint_uses_pms() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(9, 0, 0)],
    }]);
    bmson.chart_data.mode_hint = ModeHint::Popn9k;

    let chart = BmsonProcessor::process_default(&bmson).expect("processing succeeds");

    let ns = notes(&chart);
    assert_eq!(ns[0].1, key(9));
}

#[test]
fn process_default_generic_hint_uses_generic() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(3, 0, 0)],
    }]);
    bmson.chart_data.mode_hint = ModeHint::Generic(4);

    let chart = BmsonProcessor::process_default(&bmson).expect("processing succeeds");

    let ns = notes(&chart);
    assert_eq!(ns[0].1, key(3));
}

#[test]
fn process_generates_auto_bar_lines() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 960, 0)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    let bars = bar_lines(&chart);
    assert!(!bars.is_empty());
    assert_eq!(bars[0], 0);
}

#[test]
fn process_explicit_bar_lines() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);
    bmson.chart_data.lines = Some(vec![BarLine { y: 0 }, BarLine { y: 100 }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    let bars = bar_lines(&chart);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0], 0);
    assert_eq!(bars[1], 100);
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

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    assert_eq!(chart.bga_resources.len(), 1);
    assert_eq!(chart.bga_resources[0].path, Path::new("bg.png"));

    let base_bga = bga_events(&chart, BgaLayer::Base);
    assert_eq!(base_bga.len(), 1);
    assert_eq!(base_bga[0], (0, 1));
}

#[test]
fn process_mine_channel() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);
    bmson.mine_channels = vec![MineChannel {
        name: Path::new("mine.wav"),
        notes: vec![bmson_def::MineNote {
            x: 1,
            y: 480,
            damage: 0.5,
        }],
    }];

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    let ns = notes(&chart);
    let mine = ns
        .iter()
        .find(|(t, _, _)| *t == 480)
        .expect("mine note exists");
    assert_eq!(mine.2, NoteKind::Mine { damage: 0.5 });
}

#[test]
fn process_scroll_events() {
    let mut bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);
    bmson.scroll_events = vec![ScrollEvent { y: 240, rate: 2.0 }];

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    let sc = scroll_events(&chart);
    assert_eq!(sc.len(), 1);
    assert_eq!(sc[0].0, 240);
    assert!((sc[0].1 - 2.0).abs() < 1e-9);
}

#[test]
fn process_metadata() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 0, 0)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    assert_eq!(chart.metadata.title, TITLE);
    assert_eq!(chart.metadata.artist, ARTIST);
    assert_eq!(chart.metadata.genre, GENRE);
    assert_eq!(chart.metadata.chart_name, "NORMAL");
    assert_eq!(chart.metadata.level, 5);
}

#[test]
fn events_sorted_by_tick() {
    let bmson = make_bmson(vec![SoundChannel {
        name: Path::new("demo.wav"),
        note_events: vec![ne(1, 480, 0), ne(2, 0, 0), ne(3, 240, 0)],
    }]);

    let chart = BmsonProcessor::process::<Beat>(&bmson).expect("processing succeeds");

    let ticks: Vec<u64> = chart.events.iter().map(Event::tick).collect();
    assert!(ticks.windows(2).all(|w| w[0] <= w[1]));
}
