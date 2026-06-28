//! 用于脉冲 ↔ Duration 换算的计时轨。
//!
//! [`TimingTrack`] 持有初始 BPM、BPM 变更事件与停止事件。它提供
//! [`TimingTrack::tick_to_duration`] 与 [`TimingTrack::duration_to_tick`]
//! 用于在谱面位置（脉冲）与实际时间（[`Duration`]）之间换算。
//!
//! 内部计算使用 `f64` 运算（BPM 值本质上是浮点数）。`f64` ↔ [`Duration`]
//! 的转换仅发生在公开 API 边界处，通过 [`Duration::from_secs_f64`] 与
//! [`Duration::as_secs_f64`] 完成。

use std::sync::OnceLock;
use std::time::Duration;

/// 用于将脉冲位置换算为实际时间的计时信息。
///
/// 所有事件都在绝对脉冲位置上。处理器负责将格式特有的位置
/// （BMSON 脉冲、BMS 小节）换算为脉冲。
///
/// 合并的 BPM/停止事件列表在首次需要时惰性计算并缓存，避免重复构建。
#[derive(Debug)]
pub struct TimingTrack {
    /// 脉冲 0 处的初始 BPM。
    pub init_bpm: f64,
    /// BPM 变更事件，按脉冲升序排列。
    pub bpm_changes: Vec<BpmChange>,
    /// 停止（暂停）事件，按脉冲升序排列。
    pub stops: Vec<StopEvent>,
    /// 惰性缓存的合并事件列表（BPM 变更 + 停止，已排序）。
    events_cache: OnceLock<Vec<(u64, TimingEvent)>>,
}

/// BPM 变更事件。
#[derive(Clone, Debug, PartialEq)]
pub struct BpmChange {
    /// BPM 发生变更的脉冲位置。
    pub tick: u64,
    /// 新的 BPM（每分钟拍数）。
    pub bpm: f64,
}

/// 停止（暂停）事件。
///
/// 当回放到达 `tick` 时，滚动暂停 `duration` 个脉冲的时长（按当前 BPM
/// 计算）。同一脉冲上的多个停止会累加。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopEvent {
    /// 停止开始的脉冲位置。
    pub tick: u64,
    /// 时长（脉冲数）。
    pub duration: u64,
}

/// 合并时间线的内部事件表示。
#[derive(Clone, Copy, Debug)]
enum TimingEvent {
    /// BPM 变更 —— `is_stop = false` 确保同一脉冲上 BPM 排在 Stop 之前。
    Bpm(f64),
    /// 停止，时长以脉冲数表示。
    Stop(u64),
}

impl TimingEvent {
    /// 当此事件为 [`TimingEvent::Stop`] 时返回 `true`。
    const fn is_stop(self) -> bool {
        matches!(self, Self::Stop(_))
    }
}

impl TimingTrack {
    /// 创建一个新的计时轨。
    ///
    /// 使用构造函数而非直接构造结构体以确保内部缓存正确初始化。
    #[must_use]
    pub const fn new(init_bpm: f64, bpm_changes: Vec<BpmChange>, stops: Vec<StopEvent>) -> Self {
        Self {
            init_bpm,
            bpm_changes,
            stops,
            events_cache: OnceLock::new(),
        }
    }

    /// 由 `bpm_changes` 与停止事件构造已排序的事件列表。
    ///
    /// 第一次调用时构建并缓存结果，后续调用返回缓存引用。
    /// 同一脉冲上，BPM 变更排在停止之前（依据 BMSON 规范：
    /// "speed will first change, then the music pauses"），与
    /// [`Event::priority`](crate::Event::priority) 中 `Bpm(2) < Stop(3)`
    /// 的子序约定一致。
    fn cached_events(&self) -> &[(u64, TimingEvent)] {
        self.events_cache.get_or_init(|| {
            let mut events: Vec<(u64, TimingEvent)> =
                Vec::with_capacity(self.bpm_changes.len() + self.stops.len());
            for bc in &self.bpm_changes {
                events.push((bc.tick, TimingEvent::Bpm(bc.bpm)));
            }
            for st in &self.stops {
                events.push((st.tick, TimingEvent::Stop(st.duration)));
            }
            // 按脉冲排序，再按 BPM（false）排在 Stop（true）之前。
            events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.is_stop().cmp(&b.1.is_stop())));
            events
        })
    }

    /// 将脉冲位置换算为实际时间 [`Duration`]。
    ///
    /// 在带有停止事件的脉冲上，返回的时间是暂停**之前**的时刻
    /// （依据 BMSON 规范，该脉冲上的音符在暂停之前激活）。
    /// 严格位于目标脉冲之前的停止贡献其完整暂停时长。
    ///
    /// # Panic（仅 debug 构建）
    ///
    /// debug 构建中断言 `resolution > 0` 与 `init_bpm > 0`。
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "larger function; compiler can decide inlining"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "lossless or explicitly-rounded numeric conversion"
    )]
    pub fn tick_to_duration(&self, tick: u64, resolution: u64) -> Duration {
        debug_assert!(resolution > 0, "resolution must be > 0");
        debug_assert!(
            !self.init_bpm.is_nan() && self.init_bpm != 0.0,
            "init_bpm must be non-zero finite"
        );

        let res = resolution as f64;
        let events = self.cached_events();

        let mut seconds = 0.0f64;
        let mut current_tick = 0u64;
        let mut current_bpm = self.init_bpm;

        for (event_tick, event) in events {
            if *event_tick > tick {
                break;
            }
            // 将回放推进到 event_tick。
            if *event_tick > current_tick {
                let delta = (*event_tick - current_tick) as f64;
                seconds += delta / res * 60.0 / current_bpm.abs();
                current_tick = *event_tick;
            }
            match event {
                TimingEvent::Bpm(bpm) => {
                    current_bpm = *bpm;
                }
                TimingEvent::Stop(duration) => {
                    // 仅当停止严格位于目标脉冲之前时才计入停止时间。
                    // 在目标脉冲本身处，时间是暂停之前的时刻（依据规范）。
                    if *event_tick < tick {
                        seconds += *duration as f64 / res * 60.0 / current_bpm.abs();
                    }
                }
            }
        }

        // 从最后一个事件到目标脉冲的剩余时间。
        if tick > current_tick {
            let delta = (tick - current_tick) as f64;
            seconds += delta / res * 60.0 / current_bpm.abs();
        }

        Duration::from_secs_f64(seconds)
    }

    /// 将实际时间 [`Duration`] 换算为最近的脉冲位置。
    ///
    /// 这是 [`tick_to_duration`](Self::tick_to_duration) 的逆运算。
    /// 停止中消耗的时间不会推进脉冲。
    ///
    /// # Panic（仅 debug 构建）
    ///
    /// debug 构建中断言 `resolution > 0` 与 `init_bpm > 0`。
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded result is within u64 range"
    )]
    #[expect(clippy::cast_sign_loss, reason = "remaining time is non-negative")]
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "larger function; compiler can decide inlining"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "lossless or explicitly-rounded numeric conversion"
    )]
    pub fn duration_to_tick(&self, duration: Duration, resolution: u64) -> u64 {
        debug_assert!(resolution > 0, "resolution must be > 0");
        debug_assert!(
            !self.init_bpm.is_nan() && self.init_bpm != 0.0,
            "init_bpm must be non-zero finite"
        );

        let seconds = duration.as_secs_f64();

        if seconds <= 0.0 {
            return 0;
        }

        let res = resolution as f64;
        let events = self.cached_events();

        let mut remaining = seconds;
        let mut current_tick = 0u64;
        let mut current_bpm = self.init_bpm;

        for (event_tick, event) in events {
            // 将回放推进到 event_tick。
            if *event_tick > current_tick {
                let delta_ticks = *event_tick - current_tick;
                let delta_seconds = delta_ticks as f64 / res * 60.0 / current_bpm.abs();
                if delta_seconds >= remaining {
                    return current_tick
                        + (remaining * res * current_bpm.abs() / 60.0).round() as u64;
                }
                remaining -= delta_seconds;
                current_tick = *event_tick;
            }

            match event {
                TimingEvent::Bpm(bpm) => {
                    current_bpm = *bpm;
                }
                TimingEvent::Stop(stop_duration) => {
                    let stop_seconds = *stop_duration as f64 / res * 60.0 / current_bpm.abs();
                    if stop_seconds >= remaining {
                        // 目标位于停止内 —— 脉冲不推进。
                        return current_tick;
                    }
                    remaining -= stop_seconds;
                }
            }
        }

        // 目标超出所有事件。
        current_tick + (remaining * res * current_bpm.abs() / 60.0).round() as u64
    }
}

impl Clone for TimingTrack {
    fn clone(&self) -> Self {
        Self {
            init_bpm: self.init_bpm,
            bpm_changes: self.bpm_changes.clone(),
            stops: self.stops.clone(),
            // 不复制缓存状态——克隆体首次使用时惰性构建。
            events_cache: OnceLock::new(),
        }
    }
}

impl Default for TimingTrack {
    fn default() -> Self {
        Self {
            init_bpm: 0.0,
            bpm_changes: Vec::new(),
            stops: Vec::new(),
            events_cache: OnceLock::new(),
        }
    }
}

impl PartialEq for TimingTrack {
    fn eq(&self, other: &Self) -> bool {
        self.init_bpm == other.init_bpm
            && self.bpm_changes == other.bpm_changes
            && self.stops == other.stops
        // 忽略 events_cache 状态——无论是否缓存，原始数据相同则相等。
    }
}

// init_bpm 保证不含 NaN（构造时通过 debug_assert 验证），因此
// PartialEq 满足 Eq 的反射性要求。
impl Eq for TimingTrack {}

// TimingCache —— 预计算计时索引

/// 计时轨中 BPM 恒定的一段。
struct BpmSegment {
    /// 本段起始的脉冲位置。
    start_tick: u64,
    /// `start_tick` 处的实际时间秒数（不含停止）。
    start_seconds: f64,
    /// 本段内的 BPM。
    bpm: f64,
}

/// 预计算的计时索引，用于快速进行脉冲到 [`Duration`] 的换算。
///
/// 由 [`TimingTrack`] 构造，把每次查询从 O(n) 线性扫描降为 O(log n) 二分
/// 查找。适用于任何需要批量或频繁换算脉冲↔时间的场景：播放器实时查询、
/// 处理器离线切片等。
///
/// 换算分为两部分：
///
/// 1. **基准时间** —— 在恒定 BPM 段内进行线性插值。
/// 2. **停止暂停** —— 由目标脉冲之前（不含）的所有停止累积的暂停时间。
///
/// 两部分均使用二分查找，每次查询为 O(log n)。语义与
/// [`TimingTrack::tick_to_duration`] / [`TimingTrack::duration_to_tick`]
/// 一致，是它们的加速版本。
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
    #[must_use]
    pub fn new(timing: &TimingTrack, resolution: u64) -> Self {
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
    #[must_use]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
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
    #[must_use]
    pub fn bpm_at_tick(&self, tick: u64) -> f64 {
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
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
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
