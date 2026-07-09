//! `bmson-processor` 的集成测试。

#![expect(clippy::panic_in_result_fn, reason = "test helpers and assertions")]

use std::num::NonZeroU8;
use std::path::Path;

use bmson_def::{
    BGA, BGAEvent, BGAHeader, BpmEvent, ChartData, ChartInfo, LnType, ModeHint, NoteEvent,
    SongInfo, SoundChannel, StopEvent,
};
use bmson_processor::layout::{Beat, GenericLayout, Pms};
use bmson_processor::{BmsonNoteExt, BmsonProcessor};
use bmsrs_chart::mode::{Lane, NoteSide};
use bmsrs_chart::{BgaLayer, Event, EventKind, LnJudgeHint, LnLifeHint, LnTypeHint, NoteKind};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

/// 构建一个简单的 v2 BMSON 谱面供测试使用。
fn make_simple_bmson() -> bmson_def::Bmson<'static> {
    bmson_def::Bmson {
        version: "2.0.0",
        song_info: SongInfo {
            title: "Test",
            artist: "Test Artist",
            genre: "Test Genre",
        },
        chart_info: ChartInfo {
            subtitle: "",
            subartists: vec![],
            chart_name: "HYPER",
            level: 8,
            back_image: None,
            eyecatch_image: None,
            banner_image: None,
            preview_music: None,
            title_image: None,
            bga: BGA::default(),
        },
        chart_data: ChartData {
            mode_hint: ModeHint::Beat7k,
            ln_type_hint: LnType::Ln,
            ln_judge_hint: bmson_def::LnJudge::Normal,
            ln_life_hint: bmson_def::LnLife::Normal,
            init_bpm: 140.0,
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
    }
}

#[test]
fn process_basic_note() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("kick.wav"),
        note_events: vec![NoteEvent {
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
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let notes: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                side,
                lane,
                kind,
                ext: BmsonNoteExt { .. },
                ..
            } = &e.kind
            {
                Some((e.tick(), *side, *lane, *kind))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0], (0, NoteSide::P1, key(1), NoteKind::Normal));
    Ok(())
}

#[test]
fn process_beat_layout_maps_both_sides() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("p1.wav"),
        note_events: vec![
            NoteEvent {
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
            },
            NoteEvent {
                x: 8,
                y: 240,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            },
            NoteEvent {
                x: 9,
                y: 480,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            },
        ],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let note_positions: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                side,
                lane,
                ext: BmsonNoteExt { .. },
                ..
            } = &e.kind
            {
                Some((*side, *lane))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(note_positions.len(), 3);
    assert!(note_positions.contains(&(NoteSide::P1, key(1))));
    assert!(note_positions.contains(&(NoteSide::P1, Lane::Scratch(nz(1)))));
    assert!(note_positions.contains(&(NoteSide::P2, key(1))));
    Ok(())
}

#[test]
fn process_pms_layout_single_player() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("popn.wav"),
        note_events: (1..=9)
            .map(|i| NoteEvent {
                x: i,
                y: (i - 1) * 240,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            })
            .collect(),
    });

    let chart = BmsonProcessor::process::<Pms>(&bmson)?;

    let sides: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                side,
                ext: BmsonNoteExt { .. },
                ..
            } = &e.kind
            {
                Some(*side)
            } else {
                None
            }
        })
        .collect();

    // PMS 模式：所有音符均为 Player 1
    assert_eq!(sides.len(), 9);
    assert!(sides.iter().all(|s| *s == NoteSide::P1));
    Ok(())
}

#[test]
fn process_generic_layout_n_keys() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("generic.wav"),
        note_events: (1..=5)
            .map(|i| NoteEvent {
                x: i,
                y: (i - 1) * 240,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            })
            .collect(),
    });

    let chart = BmsonProcessor::process_nkeys(&bmson, 5)?;

    let positions: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                side,
                lane,
                ext: BmsonNoteExt { .. },
                ..
            } = &e.kind
            {
                Some((*side, *lane))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(positions.len(), 5);
    assert!(positions.contains(&(NoteSide::P1, key(1))));
    assert!(positions.contains(&(NoteSide::P1, key(5))));
    Ok(())
}

#[test]
fn process_default_routes_by_mode_hint() -> TestResult {
    // Beat
    let mut bmson_beat = make_simple_bmson();
    bmson_beat.chart_data.mode_hint = ModeHint::Beat7k;
    bmson_beat.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("a.wav"),
        note_events: vec![NoteEvent {
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
    });
    let chart_beat = BmsonProcessor::process_default(&bmson_beat)?;
    assert!(chart_beat.data.events.iter().any(|e| matches!(
        &e.kind,
        EventKind::Note {
            ext: BmsonNoteExt { .. },
            ..
        }
    )));

    // PMS
    let mut bmson_pms = make_simple_bmson();
    bmson_pms.chart_data.mode_hint = ModeHint::Popn9k;
    bmson_pms.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("b.wav"),
        note_events: vec![NoteEvent {
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
    });
    let chart_pms = BmsonProcessor::process_default(&bmson_pms)?;
    assert!(chart_pms.data.events.iter().any(|e| matches!(
        &e.kind,
        EventKind::Note {
            ext: BmsonNoteExt { .. },
            ..
        }
    )));

    Ok(())
}

#[test]
fn process_bpm_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson
        .chart_data
        .bpm_events
        .push(BpmEvent { y: 960, bpm: 180.0 });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let bpm_evts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Bpm { bpm } = &e.kind {
                Some((e.tick(), *bpm))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(bpm_evts.len(), 1);
    assert_eq!(bpm_evts[0], (960, 180.0));
    Ok(())
}

#[test]
fn process_stop_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.stop_events.push(StopEvent {
        y: 480,
        duration: 192,
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let stop_evts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Stop { duration } = &e.kind {
                Some((e.tick(), *duration))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(stop_evts.len(), 1);
    assert_eq!(stop_evts[0], (480, 192));
    Ok(())
}

#[test]
fn process_scroll_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson
        .scroll_events
        .push(bmson_def::ScrollEvent { y: 1920, rate: 2.0 });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let scroll_evts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Scroll { rate } = &e.kind {
                Some((e.tick(), *rate))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(scroll_evts.len(), 1);
    assert_eq!(scroll_evts[0], (1920, 2.0));
    Ok(())
}

#[test]
fn process_bga_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_info.bga = BGA {
        bga_header: vec![BGAHeader {
            id: 1,
            name: Path::new("bg.png"),
        }],
        bga_events: vec![BGAEvent { y: 0, id: 1 }],
        layer_events: vec![BGAEvent { y: 480, id: 1 }],
        poor_events: vec![BGAEvent { y: 960, id: 1 }],
    };

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let bga_evts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Bga { layer, .. } = &e.kind {
                Some((e.tick(), *layer))
            } else {
                None
            }
        })
        .collect();

    assert!(bga_evts.contains(&(0, BgaLayer::Base)));
    assert!(bga_evts.contains(&(480, BgaLayer::Layer)));
    assert!(bga_evts.contains(&(960, BgaLayer::Poor)));

    assert_eq!(chart.chart.bga_resources.len(), 1);
    assert_eq!(chart.chart.bga_resources[0].id, 1);
    Ok(())
}

#[test]
fn process_long_note() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("ln.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 480,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let lns: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Long { duration },
                ext: BmsonNoteExt { .. },
                ..
            } = &e.kind
            {
                Some(*duration)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(lns, vec![480]);
    Ok(())
}

#[test]
fn process_bgm_note_skips_playable_pulse() -> TestResult {
    // BGM 音符（x=0）在与可玩音符相同脉冲时被丢弃。
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: Path::new("playable.wav"),
        note_events: vec![
            NoteEvent {
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
            },
            NoteEvent {
                x: 0,
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
            },
        ],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let event_count = chart
        .data
        .events
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Note {
                    ext: BmsonNoteExt { .. },
                    ..
                }
            )
        })
        .count();

    // 只应有 1 个可玩音符（BGM 被丢弃）。
    assert_eq!(event_count, 1);
    Ok(())
}

#[test]
fn process_zero_bpm_returns_error() {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.init_bpm = 0.0;
    let result = BmsonProcessor::process::<Beat>(&bmson);
    assert!(result.is_err());
}

#[test]
fn process_nan_bpm_returns_error() {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.init_bpm = f64::NAN;
    let result = BmsonProcessor::process::<Beat>(&bmson);
    assert!(result.is_err());
}

#[test]
fn metadata_transferred() -> TestResult {
    let bmson = make_simple_bmson();
    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    assert_eq!(chart.song.title, "Test");
    assert_eq!(chart.song.artist, "Test Artist");
    assert_eq!(chart.song.genre, "Test Genre");
    assert_eq!(chart.chart.chart_name, "HYPER");
    assert_eq!(chart.chart.level, 8);
    Ok(())
}

#[test]
fn bar_lines_auto_generated() -> TestResult {
    let bmson = make_simple_bmson();
    let chart = BmsonProcessor::process::<Beat>(&bmson)?;

    let bars: Vec<_> = chart
        .data
        .events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Bar))
        .map(Event::tick)
        .collect();

    assert!(!bars.is_empty());
    assert_eq!(bars[0], 0);
    assert_eq!(bars[1], 960);
    Ok(())
}

#[test]
fn beat_layout_mapping() {
    // 测试 Beat 布局的 x → (side, lane) 映射
    assert_eq!(Beat::from_bmson(1), Some((NoteSide::P1, key(1))));
    assert_eq!(Beat::from_bmson(7), Some((NoteSide::P1, key(7))));
    assert_eq!(
        Beat::from_bmson(8),
        Some((NoteSide::P1, Lane::Scratch(nz(1))))
    );
    assert_eq!(Beat::from_bmson(9), Some((NoteSide::P2, key(1))));
    assert_eq!(Beat::from_bmson(15), Some((NoteSide::P2, key(7))));
    assert_eq!(
        Beat::from_bmson(16),
        Some((NoteSide::P2, Lane::Scratch(nz(1))))
    );
    assert_eq!(Beat::from_bmson(0), None);
    assert_eq!(Beat::from_bmson(17), None);
}

#[test]
fn pms_layout_mapping() {
    assert_eq!(Pms::from_bmson(1), Some((NoteSide::P1, key(1))));
    assert_eq!(Pms::from_bmson(9), Some((NoteSide::P1, key(9))));
    assert_eq!(Pms::from_bmson(0), None);
    assert_eq!(Pms::from_bmson(10), None);
}

#[test]
fn generic_layout_mapping() {
    let layout = GenericLayout { keys: 5 };
    assert_eq!(layout.map_x(1), Some((NoteSide::P1, key(1))));
    assert_eq!(layout.map_x(5), Some((NoteSide::P1, key(5))));
    assert_eq!(layout.map_x(0), None);
    assert_eq!(layout.map_x(6), None);
}

#[test]
fn bar_lines_from_explicit_lines() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.lines = Some(vec![
        bmson_def::BarLine { y: 0 },
        bmson_def::BarLine { y: 960 },
        bmson_def::BarLine { y: 1920 },
    ]);

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;
    let bars: Vec<_> = chart
        .data
        .events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Bar))
        .map(Event::tick)
        .collect();

    assert_eq!(bars, vec![0, 960, 1920]);
    Ok(())
}

#[test]
fn note_ext_fields() {
    let ext = BmsonNoteExt {
        vol: Some(-50),
        pan: Some(100),
        release_sound: Some(true),
        ln_mode: Some(2),
        ln_type_hint: Some(LnTypeHint::Cn),
        ln_judge_hint: Some(LnJudgeHint::Ticks),
        ln_life_hint: Some(LnLifeHint::Normal),
    };
    assert_eq!(ext.vol, Some(-50));
    assert_eq!(ext.pan, Some(100));
    assert_eq!(ext.release_sound, Some(true));
}

#[test]
fn chart_level_ln_hints_propagate() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.ln_type_hint = bmson_def::LnType::Cn;
    bmson.chart_data.ln_judge_hint = bmson_def::LnJudge::Ticks;
    bmson.chart_data.ln_life_hint = bmson_def::LnLife::Ticks;

    let chart = BmsonProcessor::process::<Beat>(&bmson)?;
    assert_eq!(chart.data.ln_type_hint, LnTypeHint::Cn);
    assert_eq!(chart.data.ln_judge_hint, LnJudgeHint::Ticks);
    assert_eq!(chart.data.ln_life_hint, LnLifeHint::Ticks);
    Ok(())
}
