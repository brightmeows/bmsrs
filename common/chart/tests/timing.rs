#![expect(missing_docs, reason = "integration test")]

use bmsrs_chart::{BpmChange, StopEvent, TimingCache, TimingTrack};
use std::time::Duration;

const RES: u64 = 240;

#[test]
fn constant_bpm_tick_zero_is_zero() {
    let timing = TimingTrack::simple(120.0).unwrap();
    let result = timing.tick_to_duration(0, RES);
    assert_eq!(result, Duration::ZERO);
}

#[test]
fn constant_bpm_120_one_beat_is_half_second() {
    let timing = TimingTrack::simple(120.0).unwrap();
    // 分辨率 240 下 240 脉冲 = 1 拍。
    // 120 BPM 下：1 拍 = 0.5s
    let result = timing.tick_to_duration(240, RES);
    assert_eq!(result, Duration::from_millis(500));
}

#[test]
fn constant_bpm_120_two_beats_is_one_second() {
    let timing = TimingTrack::simple(120.0).unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
    let result = timing.tick_to_duration(241, RES);
    let expected = 0.5 + 1.0 + 1.0 / 240.0;
    assert!((result.as_secs_f64() - expected).abs() < 1e-9);
}

#[test]
fn duration_to_tick_constant_bpm() {
    let timing = TimingTrack::simple(120.0).unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
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

// F6: TimingTrack extreme values

/// `u64::MAX / 2` 不会导致溢出 panic。
#[test]
fn extreme_large_tick_no_panic() {
    let timing = TimingTrack::simple(120.0).unwrap();
    let large_tick = u64::MAX / 2;
    let dur = timing.tick_to_duration(large_tick, RES);
    assert!(dur.as_secs() > 0);
    // roundtrip 不应 panic
    let _tick = timing.duration_to_tick(dur, RES);
}

/// 极小 BPM（0.001）。
#[test]
fn extreme_small_bpm() {
    let timing = TimingTrack::simple(0.001).unwrap();
    // 0.001 BPM：1 tick（240 res）≈ 60/0.001 * 1/240 = 250s
    let dur = timing.tick_to_duration(1, RES);
    assert!(dur.as_secs_f64() > 240.0);
}

/// 极大 BPM（1e6）。
#[test]
fn extreme_large_bpm() {
    let timing = TimingTrack::simple(1_000_000.0).unwrap();
    // 1e6 BPM：240 ticks = 1 beat = 60/1e6 s = 0.00006s
    let dur = timing.tick_to_duration(RES, RES);
    assert!(dur.as_secs_f64() < 0.001);
}

/// 极长曲目（>1 小时）的时间换算。
#[test]
fn very_long_song() {
    let timing = TimingTrack::simple(60.0).unwrap();
    // 60 BPM、分辨率 240：1 小时 = 60 分 = 3600 拍 = 864000 ticks
    let one_hour_ticks = 240 * 60 * 60; // 864000
    let dur = timing.tick_to_duration(one_hour_ticks, RES);
    assert!((dur.as_secs_f64() - 3600.0).abs() < 1.0);
    // roundtrip
    let back = timing.duration_to_tick(dur, RES);
    assert_eq!(back, one_hour_ticks);
}

/// BPM 极端值交替。
#[test]
fn alternating_bpm_extremes() {
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
    )
    .unwrap();
    // 0–240 ticks at 0.001 BPM ≈ 240 * 60/0.001 / 240 = 60000s
    // 240–480 ticks at 1e6 BPM ≈ 240 * 60/1e6 / 240 = 0.00006s
    // 480+ ticks at 0.001 BPM again
    let dur_240 = timing.tick_to_duration(240, RES);
    assert!(dur_240.as_secs_f64() > 50_000.0);
    let dur_480 = timing.tick_to_duration(480, RES);
    // 240 ticks at 0.001 BPM ≈ 60000s + 240 ticks at 1e6 BPM ≈ 0.00006s
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
    )
    .unwrap();
    for tick in [0u64, 100, 240, 479, 480, 960, 961, 1200, 2400] {
        let dur = timing.tick_to_duration(tick, RES);
        let back = timing.duration_to_tick(dur, RES);
        assert_eq!(
            back, tick,
            "roundtrip failed at tick {tick}: dur={dur:?}, back={back}"
        );
    }
}
