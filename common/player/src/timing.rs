//! 用于实现 O(log n) 的脉冲到 [`Duration`] 换算的预计算计时缓存。
//!
//! [`TimingCache`] 在 [`Player`](crate::Player) 创建时由 [`TimingTrack`]
//! 构造，避免每次查询时重建事件列表。内部计算使用 `f64`；公开 API 返回
//! [`Duration`]。

use std::time::Duration;

use bmsrs_chart::TimingTrack;

/// 计时轨中 BPM 恒定的一段。
struct BpmSegment {
    /// 本段起始的脉冲位置。
    start_tick: u64,
    /// `start_tick` 处的实际时间秒数（不含停止）。
    start_seconds: f64,
    /// 本段内的 BPM。
    bpm: f64,
}

/// 预计算的计时数据，用于快速进行脉冲到 [`Duration`] 的换算。
///
/// 换算分为两部分：
///
/// 1. **基准时间** —— 在恒定 BPM 段内进行线性插值。
/// 2. **停止暂停** —— 由目标脉冲之前（不含）的所有停止累积的暂停时间。
///
/// 两部分均使用二分查找，每次查询为 O(log n)。
pub struct TimingCache {
    /// 按 `start_tick` 排序的 BPM 段。
    bpm_segments: Vec<BpmSegment>,
    /// 已排序的 `(stop_tick, cumulative_pause_seconds)` 配对。
    stop_cumsum: Vec<(u64, f64)>,
    /// 每个四分音符的脉冲数（节拍分辨率）。
    resolution: u64,
}

impl TimingCache {
    /// 由 [`TimingTrack`] 与节拍分辨率构造缓存。
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    pub(crate) fn new(timing: &TimingTrack, resolution: u64) -> Self {
        debug_assert!(resolution > 0, "resolution must be positive");
        debug_assert!(
            !timing.init_bpm.is_nan() && timing.init_bpm != 0.0,
            "init_bpm must be non-zero finite"
        );

        let res = resolution as f64;

        // 构造 BPM 段（不含停止的累积秒数）。
        // BPM 段存储原始值（可能为负），时序计算使用 |bpm|。
        let mut bpm_segments = vec![BpmSegment {
            start_tick: 0,
            start_seconds: 0.0,
            bpm: timing.init_bpm,
        }];

        let mut current_tick = 0u64;
        let mut current_seconds = 0.0f64;
        let mut current_bpm = timing.init_bpm;

        for bc in &timing.bpm_changes {
            if bc.tick > current_tick {
                current_seconds += (bc.tick - current_tick) as f64 / res * 60.0 / current_bpm.abs();
                current_tick = bc.tick;
            }
            current_bpm = bc.bpm;
            bpm_segments.push(BpmSegment {
                start_tick: current_tick,
                start_seconds: current_seconds,
                bpm: current_bpm,
            });
        }

        // 构造停止累积暂停（按脉冲排序）。
        let mut sorted_stops = timing.stops.clone();
        sorted_stops.sort_by_key(|s| s.tick);

        let mut stop_cumsum = Vec::with_capacity(sorted_stops.len());
        let mut total_pause = 0.0f64;
        for stop in &sorted_stops {
            let bpm = segment_bpm_at_tick(&bpm_segments, stop.tick);
            total_pause += stop.duration as f64 / res * 60.0 / bpm.abs();
            stop_cumsum.push((stop.tick, total_pause));
        }

        Self {
            bpm_segments,
            stop_cumsum,
            resolution,
        }
    }

    /// 将脉冲位置换算为实际时间 [`Duration`]。
    ///
    /// 不计入目标脉冲处的停止（与
    /// [`TimingTrack::tick_to_duration`] 语义一致）。
    #[expect(clippy::cast_precision_loss, reason = "tick fits in f64")]
    #[expect(
        clippy::indexing_slicing,
        reason = "idx from saturating_sub on partition_point, always valid"
    )]
    pub(crate) fn tick_to_duration(&self, tick: u64) -> Duration {
        let res = self.resolution as f64;

        // 由 BPM 段得到的基准时间。
        let idx = self
            .bpm_segments
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1);
        let seg = &self.bpm_segments[idx];
        let base = seg.start_seconds + (tick - seg.start_tick) as f64 / res * 60.0 / seg.bpm.abs();

        // 加上目标脉冲之前（不含）的累积停止暂停。
        let stop_idx = self.stop_cumsum.partition_point(|(t, _)| *t < tick);
        let stop_pause = if stop_idx > 0 {
            self.stop_cumsum[stop_idx - 1].1
        } else {
            0.0
        };

        Duration::from_secs_f64(base + stop_pause)
    }

    /// 返回 `tick` 处生效的 BPM。
    pub(crate) fn bpm_at_tick(&self, tick: u64) -> f64 {
        segment_bpm_at_tick(&self.bpm_segments, tick)
    }

    /// 将实际时间 [`Duration`] 换算为最接近的脉冲位置。
    ///
    /// 这是 [`tick_to_duration`](Self::tick_to_duration) 的逆运算。
    /// 停止期间的时间不会推进脉冲。
    ///
    /// 在 [`tick_to_duration`](Self::tick_to_duration) 上执行二分查找，
    /// 复杂度为 O(log² n)。搜索上界取得很宽裕（超出最后一个 BPM 段
    /// 1000 个小节），以覆盖任意有效时间。
    #[must_use]
    pub(crate) fn duration_to_tick(&self, duration: Duration) -> u64 {
        let target = duration.as_secs_f64();
        if target <= 0.0 {
            return 0;
        }

        // 上界：超出最后一个已知 BPM 段 1000 个小节。
        let last_segment_tick = self.bpm_segments.last().map_or(0, |s| s.start_tick);
        let upper = last_segment_tick + self.resolution * 4 * 1000;

        // 二分查找：找出时间 ≤ target 的最后一个脉冲。
        let mut lo = 0u64;
        let mut hi = upper.max(1);

        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.tick_to_duration(mid).as_secs_f64() <= target {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        lo.saturating_sub(1)
    }
}

/// 二分查找 BPM 段，返回 `tick` 处生效的 BPM。
#[expect(
    clippy::indexing_slicing,
    reason = "idx from saturating_sub on partition_point, always valid"
)]
fn segment_bpm_at_tick(bpm_segments: &[BpmSegment], tick: u64) -> f64 {
    let idx = bpm_segments
        .partition_point(|s| s.start_tick <= tick)
        .saturating_sub(1);
    bpm_segments[idx].bpm
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmsrs_chart::{BpmChange, StopEvent};

    const RES: u64 = 240;

    #[test]
    fn constant_bpm_tick_zero_is_zero() {
        let timing = TimingTrack::new(120.0, vec![], vec![]);
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(0);
        assert_eq!(result, Duration::ZERO);
    }

    #[test]
    fn constant_bpm_120_one_beat_is_half_second() {
        let timing = TimingTrack::new(120.0, vec![], vec![]);
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(240);
        assert_eq!(result, Duration::from_millis(500));
    }

    #[test]
    fn bpm_change_segment_boundary() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: 60.0,
            }],
            vec![],
        );
        let cache = TimingCache::new(&timing, RES);

        // 0-240 在 120 BPM 下 = 0.5s，240-480 在 60 BPM 下 = 1.0s。
        let result = cache.tick_to_duration(480);
        assert_eq!(result, Duration::from_millis(1500));
    }

    #[test]
    fn stop_strictly_before_target_adds_pause() {
        let timing = TimingTrack::new(
            120.0,
            vec![],
            vec![StopEvent {
                tick: 240,
                duration: 240,
            }],
        );
        let cache = TimingCache::new(&timing, RES);

        // 240 处的停止严格在 241 之前，因此暂停被计入。
        let result = cache.tick_to_duration(241);
        let expected = 0.5 + 0.5 + 1.0 / 480.0;
        assert!((result.as_secs_f64() - expected).abs() < 1e-9);
    }

    #[test]
    fn stop_at_target_excludes_pause() {
        let timing = TimingTrack::new(
            120.0,
            vec![],
            vec![StopEvent {
                tick: 240,
                duration: 240,
            }],
        );
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(240);
        assert_eq!(result, Duration::from_millis(500));
    }

    #[test]
    fn matches_timing_track_constant_bpm() {
        let timing = TimingTrack::new(150.0, vec![], vec![]);
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 480, 960, 1920] {
            let expected = timing.tick_to_duration(tick, RES);
            let actual = cache.tick_to_duration(tick);
            assert_eq!(actual, expected, "mismatch at tick {tick}");
        }
    }

    #[test]
    fn matches_timing_track_with_bpm_changes_and_stops() {
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
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 479, 480, 959, 960, 961, 1200, 2400] {
            let expected = timing.tick_to_duration(tick, RES);
            let actual = cache.tick_to_duration(tick);
            assert_eq!(actual, expected, "mismatch at tick {tick}");
        }
    }

    #[test]
    fn bpm_at_tick_returns_correct_bpm() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 480,
                bpm: 200.0,
            }],
            vec![],
        );
        let cache = TimingCache::new(&timing, RES);

        assert!((cache.bpm_at_tick(0) - 120.0).abs() < 1e-9);
        assert!((cache.bpm_at_tick(480) - 200.0).abs() < 1e-9);
        assert!((cache.bpm_at_tick(960) - 200.0).abs() < 1e-9);
    }
}
