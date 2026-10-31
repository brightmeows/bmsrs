//! Integration tests for `bmsrs-player`.
#![expect(clippy::unwrap_used, reason = "test code")]

use std::num::NonZeroU8;
use std::path::PathBuf;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, AudioIndex, BgaLayer, BgaResource, BgaResourceId, BpmChange, Chart, ChartData,
    ChartInfo, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint, NoteAudio, NoteKind,
    NoteRole, NoteSide, SongInfo, StopEvent, TimingTrack, TimingTrackError,
};
use bmsrs_player::Player;

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

/// Create a simple test chart.
fn make_test_chart() -> Chart {
    // Events must be sorted ascending by (tick, priority); otherwise the
    // Player's partition_point cannot work correctly.
    Chart {
        song: SongInfo {
            title: "Test".into(),
            ..Default::default()
        },
        chart: ChartInfo::default(),
        data: ChartData {
            timing: TimingTrack::new(
                120.0,
                vec![BpmChange {
                    tick: 960,
                    bpm: 180.0,
                }],
                vec![],
                240,
            )
            .unwrap(),
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![
                Event::bar(0),
                Event::bgm(240, AudioIndex::new(0)),
                Event::new(
                    480,
                    EventKind::Note {
                        side: NoteSide::P1,
                        lane: key(1),
                        kind: NoteKind::Normal,
                        role: NoteRole::Normal,
                        audio: NoteAudio::none(),
                        ext: (),
                    },
                ),
                Event::bar(960),
                Event::bpm(960, 180.0),
            ],
            judge_deltas: None,
            life_deltas: None,
            // validate checks audio index integrity: the BGM event references
            // index 0, so the asset table must be non-empty.
            audio_assets: vec![AudioAsset::from_path_buf(
                PathBuf::from("test.wav"),
                Duration::ZERO,
                None,
            )],
        },
    }
}

// Player creation and basic operations

#[test]
fn player_new_starts_at_zero() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();
    assert_eq!(player.current_tick(), 0);
    assert!(player.current_time().is_zero());
}

#[test]
fn player_advance_moves_time_and_tick() {
    let chart = make_test_chart();
    let mut player = Player::new(chart).unwrap();
    player.advance(Duration::from_secs(1));
    // 120 BPM, resolution 240: 1 second = 480 ticks
    assert_eq!(player.current_tick(), 480);
}

#[test]
fn player_current_time() {
    let chart = make_test_chart();
    let mut player = Player::new(chart).unwrap();
    player.advance(Duration::from_secs_f64(2.0));
    assert!((player.current_time().as_secs_f64() - 2.0).abs() < 1e-9);
}

#[test]
fn player_seek() {
    let chart = make_test_chart();
    let mut player = Player::new(chart).unwrap();

    player.seek(Duration::from_secs_f64(1.0));
    assert_eq!(player.current_tick(), 480);
    assert!((player.current_time().as_secs_f64() - 1.0).abs() < 1e-6);
}

#[test]
fn player_reset() {
    let chart = make_test_chart();
    let mut player = Player::new(chart).unwrap();
    player.advance(Duration::from_secs(2));
    assert!(player.current_tick() > 0);

    player.reset();
    assert_eq!(player.current_tick(), 0);
    assert!(player.current_time().is_zero());
}

#[test]
fn player_duration() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();
    // With events up to tick 960 at 120 BPM, duration = 2 seconds
    assert!((player.duration().as_secs_f64() - 2.0).abs() < f64::EPSILON);
}

#[test]
fn player_current_bpm_initial() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();
    assert!((player.current_bpm() - 120.0).abs() < 1e-9);
}

#[test]
fn player_current_bpm_before_change() {
    let chart = make_test_chart();
    let mut player = Player::new(chart).unwrap();
    // Seek to a known tick before the BPM change at 960
    player.seek(Duration::from_secs_f64(1.5));
    assert!(
        (player.current_bpm() - 120.0).abs() < 1e-9,
        "BPM should be 120 before change, tick={}",
        player.current_tick()
    );
}

#[test]
fn player_current_bpm_at_change_tick() {
    let chart = make_test_chart();
    let mut player = Player::new(chart).unwrap();
    // Seek to exactly the BPM change tick. At tick 960, BPM should be 180.
    player.seek(Duration::from_secs_f64(2.0));
    let tick = player.current_tick();
    let bpm = player.current_bpm();
    // The BPM change at tick 960 sets BPM to 180.
    // At tick 960, the new BPM is in effect.
    assert!(
        (bpm - 180.0).abs() < 1e-6,
        "expected BPM≈180.0 at tick {tick}, got {bpm}"
    );
}

// Event queries

#[test]
fn player_events_in_range() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let events = player.events_in_range(0..=480);
    // Should contain: Bar(0), Bgm(240), Note(480), Bar(960 is outside range)
    // Actually range is 0..=480 so tick 960 is not included
    let ticks: Vec<u64> = events.iter().map(Event::tick).collect();
    assert_eq!(ticks, vec![0, 240, 480]);
}

#[test]
fn player_events_unbounded_range() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let all_events = player.events_in_range(..);
    assert_eq!(all_events.len(), 5);
}

#[test]
fn player_notes_in_range() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let notes: Vec<_> = player
        .notes_in_range(..)
        .filter_map(|e| {
            if let EventKind::Note { .. } = &e.kind {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes, vec![480]);
}

#[test]
fn player_notes_for_judgement() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    // The note in the test chart is Normal → should be counted
    assert_eq!(player.notes_for_judgement(..).count(), 1);
}

#[test]
fn player_bga_events_empty() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let bga = player
        .events_in_range(..)
        .iter()
        .find(|e| matches!(e.kind, EventKind::Bga { .. }));
    assert!(bga.is_none());
}

// Scroll cache

#[test]
fn player_scroll_rate_default() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let rate = player.scroll_rate_at(0);
    assert!((rate - 1.0).abs() < f64::EPSILON);
}

#[test]
fn player_scroll_rate_with_events() {
    let mut chart = make_test_chart();
    chart.data.events.push(Event::scroll(480, 2.0));
    chart.data.sort_events();
    let player = Player::new(chart).unwrap();

    // Before scroll event
    assert!((player.scroll_rate_at(0) - 1.0).abs() < f64::EPSILON);
    // After scroll event
    assert!((player.scroll_rate_at(480) - 2.0).abs() < f64::EPSILON);
    // At event
    assert!((player.scroll_rate_at(500) - 2.0).abs() < f64::EPSILON);
}

// Timeline queries

#[test]
fn player_tick_to_duration() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let dur = player.tick_to_duration(240);
    assert!((dur.as_secs_f64() - 0.5).abs() < 1e-9);
}

#[test]
fn player_duration_to_tick() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let tick = player.duration_to_tick(Duration::from_secs_f64(0.5));
    assert_eq!(tick, 240);
}

// Chart access

#[test]
fn player_chart_ref() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let chart_ref = player.chart();
    assert_eq!(chart_ref.song.title, "Test");
}

#[test]
fn player_into_chart() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let chart_back = player.into_chart();
    assert_eq!(chart_back.song.title, "Test");
}

#[test]
fn player_audio_assets() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    // The fixture has one audio asset (validate requires BGM references to
    // resolve).
    let assets = player.audio_assets();
    assert_eq!(assets.len(), 1);
}

#[test]
fn player_bga_resources() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    let resources = player.bga_resources();
    assert_eq!(resources, []);
}

// Larger-range tests with BPM changes

#[test]
fn player_advance_with_bpm_change() {
    let mut chart = make_test_chart();
    chart.data.timing = TimingTrack::new(
        120.0,
        vec![BpmChange {
            tick: 240,
            bpm: 240.0,
        }],
        vec![],
        240,
    )
    .unwrap();
    let mut player = Player::new(chart).unwrap();

    // 0-240 ticks at 120 BPM = 0.5s
    player.advance(Duration::from_secs_f64(0.5));
    assert_eq!(player.current_tick(), 240);

    // 240-480 ticks at 240 BPM = 0.25s
    player.advance(Duration::from_secs_f64(0.25));
    assert_eq!(player.current_tick(), 480);
}

// Tests with stop events

#[test]
fn player_advance_with_stop() {
    let mut chart = make_test_chart();
    chart.data.timing = TimingTrack::new(
        120.0,
        vec![],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
        240,
    )
    .unwrap();
    let mut player = Player::new(chart).unwrap();

    // 0 to 0.5s (240 ticks at 120 BPM): should be at tick 240
    player.advance(Duration::from_secs_f64(0.5));
    assert_eq!(player.current_tick(), 240);

    // Advance 1.2s total (0.7 more): the stop at tick 240 consumes 0.5s
    // of real time (240 ticks / 240 res * 60 / 120 BPM = 0.5s).
    // After the stop, remaining 0.2s advances 96 more ticks.
    // Total tick = 240 + 96 = 336.
    player.advance(Duration::from_secs_f64(0.7));
    assert_eq!(player.current_tick(), 336);
}

#[test]
fn frame_wise_advance_crosses_stop() {
    let mut chart = make_test_chart();
    chart.data.timing = TimingTrack::new(
        120.0,
        vec![],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
        240,
    )
    .unwrap();
    let mut player = Player::new(chart).unwrap();

    // A 3-second 60fps frame loop: 180 frames at 1/60s. The frozen segment is
    // [0.5, 1.0)s, so ticks should reach (3.0 - 1.0)s * 480 ticks/s + 240 = 1200;
    // with ticks as state and time derived from ticks, it would stay stuck at
    // 240 forever.
    for _ in 0..180 {
        player.advance(Duration::from_secs_f64(1.0 / 60.0));
    }
    assert!(player.current_tick() > 1000);
}

// visible_tick_range

#[test]
fn visible_tick_range_at_start_covers_negative_reaction() {
    let chart = make_test_chart();
    let player = Player::new(chart).unwrap();

    // At tick 0, a 1s reaction clamped to 0.
    let (start, end) = player.visible_tick_range(Duration::from_secs(1), Duration::from_secs(2));
    assert_eq!(start, 0);
    // 120 BPM, resolution 240: 2s = 4 beats = 960 ticks
    assert_eq!(end, 960);
}

#[test]
fn visible_tick_range_at_midpoint_returns_symmetric_window() {
    let chart = make_test_chart();
    let mut player = Player::new(chart).unwrap();

    // Advance to 1s (in the 120 BPM segment: 1s = 2 beats = 480 ticks).
    player.advance(Duration::from_secs_f64(1.0));
    assert_eq!(player.current_tick(), 480);

    let (start, end) =
        player.visible_tick_range(Duration::from_millis(500), Duration::from_millis(500));
    // 0.5s before is tick 240, 0.5s after is tick 720 (still in the 120 BPM
    // segment).
    assert_eq!(start, 240);
    assert_eq!(end, 720);
}

#[test]
fn player_new_accepts_negative_bpm() {
    // Negative BPM (reverse-scroll charts) must pass through the whole chain:
    // TimingTrack, ChartData::validate, Player::new. If any validation point
    // still uses <=0.0, this panics.
    let chart: Chart = Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            timing: TimingTrack::simple(-120.0, 240).unwrap(),
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: vec![Event::bar(0)],
            audio_assets: vec![],
        },
    };
    // Not panicking means success: the whole chain accepts negative BPM.
    assert!(Player::new(chart).is_ok());
}

// F7: BGM and BGA event queries

/// Create a chart with multiple BGM events.
fn make_chart_with_bgm() -> Chart {
    Chart {
        song: SongInfo {
            title: "BGM Test".into(),
            ..Default::default()
        },
        chart: ChartInfo::default(),
        data: ChartData {
            timing: TimingTrack::simple(120.0, 240).unwrap(),
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: vec![
                Event::bar(0),
                Event::bgm(120, AudioIndex::new(0)),
                Event::bgm(240, AudioIndex::new(1)),
                Event::bgm(360, AudioIndex::new(2)),
                Event::bar(960),
            ],
            // validate checks audio index integrity: three BGM events reference
            // indices 0-2, so the asset table must cover them.
            audio_assets: ["a.wav", "b.wav", "c.wav"]
                .iter()
                .map(|&name| AudioAsset::from_path_buf(PathBuf::from(name), Duration::ZERO, None))
                .collect(),
        },
    }
}

/// Create a chart with multiple BGA events (one per layer).
fn make_chart_with_bga() -> Chart {
    Chart {
        song: SongInfo {
            title: "BGA Test".into(),
            ..Default::default()
        },
        chart: ChartInfo {
            bga_resources: vec![
                BgaResource {
                    path: "base.png".into(),
                    crop: None,
                },
                BgaResource {
                    path: "layer.png".into(),
                    crop: None,
                },
                BgaResource {
                    path: "poor.png".into(),
                    crop: None,
                },
                BgaResource {
                    path: "layer2.png".into(),
                    crop: None,
                },
            ],
            ..Default::default()
        },
        data: ChartData {
            timing: TimingTrack::simple(120.0, 240).unwrap(),
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: vec![
                Event::bar(0),
                Event::new(
                    120,
                    EventKind::Bga {
                        layer: BgaLayer::Base,
                        resource_id: BgaResourceId::new(0),
                    },
                ),
                Event::new(
                    240,
                    EventKind::Bga {
                        layer: BgaLayer::Layer,
                        resource_id: BgaResourceId::new(1),
                    },
                ),
                Event::new(
                    360,
                    EventKind::Bga {
                        layer: BgaLayer::Poor,
                        resource_id: BgaResourceId::new(2),
                    },
                ),
                Event::new(
                    480,
                    EventKind::Bga {
                        layer: BgaLayer::Layer2,
                        resource_id: BgaResourceId::new(3),
                    },
                ),
                Event::bar(960),
            ],
            audio_assets: vec![],
        },
    }
}

/// BGM events are extracted through `events_in_range` plus caller-side kind
/// filtering.
#[test]
fn player_bgm_in_range_returns_events() {
    let chart = make_chart_with_bgm();
    let player = Player::new(chart).unwrap();

    let bgm: Vec<_> = player
        .events_in_range(..)
        .iter()
        .filter_map(|e| {
            if let EventKind::Bgm { audio_index } = &e.kind {
                Some((e.tick(), audio_index.get()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bgm, vec![(120, 0), (240, 1), (360, 2)]);
}

/// BGM subranges are extracted through `events_in_range`.
#[test]
fn player_bgm_in_range_subrange_returns_subset() {
    let chart = make_chart_with_bgm();
    let player = Player::new(chart).unwrap();

    let bgm: Vec<_> = player
        .events_in_range(120..=360)
        .iter()
        .filter_map(|e| {
            if let EventKind::Bgm { .. } = &e.kind {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bgm, vec![120, 240, 360]);
}

/// BGA events for every layer are extracted through `events_in_range` plus
/// caller-side kind filtering.
#[test]
fn player_bga_events_base_returns_all_events() {
    let chart = make_chart_with_bga();
    let player = Player::new(chart).unwrap();

    let bga: Vec<_> = player
        .events_in_range(..)
        .iter()
        .filter_map(|e| {
            if let EventKind::Bga { layer, resource_id } = &e.kind {
                Some((*layer, resource_id.get(), e.tick()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bga.len(), 4);
    assert_eq!(bga[0], (BgaLayer::Base, 0, 120));
    assert_eq!(bga[1], (BgaLayer::Layer, 1, 240));
    assert_eq!(bga[2], (BgaLayer::Poor, 2, 360));
    assert_eq!(bga[3], (BgaLayer::Layer2, 3, 480));
}

/// Filter by `BgaLayer`.
#[test]
fn player_bga_events_filter_by_layer() {
    let chart = make_chart_with_bga();
    let player = Player::new(chart).unwrap();

    let base: Vec<_> = player
        .events_in_range(..)
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Bga {
                    layer: BgaLayer::Base,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].tick(), 120);

    let layer: Vec<_> = player
        .events_in_range(..)
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Bga {
                    layer: BgaLayer::Layer,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(layer.len(), 1);
    assert_eq!(layer[0].tick(), 240);

    let poor: Vec<_> = player
        .events_in_range(..)
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Bga {
                    layer: BgaLayer::Poor,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(poor.len(), 1);
    assert_eq!(poor[0].tick(), 360);

    let layer2: Vec<_> = player
        .events_in_range(..)
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Bga {
                    layer: BgaLayer::Layer2,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(layer2.len(), 1);
    assert_eq!(layer2[0].tick(), 480);
}

/// Multiple BGM channels (different `audio_index` values).
#[test]
fn player_bgm_multiple_channels_returns_all() {
    let chart = make_chart_with_bgm();
    let player = Player::new(chart).unwrap();

    let mut bgm_pairs: Vec<_> = player
        .events_in_range(..)
        .iter()
        .filter_map(|e| {
            if let EventKind::Bgm { audio_index } = &e.kind {
                Some((e.tick(), audio_index.get()))
            } else {
                None
            }
        })
        .collect();
    bgm_pairs.sort_unstable();
    assert_eq!(bgm_pairs, vec![(120, 0), (240, 1), (360, 2)]);
}

#[test]
fn timing_track_rejects_invalid_bpm() {
    let result = TimingTrack::simple(0.0, 240);
    assert!(result.is_err());
    assert_eq!(result, Err(TimingTrackError::InvalidBpm { bpm: 0.0 }));
}
