//! `bmsrs-player` 的集成测试。

use std::num::NonZeroU8;
use std::time::Duration;

use bmsrs_chart::{
    BpmChange, Chart, ChartData, ChartInfo, Event, EventKind, Lane, LnJudgeHint, LnLifeHint,
    LnTypeHint, NoteKind, NoteSide, SongInfo, StopEvent, TimingTrack,
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

/// 创建一个简单的测试谱面。
fn make_test_chart() -> Chart {
    // 事件必须按 (tick, priority) 升序排列，否则 Player 的
    // partition_point 无法正确工作。
    Chart {
        song: SongInfo {
            title: "Test".into(),
            ..Default::default()
        },
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(
                120.0,
                vec![BpmChange {
                    tick: 960,
                    bpm: 180.0,
                }],
                vec![],
            ),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![
                Event::new(0, EventKind::Bar),
                Event::new(240, EventKind::Bgm { audio_index: 0 }),
                Event::new(
                    480,
                    EventKind::Note {
                        side: NoteSide::P1,
                        lane: key(1),
                        kind: NoteKind::Normal,
                        audio_index: None,
                        ext: (),
                    },
                ),
                Event::new(960, EventKind::Bar),
                Event::new(960, EventKind::Bpm { bpm: 180.0 }),
            ],
            judge_deltas: None,
            life_deltas: None,
            audio_assets: vec![],
        },
    }
}

// Player 创建与基本操作

#[test]
fn player_new_starts_at_zero() {
    let chart = make_test_chart();
    let player = Player::new(chart);
    assert_eq!(player.current_tick(), 0);
    assert!(player.current_time().is_zero());
}

#[test]
fn player_advance_moves_time_and_tick() {
    let chart = make_test_chart();
    let mut player = Player::new(chart);
    player.advance(Duration::from_secs(1));
    // 120 BPM, resolution 240: 1 second = 480 ticks
    assert_eq!(player.current_tick(), 480);
}

#[test]
fn player_current_time() {
    let chart = make_test_chart();
    let mut player = Player::new(chart);
    player.advance(Duration::from_secs_f64(2.0));
    assert!((player.current_time().as_secs_f64() - 2.0).abs() < 1e-9);
}

#[test]
fn player_seek() {
    let chart = make_test_chart();
    let mut player = Player::new(chart);

    player.seek(Duration::from_secs_f64(1.0));
    assert_eq!(player.current_tick(), 480);
    assert!((player.current_time().as_secs_f64() - 1.0).abs() < 1e-6);
}

#[test]
fn player_reset() {
    let chart = make_test_chart();
    let mut player = Player::new(chart);
    player.advance(Duration::from_secs(2));
    assert!(player.current_tick() > 0);

    player.reset();
    assert_eq!(player.current_tick(), 0);
    assert!(player.current_time().is_zero());
}

#[test]
fn player_duration() {
    let chart = make_test_chart();
    let player = Player::new(chart);
    // With events up to tick 960 at 120 BPM, duration = 2 seconds
    assert!((player.duration().as_secs_f64() - 2.0).abs() < f64::EPSILON);
}

#[test]
fn player_current_bpm_initial() {
    let chart = make_test_chart();
    let player = Player::new(chart);
    assert!((player.current_bpm() - 120.0).abs() < 1e-9);
}

#[test]
fn player_current_bpm_before_change() {
    let chart = make_test_chart();
    let mut player = Player::new(chart);
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
    let mut player = Player::new(chart);
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

// 事件查询

#[test]
fn player_events_in_range() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let events = player.events_in_range(0..=480);
    // Should contain: Bar(0), Bgm(240), Note(480), Bar(960 is outside range)
    // Actually range is 0..=480 so tick 960 is not included
    let ticks: Vec<u64> = events.iter().map(Event::tick).collect();
    assert_eq!(ticks, vec![0, 240, 480]);
}

#[test]
fn player_events_unbounded_range() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let all_events = player.events_in_range(..);
    assert_eq!(all_events.len(), 5);
}

#[test]
fn player_notes_in_range() {
    let chart = make_test_chart();
    let player = Player::new(chart);

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
fn player_notes_in_lane() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let notes_p1k1: Vec<_> = player
        .notes_in_lane(NoteSide::P1, key(1), ..)
        .filter_map(|e| {
            if let EventKind::Note { .. } = &e.kind {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes_p1k1, vec![480]);
}

#[test]
fn player_notes_in_lane_wrong_side() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    assert!(
        player
            .notes_in_lane(NoteSide::P2, key(1), ..)
            .next()
            .is_none()
    );
}

#[test]
fn player_notes_for_judgement() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    // The note in the test chart is Normal → should be counted
    assert_eq!(player.notes_for_judgement(..).count(), 1);
}

#[test]
fn player_bgm_in_range() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let bgm: Vec<_> = player
        .bgm_in_range(..)
        .filter_map(|e| {
            if let EventKind::Bgm { .. } = &e.kind {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bgm, vec![240]);
}

#[test]
fn player_bar_lines_in_range() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let bars: Vec<_> = player
        .bar_lines_in_range(..)
        .filter(|e| matches!(&e.kind, EventKind::Bar))
        .map(Event::tick)
        .collect();
    assert_eq!(bars, vec![0, 960]);
}

#[test]
fn player_bga_events_empty() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    assert!(player.bga_events_in_range(..).next().is_none());
}

// Scroll 缓存

#[test]
fn player_scroll_rate_default() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let rate = player.scroll_rate_at(0);
    assert!((rate - 1.0).abs() < f64::EPSILON);
}

#[test]
fn player_scroll_rate_with_events() {
    let mut chart = make_test_chart();
    chart
        .data
        .events
        .push(Event::new(480, EventKind::Scroll { rate: 2.0 }));
    chart.data.sort_events();
    let player = Player::new(chart);

    // Before scroll event
    assert!((player.scroll_rate_at(0) - 1.0).abs() < f64::EPSILON);
    // After scroll event
    assert!((player.scroll_rate_at(480) - 2.0).abs() < f64::EPSILON);
    // At event
    assert!((player.scroll_rate_at(500) - 2.0).abs() < f64::EPSILON);
}

// Timeline 查询

#[test]
fn player_tick_to_duration() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let dur = player.tick_to_duration(240);
    assert!((dur.as_secs_f64() - 0.5).abs() < 1e-9);
}

#[test]
fn player_duration_to_tick() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let tick = player.duration_to_tick(Duration::from_secs_f64(0.5));
    assert_eq!(tick, 240);
}

// 谱面访问

#[test]
fn player_chart_ref() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let chart_ref = player.chart();
    assert_eq!(chart_ref.song.title, "Test");
}

#[test]
fn player_into_chart() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let chart_back = player.into_chart();
    assert_eq!(chart_back.song.title, "Test");
}

#[test]
fn player_audio_assets() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let assets = player.audio_assets();
    assert!(assets.is_empty());
}

#[test]
fn player_bga_resources() {
    let chart = make_test_chart();
    let player = Player::new(chart);

    let resources = player.bga_resources();
    assert!(resources.is_empty());
}

// 带 BPM 变化的更大范围测试

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
    );
    let mut player = Player::new(chart);

    // 0-240 ticks at 120 BPM = 0.5s
    player.advance(Duration::from_secs_f64(0.5));
    assert_eq!(player.current_tick(), 240);

    // 240-480 ticks at 240 BPM = 0.25s
    player.advance(Duration::from_secs_f64(0.25));
    assert_eq!(player.current_tick(), 480);
}

// 带停止事件的测试

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
    );
    let mut player = Player::new(chart);

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
