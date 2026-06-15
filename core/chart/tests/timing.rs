#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BpmChange, StopEvent, TimingTrack};
use std::time::Duration;

const RES: u64 = 240;

#[test]
fn constant_bpm_tick_zero_is_zero() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![],
    };
    let result = timing.tick_to_duration(0, RES);
    assert_eq!(result, Duration::ZERO);
}

#[test]
fn constant_bpm_120_one_beat_is_half_second() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![],
    };
    // 240 ticks = 1 beat at resolution 240.
    // At 120 BPM: 1 beat = 0.5s
    let result = timing.tick_to_duration(240, RES);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn constant_bpm_120_two_beats_is_one_second() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![],
    };
    let result = timing.tick_to_duration(480, RES);
    assert_eq!(result, Duration::from_secs(1));
}

#[test]
fn bpm_change_at_tick_240() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        stops: vec![],
    };
    // 0-240 at 120 BPM = 0.5s, 240-480 at 60 BPM = 1.0s, total = 1.5s
    let result = timing.tick_to_duration(480, RES);
    assert_eq!(result, Duration::from_millis(1500));
}

#[test]
fn bpm_change_at_target_tick_uses_old_bpm() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        stops: vec![],
    };
    let result = timing.tick_to_duration(240, RES);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn stop_before_target_adds_pause_time() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
    };
    // 0-240 at 120 BPM = 0.5s
    // Stop at 240: 240/240 * 60/120 = 0.5s
    // 240-241 at 120 BPM = 1/480 s
    let result = timing.tick_to_duration(241, RES);
    let expected = 0.5 + 0.5 + 1.0 / 480.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn stop_at_target_tick_excludes_pause() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
    };
    let result = timing.tick_to_duration(240, RES);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn multiple_stops_same_tick_accumulate() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![
            StopEvent {
                tick: 240,
                duration: 240,
            },
            StopEvent {
                tick: 240,
                duration: 960,
            },
        ],
    };
    // Stop total = 1200 ticks at 120 BPM = 2.5s
    // Tick 241 = 0.5 + 2.5 + 1/480
    let result = timing.tick_to_duration(241, RES);
    let expected = 0.5 + 2.5 + 1.0 / 480.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn bpm_before_stop_at_same_tick() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        stops: vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
    };
    let result = timing.tick_to_duration(241, RES);
    let expected = 0.5 + 1.0 + 1.0 / 240.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn duration_to_tick_constant_bpm() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![],
    };
    assert_eq!(timing.duration_to_tick(Duration::ZERO, RES), 0);
    assert_eq!(
        timing.duration_to_tick(Duration::from_millis(500), RES),
        240
    );
    assert_eq!(timing.duration_to_tick(Duration::from_secs(1), RES), 480);
}

#[test]
fn duration_to_tick_bpm_change() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![BpmChange {
            tick: 240,
            bpm: 60.0,
        }],
        stops: vec![],
    };
    // 1.5s -> tick 480 (0.5s at 120 + 1.0s at 60)
    assert_eq!(
        timing.duration_to_tick(Duration::from_millis(1500), RES),
        480
    );
}

#[test]
fn duration_to_tick_within_stop_returns_stop_tick() {
    let timing = TimingTrack {
        init_bpm: 120.0,
        bpm_changes: vec![],
        stops: vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
    };
    // 0.5s = tick 240 (just reached stop)
    // 0.6s = within stop -> still tick 240
    assert_eq!(
        timing.duration_to_tick(Duration::from_millis(600), RES),
        240
    );
}

#[test]
fn roundtrip_tick_to_duration_and_back() {
    let timing = TimingTrack {
        init_bpm: 150.0,
        bpm_changes: vec![
            BpmChange {
                tick: 480,
                bpm: 200.0,
            },
            BpmChange {
                tick: 1200,
                bpm: 100.0,
            },
        ],
        stops: vec![StopEvent {
            tick: 960,
            duration: 480,
        }],
    };
    for tick in [0u64, 100, 240, 479, 480, 960, 961, 1200, 2400] {
        let dur = timing.tick_to_duration(tick, RES);
        let back = timing.duration_to_tick(dur, RES);
        assert_eq!(
            back, tick,
            "roundtrip failed at tick {tick}: dur={dur:?}, back={back}"
        );
    }
}
