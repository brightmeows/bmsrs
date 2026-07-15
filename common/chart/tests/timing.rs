#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BpmChange, StopEvent, TimingTrack};
use std::time::Duration;

const RES: u64 = 240;

#[test]
fn constant_bpm_tick_zero_is_zero() {
    let timing = TimingTrack::simple(120.0, RES).unwrap();
    let result = timing.tick_to_duration(0);
    assert_eq!(result, Duration::ZERO);
}

#[test]
fn constant_bpm_120_one_beat_is_half_second() {
    let timing = TimingTrack::simple(120.0, RES).unwrap();
    let result = timing.tick_to_duration(240);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn constant_bpm_120_two_beats_is_one_second() {
    let timing = TimingTrack::simple(120.0, RES).unwrap();
    let result = timing.tick_to_duration(480);
    assert_eq!(result, Duration::from_secs(1));
}

#[test]
fn bpm_change_at_tick_240() {
    let timing = TimingTrack::new(
        120.0,
        vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        vec![],
        RES,
    )
    .unwrap();
    let result = timing.tick_to_duration(480);
    assert_eq!(result, Duration::from_millis(1500));
}

#[test]
fn bpm_change_at_target_tick_uses_old_bpm() {
    let timing = TimingTrack::new(
        120.0,
        vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        vec![],
        RES,
    )
    .unwrap();
    let result = timing.tick_to_duration(240);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn stop_before_target_adds_pause_time() {
    let timing = TimingTrack::new(
        120.0,
        vec![],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
        RES,
    )
    .unwrap();
    let result = timing.tick_to_duration(241);
    let expected = 0.5 + 0.5 + 1.0 / 480.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn stop_at_target_tick_excludes_pause() {
    let timing = TimingTrack::new(
        120.0,
        vec![],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
        RES,
    )
    .unwrap();
    let result = timing.tick_to_duration(240);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn multiple_stops_same_tick_accumulate() {
    let timing = TimingTrack::new(
        120.0,
        vec![],
        vec![
            StopEvent {
                tick: 240,
                duration: 240,
            },
            StopEvent {
                tick: 240,
                duration: 960,
            },
        ],
        RES,
    )
    .unwrap();
    let result = timing.tick_to_duration(241);
    let expected = 0.5 + 2.5 + 1.0 / 480.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn bpm_before_stop_at_same_tick() {
    let timing = TimingTrack::new(
        120.0,
        vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
        RES,
    )
    .unwrap();
    let result = timing.tick_to_duration(241);
    let expected = 0.5 + 1.0 + 1.0 / 240.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn duration_to_tick_constant_bpm() {
    let timing = TimingTrack::simple(120.0, RES).unwrap();
    assert_eq!(timing.duration_to_tick(Duration::ZERO), 0);
    assert_eq!(timing.duration_to_tick(Duration::from_millis(500)), 240);
    assert_eq!(timing.duration_to_tick(Duration::from_secs(1)), 480);
}

#[test]
fn duration_to_tick_bpm_change() {
    let timing = TimingTrack::new(
        120.0,
        vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        vec![],
        RES,
    )
    .unwrap();
    assert_eq!(timing.duration_to_tick(Duration::from_millis(1500)), 480);
}

#[test]
fn duration_to_tick_within_stop_returns_stop_tick() {
    let timing = TimingTrack::new(
        120.0,
        vec![],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
        RES,
    )
    .unwrap();
    assert_eq!(timing.duration_to_tick(Duration::from_millis(600)), 240);
}

#[test]
fn cache_duration_to_tick_matches_timing_track() {
    let timing = TimingTrack::new(
        150.0,
        vec![
            BpmChange {
                tick: 480,
                bpm: 200.0,
            },
            BpmChange {
                tick: 1200,
                bpm: 100.0,
            },
        ],
        vec![
            StopEvent {
                tick: 960,
                duration: 480,
            },
            StopEvent {
                tick: 2400,
                duration: 960,
            },
        ],
        RES,
    )
    .unwrap();
    for ms in (0..4000u64).step_by(10) {
        let d = Duration::from_millis(ms);
        let expected = timing.duration_to_tick(d);
        let actual = timing.duration_to_tick(d);
        assert_eq!(
            actual, expected,
            "duration_to_tick mismatch at {ms}ms: timing={expected}, cache={actual}"
        );
    }
}

// F6: TimingTrack extreme values

#[test]
fn extreme_large_tick_no_panic() {
    let timing = TimingTrack::simple(120.0, RES).unwrap();
    let large_tick = u64::MAX / 2;
    let dur = timing.tick_to_duration(large_tick);
    assert!(dur.as_secs() > 0);
    let _tick = timing.duration_to_tick(dur);
}

#[test]
fn extreme_small_bpm_stays_valid() {
    let timing = TimingTrack::simple(0.001, RES).unwrap();
    let dur = timing.tick_to_duration(1);
    assert!(dur.as_secs_f64() > 240.0);
}

#[test]
fn extreme_large_bpm_stays_valid() {
    let timing = TimingTrack::simple(1_000_000.0, RES).unwrap();
    let dur = timing.tick_to_duration(RES);
    assert!(dur.as_secs_f64() < 0.001);
}

#[test]
fn very_long_song_computes_duration() {
    let timing = TimingTrack::simple(60.0, RES).unwrap();
    let one_hour_ticks = 240 * 60 * 60; // 864000
    let dur = timing.tick_to_duration(one_hour_ticks);
    assert!((dur.as_secs_f64() - 3600.0).abs() < 1.0);
    let back = timing.duration_to_tick(dur);
    assert_eq!(back, one_hour_ticks);
}

#[test]
fn alternating_bpm_extremes_no_panic() {
    let timing = TimingTrack::new(
        0.001,
        vec![
            BpmChange {
                tick: 240,
                bpm: 1_000_000.0,
            },
            BpmChange {
                tick: 480,
                bpm: 0.001,
            },
        ],
        vec![],
        RES,
    )
    .unwrap();
    let dur_240 = timing.tick_to_duration(240);
    assert!(dur_240.as_secs_f64() > 50_000.0);
    let dur_480 = timing.tick_to_duration(480);
    assert!((dur_480.as_secs_f64() - dur_240.as_secs_f64() - 0.00006).abs() < 1e-6);
}

#[test]
fn roundtrip_tick_to_duration_and_back() {
    let timing = TimingTrack::new(
        150.0,
        vec![
            BpmChange {
                tick: 480,
                bpm: 200.0,
            },
            BpmChange {
                tick: 1200,
                bpm: 100.0,
            },
        ],
        vec![StopEvent {
            tick: 960,
            duration: 480,
        }],
        RES,
    )
    .unwrap();
    for tick in [0u64, 100, 240, 479, 480, 960, 961, 1200, 2400] {
        let dur = timing.tick_to_duration(tick);
        let back = timing.duration_to_tick(dur);
        assert_eq!(
            back, tick,
            "roundtrip failed at tick {tick}: dur={dur:?}, back={back}"
        );
    }
}
