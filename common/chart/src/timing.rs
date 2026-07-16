//! 用于脉冲 ↔ Duration 换算的计时轨。
//!
//! [`TimingTrack`] 持有初始 BPM、BPM 变更事件与停止事件。它提供
//! [`TimingTrack::tick_to_duration`] 与 [`TimingTrack::duration_to_tick`]
//! 用于在谱面位置（脉冲）与实际时间（[`Duration`]）之间换算。
//!
//! 内部计算使用 `f64` 运算（BPM 值本质上是浮点数）。`f64` ↔ [`Duration`]
//! 的转换仅发生在公开 API 边界处，通过 [`Duration::from_secs_f64`] 与
//! [`Duration::as_secs_f64`] 完成。

use std::time::Duration;

/// `TimingTrack` 构造或验证时发生的错误。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TimingTrackError {
    /// 初始 BPM 无效（必须为非零有限值）。
    InvalidBpm {
        /// 无效的 BPM 值。
        bpm: f64,
    },
    /// 节拍分辨率 `resolution` 为零（必须为正）。
    ZeroResolution,
}

impl std::fmt::Display for TimingTrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBpm { bpm } => {
                write!(f, "invalid initial BPM: {bpm} (must be non-zero finite)")
            }
            Self::ZeroResolution => write!(f, "resolution must be positive, got 0"),
        }
    }
}

impl std::error::Error for TimingTrackError {}

impl TimingTrackError {
    /// 返回导致错误的 BPM 值；[`ZeroResolution`](Self::ZeroResolution) 时返回 `0.0`。
    #[must_use]
    pub const fn bpm(&self) -> f64 {
        match self {
            Self::InvalidBpm { bpm } => *bpm,
            Self::ZeroResolution => 0.0,
        }
    }
}

/// 用于将脉冲位置换算为实际时间的计时信息。
///
/// 所有事件都在绝对脉冲位置上。处理器负责将格式特有的位置
/// （BMSON 脉冲、BMS 小节）换算为脉冲。
///
/// 内部使用预计算的 BPM 段与停止累积和，所有查询均为 O(log n)。
#[derive(Debug)]
pub struct TimingTrack {
    /// 脉冲 0 处的初始 BPM。
    pub(crate) init_bpm: f64,
    /// 每个四分音符的脉冲数（节拍分辨率）。
    resolution: u64,
    /// BPM 变更事件，按脉冲升序排列。
    pub(crate) bpm_changes: Vec<BpmChange>,
    /// 停止（暂停）事件，按脉冲升序排列。
    pub(crate) stops: Vec<StopEvent>,
    /// 按 `start_tick` 排序的 BPM 段。
    bpm_segments: Vec<BpmSegment>,
    /// 已排序的 `(stop_tick, cumulative_pause_seconds)` 配对。
    stop_cumsum: Vec<(u64, f64)>,
    /// `duration_to_tick` 的逆查找段（按 `sec_lo` 升序）。
    inv: Vec<InvSeg>,
}

impl TimingTrack {
    /// 返回初始 BPM。
    #[must_use]
    pub const fn init_bpm(&self) -> f64 {
        self.init_bpm
    }

    /// 返回 BPM 变更事件切片。
    #[must_use]
    pub fn bpm_changes(&self) -> &[BpmChange] {
        &self.bpm_changes
    }

    /// 返回停止事件切片。
    #[must_use]
    pub fn stops(&self) -> &[StopEvent] {
        &self.stops
    }

    /// 返回节拍分辨率。
    #[must_use]
    pub const fn resolution(&self) -> u64 {
        self.resolution
    }

    /// 验证当前状态的不变量。
    ///
    /// # Errors
    ///
    /// 若 `init_bpm` 无效（零、NaN 或无穷大），返回
    /// [`TimingTrackError::InvalidBpm`]。
    pub fn validate(&self) -> Result<(), TimingTrackError> {
        if self.init_bpm.is_finite() && self.init_bpm != 0.0 {
            Ok(())
        } else {
            Err(TimingTrackError::InvalidBpm { bpm: self.init_bpm })
        }
    }
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
    #[expect(dead_code, reason = "保留用于基于排序的备用 merge 逻辑")]
    const fn is_stop(self) -> bool {
        matches!(self, Self::Stop(_))
    }
}

/// 检查 BPM 值是否有效（有限且非零）。
///
/// 无效值（`0.0`、`NaN`、`±Inf`）的 BPM 变更在消费点被跳过，
/// 保持上一个有效 BPM 不变——与 beatoraja 行为一致。
const fn is_valid_bpm(bpm: f64) -> bool {
    bpm.is_finite() && bpm != 0.0
}

/// 将秒数安全转换为 [`Duration`]，避免溢出 panic。
///
/// 负值与 NaN 钳位为 [`Duration::ZERO`]，超出 [`Duration::MAX`] 的值
/// 钳位为 [`Duration::MAX`]。正常范围内的值不受影响。
fn safe_from_secs_f64(secs: f64) -> Duration {
    Duration::try_from_secs_f64(secs.max(0.0)).unwrap_or(Duration::MAX)
}

impl TimingTrack {
    /// 创建一个新的计时轨。
    ///
    /// 在构造时预计算所有内部段，后续查询为 O(log n)。
    ///
    /// # Errors
    ///
    /// - 若 `resolution == 0`，返回 [`TimingTrackError::ZeroResolution`]。
    /// - 若 `init_bpm` 无效（零、NaN 或无穷大），返回
    ///   [`TimingTrackError::InvalidBpm`]。
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    pub fn new(
        init_bpm: f64,
        mut bpm_changes: Vec<BpmChange>,
        mut stops: Vec<StopEvent>,
        resolution: u64,
    ) -> Result<Self, TimingTrackError> {
        if resolution == 0 {
            return Err(TimingTrackError::ZeroResolution);
        }
        if !init_bpm.is_finite() || init_bpm == 0.0 {
            return Err(TimingTrackError::InvalidBpm { bpm: init_bpm });
        }

        let res = resolution as f64;

        // 在构造时一次性排序，避免下游 clone+sort。
        bpm_changes.sort_by_key(|bc| bc.tick);
        stops.sort_by_key(|s| s.tick);

        // Build BPM segments
        let mut bpm_segments = vec![BpmSegment {
            start_tick: 0,
            start_seconds: 0.0,
            bpm: init_bpm,
        }];

        let mut current_tick = 0u64;
        let mut current_seconds = 0.0f64;
        let mut current_bpm = init_bpm;

        for bc in &bpm_changes {
            if !is_valid_bpm(bc.bpm) {
                continue;
            }
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

        // Build stop cumulative sum
        let mut stop_cumsum = Vec::with_capacity(stops.len());
        let mut total_pause = 0.0f64;
        for stop in &stops {
            let bpm = segment_bpm_at_tick(&bpm_segments, stop.tick);
            total_pause += stop.duration as f64 / res * 60.0 / bpm.abs();
            stop_cumsum.push((stop.tick, total_pause));
        }

        // Build inv segments（bpm_changes 与 stops 已排序）
        let inv = merge_inv(init_bpm, &bpm_changes, &stops, resolution);

        Ok(Self {
            init_bpm,
            resolution,
            bpm_changes,
            stops,
            bpm_segments,
            stop_cumsum,
            inv,
        })
    }

    /// 创建仅含初始 BPM、无 BPM 变更和无停止的计时轨。
    ///
    /// 等价于 `TimingTrack::new(bpm, vec![], vec![], resolution)`，但无需
    /// 手动传递空向量，在测试和简单场景中更简洁。
    ///
    /// # Errors
    ///
    /// 若 `init_bpm` 无效，返回 [`TimingTrackError::InvalidBpm`]。
    /// 若 `resolution == 0`，返回 [`TimingTrackError::ZeroResolution`]。
    pub fn simple(init_bpm: f64, resolution: u64) -> Result<Self, TimingTrackError> {
        Self::new(init_bpm, Vec::new(), Vec::new(), resolution)
    }

    /// 将脉冲位置换算为实际时间 [`Duration`]。
    ///
    /// 在带有停止事件的脉冲上，返回的时间是暂停**之前**的时刻
    /// （依据 BMSON 规范，该脉冲上的音符在暂停之前激活）。
    /// 严格位于目标脉冲之前的停止贡献其完整暂停时长。
    ///
    /// 使用预计算段执行 O(log n) 二分查找。
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "idx from saturating_sub on partition_point, always valid"
    )]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
        let res = self.resolution as f64;

        // 1. BPM 段查找得到基准时间。
        let idx = self
            .bpm_segments
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1);
        let seg = &self.bpm_segments[idx];
        let base = seg.start_seconds + (tick - seg.start_tick) as f64 / res * 60.0 / seg.bpm.abs();

        // 2. 目标脉冲之前（不含）的累积停止暂停。
        let stop_idx = self.stop_cumsum.partition_point(|(t, _)| *t < tick);
        let pause = if stop_idx > 0 {
            self.stop_cumsum[stop_idx - 1].1
        } else {
            0.0
        };

        safe_from_secs_f64(base + pause)
    }

    /// 将实际时间 [`Duration`] 换算为最近的脉冲位置。
    ///
    /// 这是 [`tick_to_duration`](Self::tick_to_duration) 的逆运算。
    /// 停止中消耗的时间不会推进脉冲。
    ///
    /// 在预计算的逆查找段上执行二分查找，复杂度为 O(log n)。
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded result is within u64 range"
    )]
    #[expect(clippy::cast_sign_loss, reason = "remaining time is non-negative")]
    #[expect(
        clippy::indexing_slicing,
        reason = "inv non-empty (always has initial seg); idx from saturating_sub on partition_point"
    )]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        let target = duration.as_secs_f64();
        if target <= 0.0 {
            return 0;
        }

        let idx = self
            .inv
            .partition_point(|s| s.sec_lo <= target)
            .saturating_sub(1);
        let seg = &self.inv[idx];

        if seg.bpm == 0.0 {
            // 冻结段（停止内）：脉冲不推进。
            return seg.tick;
        }

        // 线性段内插值：dt 秒对应 dt * res * |bpm| / 60 个脉冲。
        let dt = target - seg.sec_lo;
        seg.tick + (dt * self.resolution as f64 * seg.bpm.abs() / 60.0).round() as u64
    }

    /// 返回 `tick` 处生效的 BPM。
    #[must_use]
    pub fn bpm_at_tick(&self, tick: u64) -> f64 {
        segment_bpm_at_tick(&self.bpm_segments, tick)
    }
}

impl Clone for TimingTrack {
    fn clone(&self) -> Self {
        Self {
            init_bpm: self.init_bpm,
            resolution: self.resolution,
            bpm_changes: self.bpm_changes.clone(),
            stops: self.stops.clone(),
            bpm_segments: self.bpm_segments.clone(),
            stop_cumsum: self.stop_cumsum.clone(),
            inv: self.inv.clone(),
        }
    }
}

impl Default for TimingTrack {
    #[expect(
        clippy::expect_used,
        reason = "120 BPM and resolution 240 are hardcoded constants that are always valid"
    )]
    fn default() -> Self {
        Self::new(120.0, vec![], vec![], 240).expect("120 BPM at resolution 240 is always valid")
    }
}

impl PartialEq for TimingTrack {
    fn eq(&self, other: &Self) -> bool {
        self.init_bpm == other.init_bpm
            && self.resolution == other.resolution
            && self.bpm_changes == other.bpm_changes
            && self.stops == other.stops
    }
}

// init_bpm 保证不含 NaN（构造时通过 TimingTrack::new 验证），因此
// PartialEq 满足 Eq 的反射性要求。
impl Eq for TimingTrack {}

// TimingCache —— 预计算计时索引（已弃用）

/// 计时轨中 BPM 恒定的一段。
#[derive(Clone, Debug)]
struct BpmSegment {
    /// 本段起始的脉冲位置。
    start_tick: u64,
    /// `start_tick` 处的实际时间秒数（不含停止）。
    start_seconds: f64,
    /// 本段内的 BPM。
    bpm: f64,
}

/// `duration_to_tick` 逆查找的一段。
///
/// 将单调的 `total_seconds(tick)` 函数拆分为若干段：`BPM` 恒定的线性段
/// （脉冲随时间推进）与停止冻结段（脉冲恒定、时间跳过停止时长）。
/// 各段 `sec_lo` 严格递增，支持 O(log n) 二分查找。
#[derive(Clone, Debug)]
struct InvSeg {
    /// 本段起始的实际时间（秒）。
    sec_lo: f64,
    /// `sec_lo` 处的脉冲位置。
    tick: u64,
    /// 本段 BPM；`0.0` 表示冻结段（停止期间脉冲不推进）。
    bpm: f64,
}

/// [`TimingTrack`] 的委托包装（已弃用）。
///
/// 保留此类型仅为兼容旧调用方。所有方法直接委托给内部的
/// [`TimingTrack`] 实例。
///
/// # 弃用
///
/// 直接使用 [`TimingTrack::tick_to_duration`] / [`TimingTrack::bpm_at_tick`] 代替。
#[derive(Debug)]
#[deprecated(note = "直接使用 TimingTrack 的 tick_to_duration / bpm_at_tick 代替")]
pub struct TimingCache {
    /// 内部的 [`TimingTrack`] 实例，所有方法委托至此。
    timing: TimingTrack,
}

#[expect(deprecated, reason = "仍保留 TimingCache 供旧调用方使用")]
impl TimingCache {
    /// 由 [`TimingTrack`] 与节拍分辨率构造缓存。
    ///
    /// # 弃用
    ///
    /// 直接使用 [`TimingTrack`] 的方法代替。
    #[must_use]
    #[deprecated(note = "直接使用 TimingTrack 的 tick_to_duration / bpm_at_tick 代替")]
    pub fn new(timing: &TimingTrack, _resolution: u64) -> Self {
        Self {
            timing: timing.clone(),
        }
    }

    /// 将脉冲位置换算为实际时间 [`Duration`]。
    #[must_use]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
        self.timing.tick_to_duration(tick)
    }

    /// 返回 `tick` 处生效的 BPM。
    #[must_use]
    pub fn bpm_at_tick(&self, tick: u64) -> f64 {
        self.timing.bpm_at_tick(tick)
    }

    /// 将实际时间 [`Duration`] 换算为最接近的脉冲位置。
    #[must_use]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        self.timing.duration_to_tick(duration)
    }
}

/// 在给定脉冲处快速查询生效 BPM 的轻量查找器。
///
/// 适用于需在构造 [`TimingTrack`] 之前查询 BPM 的场景（例如将 STP 毫秒
/// 换算为脉冲），以及任何无须完整计时轨的一次性/低频查询。
///
/// 与 [`TimingCache::bpm_at_tick`] 语义一致，但无需预建 `TimingCache`。
#[derive(Clone, Debug)]
pub struct BpmLookup<'a> {
    /// 脉冲 0 处的初始 BPM。
    init_bpm: f64,
    /// BPM 变更事件，按 `tick` 升序排列。
    bpm_changes: &'a [BpmChange],
}

impl BpmLookup<'_> {
    /// 构造 `BpmLookup`。
    ///
    /// `bpm_changes` 必须按 `tick` 升序排列（由调用方保证；
    /// [`BpmChange`] 本身无排序不变量）。
    #[must_use]
    pub const fn new(init_bpm: f64, bpm_changes: &[BpmChange]) -> BpmLookup<'_> {
        BpmLookup {
            init_bpm,
            bpm_changes,
        }
    }

    /// 返回 `tick` 处生效的 BPM（不晚于 `tick` 的最后一次 BPM 变更）。
    ///
    /// 当 `tick` 早于所有 BPM 变更时返回构造时传入的 `init_bpm`。
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "idx ≥ 1 由 match 分支保证，idx-1 必在界内"
    )]
    pub fn bpm_at_tick(&self, tick: u64) -> f64 {
        match self.bpm_changes.partition_point(|bc| bc.tick <= tick) {
            0 => self.init_bpm,
            idx => self.bpm_changes[idx - 1].bpm,
        }
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

/// 合并已排序的 BPM 变更与停止事件。
///
/// 两输入均假设已按 `tick` 升序排序；同一脉冲上 BPM 排在 Stop 之前
/// （"speed will first change, then the music pauses"）。
fn merge_sorted_events(bpm_changes: &[BpmChange], stops: &[StopEvent]) -> Vec<(u64, TimingEvent)> {
    let mut events = Vec::with_capacity(bpm_changes.len() + stops.len());
    let mut bpm_iter = bpm_changes.iter().peekable();
    let mut stop_iter = stops.iter().peekable();
    loop {
        match (bpm_iter.peek(), stop_iter.peek()) {
            (Some(bc), Some(st)) if bc.tick <= st.tick => {
                events.push((bc.tick, TimingEvent::Bpm(bc.bpm)));
                bpm_iter.next();
            }
            (Some(bc), None) => {
                events.push((bc.tick, TimingEvent::Bpm(bc.bpm)));
                bpm_iter.next();
            }
            (_, Some(st)) => {
                events.push((st.tick, TimingEvent::Stop(st.duration)));
                stop_iter.next();
            }
            (None, None) => break,
        }
    }
    events
}

/// 构建 `duration_to_tick` 的逆查找段列表。
///
/// 合并 BPM 变更与停止事件，按 `(tick, is_stop)` 升序遍历，累计实际时间
/// 秒数，在每个断点记录 `(sec_lo, tick, bpm)`。停止产生一个 `bpm = 0.0` 的
/// 冻结段，其后的线性段在同一脉冲以同 BPM 继续。
#[expect(clippy::cast_precision_loss, reason = "tick/duration fit in f64")]
fn merge_inv(
    init_bpm: f64,
    bpm_changes: &[BpmChange],
    stops: &[StopEvent],
    resolution: u64,
) -> Vec<InvSeg> {
    let res_f = resolution as f64;

    let events = merge_sorted_events(bpm_changes, stops);
    let mut inv = Vec::with_capacity(events.len() * 2 + 1);
    let mut cur_tick = 0u64;
    let mut cur_sec = 0.0f64;
    let mut cur_bpm = init_bpm;
    inv.push(InvSeg {
        sec_lo: 0.0,
        tick: 0,
        bpm: init_bpm,
    });

    for (tick, ev) in events {
        if tick > cur_tick {
            cur_sec += (tick - cur_tick) as f64 / res_f * 60.0 / cur_bpm.abs();
            cur_tick = tick;
        }
        match ev {
            TimingEvent::Bpm(b) => {
                if !is_valid_bpm(b) {
                    continue;
                }
                cur_bpm = b;
                inv.push(InvSeg {
                    sec_lo: cur_sec,
                    tick: cur_tick,
                    bpm: b,
                });
            }
            TimingEvent::Stop(d) => {
                let d_sec = d as f64 / res_f * 60.0 / cur_bpm.abs();
                inv.push(InvSeg {
                    sec_lo: cur_sec,
                    tick: cur_tick,
                    bpm: 0.0,
                });
                cur_sec += d_sec;
                inv.push(InvSeg {
                    sec_lo: cur_sec,
                    tick: cur_tick,
                    bpm: cur_bpm,
                });
            }
        }
    }
    inv
}

#[cfg(test)]
#[expect(deprecated, reason = "测试中仍使用 TimingCache 验证回归一致性")]
mod tests {
    use super::*;

    const RES: u64 = 240;

    // 基本功能（新 TimingTrack 直接测试）

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
    fn bpm_change_segment_boundary() {
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
    fn stop_strictly_before_target_adds_pause() {
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
    fn stop_at_target_excludes_pause() {
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
    fn matches_timing_track_constant_bpm() {
        let timing = TimingTrack::simple(150.0, RES).unwrap();
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 480, 960, 1920] {
            let expected = timing.tick_to_duration(tick);
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
            RES,
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 479, 480, 959, 960, 961, 1200, 2400] {
            let expected = timing.tick_to_duration(tick);
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
            RES,
        )
        .unwrap();

        assert!((timing.bpm_at_tick(0) - 120.0).abs() < 1e-9);
        assert!((timing.bpm_at_tick(480) - 200.0).abs() < 1e-9);
        assert!((timing.bpm_at_tick(960) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn unsorted_bpm_changes_sorted_internally() {
        let timing = TimingTrack::new(
            120.0,
            vec![
                BpmChange {
                    tick: 480,
                    bpm: 200.0,
                },
                BpmChange {
                    tick: 240,
                    bpm: 60.0,
                },
            ],
            vec![],
            RES,
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        let expected = Duration::from_millis(1500);
        assert_eq!(cache.tick_to_duration(480), expected);
        assert_eq!(timing.tick_to_duration(480), expected);
    }

    #[test]
    fn zero_bpm_change_skipped() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: 0.0,
            }],
            vec![],
            RES,
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        let expected = Duration::from_secs(1);
        assert_eq!(cache.tick_to_duration(480), expected);
        assert_eq!(timing.tick_to_duration(480), expected);
    }

    #[test]
    fn nan_bpm_change_skipped() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: f64::NAN,
            }],
            vec![],
            RES,
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        assert_eq!(cache.tick_to_duration(480), Duration::from_secs(1));
    }

    #[test]
    fn inf_bpm_change_skipped() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: f64::INFINITY,
            }],
            vec![],
            RES,
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        assert_eq!(cache.tick_to_duration(480), Duration::from_secs(1));
    }

    #[test]
    fn valid_bpm_after_invalid_one_works() {
        let timing = TimingTrack::new(
            120.0,
            vec![
                BpmChange {
                    tick: 240,
                    bpm: 0.0,
                },
                BpmChange {
                    tick: 480,
                    bpm: 60.0,
                },
            ],
            vec![],
            RES,
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        let expected = Duration::from_secs(2);
        assert_eq!(cache.tick_to_duration(720), expected);
        assert_eq!(timing.tick_to_duration(720), expected);
    }

    #[test]
    fn extreme_stop_does_not_panic() {
        let timing = TimingTrack::new(
            0.1,
            vec![],
            vec![StopEvent {
                tick: 0,
                duration: u64::MAX,
            }],
            RES,
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(1);
        assert_eq!(result, Duration::MAX);

        let result_track = timing.tick_to_duration(1);
        assert_eq!(result_track, Duration::MAX);
    }

    // 构造错误

    #[test]
    fn new_rejects_zero_init_bpm() {
        let result = TimingTrack::new(0.0, vec![], vec![], RES);
        assert_eq!(result, Err(TimingTrackError::InvalidBpm { bpm: 0.0 }));
    }

    #[test]
    fn new_accepts_negative_init_bpm() {
        let timing = TimingTrack::new(-120.0, vec![], vec![], RES).unwrap();
        assert!((timing.init_bpm() - (-120.0)).abs() < 1e-9);
    }

    #[test]
    fn new_rejects_nan_init_bpm() {
        let result = TimingTrack::new(f64::NAN, vec![], vec![], RES);
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_infinite_init_bpm() {
        let result = TimingTrack::new(f64::INFINITY, vec![], vec![], RES);
        assert!(result.is_err());
    }

    #[test]
    fn simple_rejects_invalid_bpm() {
        let result = TimingTrack::simple(0.0, RES);
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_zero_resolution() {
        let result = TimingTrack::new(120.0, vec![], vec![], 0);
        assert_eq!(result, Err(TimingTrackError::ZeroResolution));
    }

    #[test]
    fn simple_rejects_zero_resolution() {
        let result = TimingTrack::simple(120.0, 0);
        assert_eq!(result, Err(TimingTrackError::ZeroResolution));
    }

    // validate

    #[test]
    fn validate_returns_ok_for_valid_track() {
        let timing = TimingTrack::simple(120.0, RES).unwrap();
        assert!(timing.validate().is_ok());
    }

    #[test]
    fn validate_accepts_negative_bpm() {
        let timing = TimingTrack::simple(-120.0, RES).unwrap();
        assert!(timing.validate().is_ok());
    }

    #[test]
    fn validate_returns_err_for_invalid_track() {
        let timing = TimingTrack::simple(0.0, RES);
        assert!(timing.is_err());
    }

    // 回归测试：新旧结果一致性

    #[test]
    fn regression_tick_to_duration_matches_cache_constant() {
        let timing = TimingTrack::simple(150.0, RES).unwrap();
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 480, 960, 1920, 5000] {
            assert_eq!(
                timing.tick_to_duration(tick),
                cache.tick_to_duration(tick),
                "regression mismatch at tick {tick}",
            );
        }
    }

    #[test]
    fn regression_tick_to_duration_matches_cache_with_changes() {
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
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 480, 960, 1920, 5000] {
            assert_eq!(
                timing.tick_to_duration(tick),
                cache.tick_to_duration(tick),
                "regression mismatch at tick {tick}",
            );
        }
    }

    #[test]
    fn regression_duration_to_tick_matches_cache() {
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
        let cache = TimingCache::new(&timing, RES);

        for ms in (0..4000u64).step_by(50) {
            let d = Duration::from_millis(ms);
            assert_eq!(
                timing.duration_to_tick(d),
                cache.duration_to_tick(d),
                "regression mismatch at {ms}ms",
            );
        }
    }

    #[test]
    fn regression_roundtrip_consistency() {
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

        for tick in [0u64, 100, 240, 479, 480, 959, 960, 961, 1200, 2400] {
            let dur = timing.tick_to_duration(tick);
            let back = timing.duration_to_tick(dur);
            assert_eq!(
                back, tick,
                "roundtrip failed at tick {tick}: dur={dur:?}, back={back}",
            );
        }
    }
}
