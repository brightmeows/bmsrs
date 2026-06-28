#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BpmChange, StopEvent, TimingCache, TimingTrack};
use std::time::Duration;

const RES: u64 = 240;

#[test]
fn constant_bpm_tick_zero_is_zero() {
    let timing = TimingTrack::new(120.0, vec![], vec![]);
    let result = timing.tick_to_duration(0, RES);
    assert_eq!(result, Duration::ZERO);
}

#[test]
fn constant_bpm_120_one_beat_is_half_second() {
    let timing = TimingTrack::new(120.0, vec![], vec![]);
    // 分辨率 240 下 240 脉冲 = 1 拍。
    // 120 BPM 下：1 拍 = 0.5s
    let result = timing.tick_to_duration(240, RES);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn constant_bpm_120_two_beats_is_one_second() {
    let timing = TimingTrack::new(120.0, vec![], vec![]);
    let result = timing.tick_to_duration(480, RES);
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
    );
    // 0-240 在 120 BPM = 0.5s，240-480 在 60 BPM = 1.0s，合计 = 1.5s
    let result = timing.tick_to_duration(480, RES);
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
    );
    let result = timing.tick_to_duration(240, RES);
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
    );
    // 0-240 在 120 BPM = 0.5s
    // 240 处停止：240/240 * 60/120 = 0.5s
    // 240-241 在 120 BPM = 1/480 s
    let result = timing.tick_to_duration(241, RES);
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
    );
    let result = timing.tick_to_duration(240, RES);
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
    );
    // 停止合计 = 1200 脉冲，120 BPM 下 = 2.5s
    // 脉冲 241 = 0.5 + 2.5 + 1/480
    let result = timing.tick_to_duration(241, RES);
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
    );
    let result = timing.tick_to_duration(241, RES);
    let expected = 0.5 + 1.0 + 1.0 / 240.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn duration_to_tick_constant_bpm() {
    let timing = TimingTrack::new(120.0, vec![], vec![]);
    assert_eq!(timing.duration_to_tick(Duration::ZERO, RES), 0);
    assert_eq!(
        timing.duration_to_tick(Duration::from_millis(500), RES),
        240
    );
    assert_eq!(timing.duration_to_tick(Duration::from_secs(1), RES), 480);
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
    );
    // 1.5s -> 脉冲 480（120 BPM 下 0.5s + 60 BPM 下 1.0s）
    assert_eq!(
        timing.duration_to_tick(Duration::from_millis(1500), RES),
        480
    );
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
    );
    // 0.5s = 脉冲 240（刚好到达停止）
    // 0.6s = 位于停止内 -> 仍是脉冲 240
    assert_eq!(
        timing.duration_to_tick(Duration::from_millis(600), RES),
        240
    );
}

#[test]
fn cache_duration_to_tick_matches_timing_track() {
    // P1 前置 + O(log n) 重写安全网：Player 的 advance/seek 由
    // TimingTrack::duration_to_tick 切换到 TimingCache::duration_to_tick，
    // 两者必须在所有场景（含停止内部、BPM 段边界）下语义一致。
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
    );
    let cache = TimingCache::new(&timing, RES);

    // 穷举扫描 0–4s 每 10ms 一个点，覆盖 BPM 段边界、停止内部与段外区域。
    for ms in (0..4000u64).step_by(10) {
        let d = Duration::from_millis(ms);
        let expected = timing.duration_to_tick(d, RES);
        let actual = cache.duration_to_tick(d);
        assert_eq!(
            actual, expected,
            "duration_to_tick mismatch at {ms}ms: timing={expected}, cache={actual}"
        );
    }
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
    );
    for tick in [0u64, 100, 240, 479, 480, 960, 961, 1200, 2400] {
        let dur = timing.tick_to_duration(tick, RES);
        let back = timing.duration_to_tick(dur, RES);
        assert_eq!(
            back, tick,
            "roundtrip failed at tick {tick}: dur={dur:?}, back={back}"
        );
    }
}
