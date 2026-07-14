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

/// `TimingTrack` 构造或验证时发生的错误。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TimingTrackError {
    /// 初始 BPM 无效（必须为非零有限值）。
    InvalidBpm {
        /// 无效的 BPM 值。
        bpm: f64,
    },
}

impl std::fmt::Display for TimingTrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBpm { bpm } => {
                write!(f, "invalid initial BPM: {bpm} (must be non-zero finite)")
            }
        }
    }
}

impl std::error::Error for TimingTrackError {}

impl TimingTrackError {
    /// 返回导致错误的 BPM 值。
    #[must_use]
    pub const fn bpm(&self) -> f64 {
        match self {
            Self::InvalidBpm { bpm } => *bpm,
        }
    }
}

/// 用于将脉冲位置换算为实际时间的计时信息。
///
/// 所有事件都在绝对脉冲位置上。处理器负责将格式特有的位置
/// （BMSON 脉冲、BMS 小节）换算为脉冲。
///
/// 合并的 BPM/停止事件列表在首次需要时惰性计算并缓存，避免重复构建。
///
/// # 构造后不可变性
///
/// `init_bpm`、`bpm_changes`、`stops` 构造后为只读（通过 getter 访问），
/// 以保证 `events_cache` 始终有效。修改应通过构建新 `TimingTrack` 完成。
#[derive(Debug)]
pub struct TimingTrack {
    /// 脉冲 0 处的初始 BPM。
    pub(crate) init_bpm: f64,
    /// BPM 变更事件，按脉冲升序排列。
    pub(crate) bpm_changes: Vec<BpmChange>,
    /// 停止（暂停）事件，按脉冲升序排列。
    pub(crate) stops: Vec<StopEvent>,
    /// 惰性缓存的合并事件列表（BPM 变更 + 停止，已排序）。
    events_cache: OnceLock<Vec<(u64, TimingEvent)>>,
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
    /// 使用构造函数而非直接构造结构体以确保内部缓存正确初始化。
    ///
    /// # Errors
    ///
    /// 若 `init_bpm` 无效（零、NaN 或无穷大），返回
    /// [`TimingTrackError::InvalidBpm`]。
    pub fn new(
        init_bpm: f64,
        bpm_changes: Vec<BpmChange>,
        stops: Vec<StopEvent>,
    ) -> Result<Self, TimingTrackError> {
        let track = Self {
            init_bpm,
            bpm_changes,
            stops,
            events_cache: OnceLock::new(),
        };
        track.validate()?;
        Ok(track)
    }

    /// 创建仅含初始 BPM、无 BPM 变更和无停止的计时轨。
    ///
    /// 等价于 `TimingTrack::new(bpm, vec![], vec![])`，但无需手动传递
    /// 空向量，在测试和简单场景中更简洁。
    ///
    /// # Errors
    ///
    /// 若 `init_bpm` 无效，返回 [`TimingTrackError::InvalidBpm`]。
    pub fn simple(init_bpm: f64) -> Result<Self, TimingTrackError> {
        Self::new(init_bpm, Vec::new(), Vec::new())
    }

    /// 由 `bpm_changes` 与停止事件构造已排序的事件列表。
    ///
    /// 第一次调用时构建并缓存结果，后续调用返回缓存引用。
    /// 同一脉冲上，BPM 变更排在停止之前（依据 BMSON 规范：
    /// "speed will first change, then the music pauses"），与
    /// [`Event::priority`](crate::Event::priority) 中 `Bpm(2) < Stop(3)`
    /// 的子序约定一致。
    fn cached_events(&self) -> &[(u64, TimingEvent)] {
        self.events_cache
            .get_or_init(|| build_sorted_events(&self.bpm_changes, &self.stops))
    }

    /// 将脉冲位置换算为实际时间 [`Duration`]。
    ///
    /// 在带有停止事件的脉冲上，返回的时间是暂停**之前**的时刻
    /// （依据 BMSON 规范，该脉冲上的音符在暂停之前激活）。
    /// 严格位于目标脉冲之前的停止贡献其完整暂停时长。
    ///
    /// # 复杂度与适用场景
    ///
    /// O(n) 线性扫描，零额外内存。适合一次性 / 低频查询
    /// （如 [`ChartData::duration`](crate::ChartData::duration)）。
    /// 高频查询（如播放器每帧）请用预计算的 [`TimingCache`]。
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
                    if is_valid_bpm(*bpm) {
                        current_bpm = *bpm;
                    }
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

        safe_from_secs_f64(seconds)
    }

    /// 将实际时间 [`Duration`] 换算为最近的脉冲位置。
    ///
    /// 这是 [`tick_to_duration`](Self::tick_to_duration) 的逆运算。
    /// 停止中消耗的时间不会推进脉冲。
    ///
    /// # 复杂度与适用场景
    ///
    /// O(n) 线性扫描。适合一次性 / 低频查询；高频查询请用
    /// [`TimingCache::duration_to_tick`]。
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
                    if is_valid_bpm(*bpm) {
                        current_bpm = *bpm;
                    }
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
    #[expect(
        clippy::expect_used,
        reason = "120 BPM is a hardcoded constant that is always valid"
    )]
    fn default() -> Self {
        Self::new(120.0, vec![], vec![]).expect("120 BPM is always valid")
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

// init_bpm 保证不含 NaN（构造时通过 TimingTrack::new 验证），因此
// PartialEq 满足 Eq 的反射性要求。
impl Eq for TimingTrack {}

// TimingCache —— 预计算计时索引

/// 计时轨中 BPM 恒定的一段。
#[derive(Debug)]
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
#[derive(Debug)]
struct InvSeg {
    /// 本段起始的实际时间（秒）。
    sec_lo: f64,
    /// `sec_lo` 处的脉冲位置。
    tick: u64,
    /// 本段 BPM；`0.0` 表示冻结段（停止期间脉冲不推进）。
    bpm: f64,
}

/// 预计算的计时索引，用于快速进行脉冲到 [`Duration`] 的换算。
///
/// 由 [`TimingTrack`] 构造，把每次查询从 O(n) 线性扫描降为 O(log n) 二分
/// 查找。适用于任何需要批量或频繁换算脉冲↔时间的场景：播放器实时查询、
/// 处理器离线切片等。
///
/// 与 [`TimingTrack`] 的转换方法是“简单实现 + 优化实现”的关系：
/// [`TimingTrack`] 的方法为 O(n) 直接实现（零额外内存，适合一次性查询），
/// 本类型预计算索引后为 O(log n)（以 O(n) 构建 + 额外内存换取查询加速）。
/// 两者语义一致（见 `matches_timing_track_*` 测试）。
///
/// 换算分为两部分：
///
/// 1. **基准时间** —— 在恒定 BPM 段内进行线性插值。
/// 2. **停止暂停** —— 由目标脉冲之前（不含）的所有停止累积的暂停时间。
///
/// 两部分均使用二分查找，每次查询为 O(log n)。语义与
/// [`TimingTrack::tick_to_duration`] / [`TimingTrack::duration_to_tick`]
/// 一致，是它们的加速版本。
#[derive(Debug)]
pub struct TimingCache {
    /// 按 `start_tick` 排序的 BPM 段。
    bpm_segments: Vec<BpmSegment>,
    /// 已排序的 `(stop_tick, cumulative_pause_seconds)` 配对。
    stop_cumsum: Vec<(u64, f64)>,
    /// `duration_to_tick` 的逆查找段（按 `sec_lo` 升序）。
    inv: Vec<InvSeg>,
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
        // 先按 tick 排序（与停止事件的处理一致），防止输入未排序时
        // current_seconds 累加出错。
        let mut sorted_bpm_changes = timing.bpm_changes.clone();
        sorted_bpm_changes.sort_by_key(|bc| bc.tick);

        let mut bpm_segments = vec![BpmSegment {
            start_tick: 0,
            start_seconds: 0.0,
            bpm: timing.init_bpm,
        }];

        let mut current_tick = 0u64;
        let mut current_seconds = 0.0f64;
        let mut current_bpm = timing.init_bpm;

        for bc in &sorted_bpm_changes {
            // 跳过无效 BPM 变更（0、NaN、无穷），保持当前 BPM 不变。
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
            inv: build_inv(
                timing.init_bpm,
                &timing.bpm_changes,
                &timing.stops,
                resolution,
            ),
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

        safe_from_secs_f64(base + stop_pause)
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
    /// 在预计算的逆查找段上执行二分查找，复杂度为 O(log n)。
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded result within u64 range"
    )]
    #[expect(clippy::cast_sign_loss, reason = "remaining time is non-negative")]
    #[expect(
        clippy::indexing_slicing,
        reason = "inv non-empty (always has initial seg); idx from saturating_sub on partition_point"
    )]
    #[must_use]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        let target = duration.as_secs_f64();
        if target <= 0.0 {
            return 0;
        }

        // 二分查找最后一个 sec_lo <= target 的段。
        let idx = self
            .inv
            .partition_point(|s| s.sec_lo <= target)
            .saturating_sub(1);
        let seg = &self.inv[idx];

        if seg.bpm == 0.0 {
            // 冻结段（停止内）：脉冲不推进。
            return seg.tick;
        }

        // 线性段内插值：dt 秒对应 dt*res*|bpm|/60 个脉冲。
        let dt = target - seg.sec_lo;
        seg.tick + (dt * self.resolution as f64 * seg.bpm.abs() / 60.0).round() as u64
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

/// 合并 BPM 变更与停止事件，按 `(tick, is_stop)` 升序排序。
///
/// 同一脉冲上 BPM 排在 Stop 之前（"speed will first change, then the music pauses"）。
fn build_sorted_events(bpm_changes: &[BpmChange], stops: &[StopEvent]) -> Vec<(u64, TimingEvent)> {
    let mut events: Vec<_> = bpm_changes
        .iter()
        .map(|bc| (bc.tick, TimingEvent::Bpm(bc.bpm)))
        .chain(
            stops
                .iter()
                .map(|st| (st.tick, TimingEvent::Stop(st.duration))),
        )
        .collect();
    events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.is_stop().cmp(&b.1.is_stop())));
    events
}

/// 构建 `duration_to_tick` 的逆查找段列表。
///
/// 合并 BPM 变更与停止事件，按 `(tick, is_stop)` 升序遍历（与
/// [`TimingTrack::cached_events`] 同序，保证同一脉冲上 BPM 先于 Stop），
/// 累计实际时间秒数，在每个断点记录 `(sec_lo, tick, bpm)`。停止产生一个
/// `bpm = 0.0` 的冻结段，其后的线性段在同一脉冲以同 BPM 继续。
#[expect(clippy::cast_precision_loss, reason = "tick/duration fit in f64")]
fn build_inv(
    init_bpm: f64,
    bpm_changes: &[BpmChange],
    stops: &[StopEvent],
    resolution: u64,
) -> Vec<InvSeg> {
    let res_f = resolution as f64;

    let events = build_sorted_events(bpm_changes, stops);
    let mut inv = Vec::with_capacity(events.len() * 2 + 1);
    let mut cur_tick = 0u64;
    let mut cur_sec = 0.0f64;
    let mut cur_bpm = init_bpm;
    // 初始线性段：脉冲 0 处以初始 BPM 开始。
    inv.push(InvSeg {
        sec_lo: 0.0,
        tick: 0,
        bpm: init_bpm,
    });

    for (tick, ev) in events {
        // 推进到 event_tick（当前线性段结束处）。
        if tick > cur_tick {
            cur_sec += (tick - cur_tick) as f64 / res_f * 60.0 / cur_bpm.abs();
            cur_tick = tick;
        }
        match ev {
            TimingEvent::Bpm(b) => {
                // 跳过无效 BPM 变更，保持当前 BPM 不变。
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
                // 冻结段：脉冲恒定，时间跳过 d_sec。
                inv.push(InvSeg {
                    sec_lo: cur_sec,
                    tick: cur_tick,
                    bpm: 0.0,
                });
                cur_sec += d_sec;
                // 停止后线性段在同一脉冲以同 BPM 继续。
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
mod tests {
    use super::*;

    const RES: u64 = 240;

    #[test]
    fn constant_bpm_tick_zero_is_zero() {
        let timing = TimingTrack::simple(120.0).unwrap();
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(0);
        assert_eq!(result, Duration::ZERO);
    }

    #[test]
    fn constant_bpm_120_one_beat_is_half_second() {
        let timing = TimingTrack::simple(120.0).unwrap();
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
        )
        .unwrap();
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
        )
        .unwrap();
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
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(240);
        assert_eq!(result, Duration::from_millis(500));
    }

    #[test]
    fn matches_timing_track_constant_bpm() {
        let timing = TimingTrack::simple(150.0).unwrap();
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
        )
        .unwrap();
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
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        assert!((cache.bpm_at_tick(0) - 120.0).abs() < 1e-9);
        assert!((cache.bpm_at_tick(480) - 200.0).abs() < 1e-9);
        assert!((cache.bpm_at_tick(960) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn unsorted_bpm_changes_sorted_internally() {
        // BPM 变更顺序为 tick 480 → 240（未排序）。
        // 若不排序，bpm_segments 的 current_seconds 累加会出错。
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
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        // 240 tick = 1 拍（resolution 240）。
        // tick 0-240 at 120 BPM = 0.5s，tick 240-480 at 60 BPM = 1.0s。
        let expected = Duration::from_millis(1500);
        assert_eq!(cache.tick_to_duration(480), expected);

        // 应与 TimingTrack（内部通过 cached_events 排序）一致。
        assert_eq!(timing.tick_to_duration(480, RES), expected);
    }

    #[test]
    fn zero_bpm_change_skipped() {
        // BPM 变更到 0.0 被跳过，保持 120 BPM。
        // 旧实现中 60.0 / 0.0 = inf，from_secs_f64 会 panic。
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: 0.0,
            }],
            vec![],
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        // tick 480 at 120 BPM = 1.0s（保持初始 BPM）。
        let expected = Duration::from_secs(1);
        assert_eq!(cache.tick_to_duration(480), expected);
        assert_eq!(timing.tick_to_duration(480, RES), expected);
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
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        // NaN BPM 被跳过，保持 120 BPM。
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
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        // Inf BPM 被跳过，保持 120 BPM。
        assert_eq!(cache.tick_to_duration(480), Duration::from_secs(1));
    }

    #[test]
    fn valid_bpm_after_invalid_one_works() {
        // 无效 BPM（0.0）被跳过后，后续有效 BPM 变更（60.0）应正常生效。
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
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        // tick 0-480 at 120 BPM = 1.0s，tick 480-720 at 60 BPM = 1.0s。
        let expected = Duration::from_secs(2);
        assert_eq!(cache.tick_to_duration(720), expected);
        assert_eq!(timing.tick_to_duration(720, RES), expected);
    }

    #[test]
    fn extreme_stop_does_not_panic() {
        // 极长停止（u64::MAX 脉冲）+ 极低 BPM（0.1）产生超大秒数，
        // 超出 Duration::MAX。旧实现中 from_secs_f64 会 panic。
        // u64::MAX / 240 * 60 / 0.1 ≈ 4.6e19 > Duration::MAX (1.8e19)。
        let timing = TimingTrack::new(
            0.1,
            vec![],
            vec![StopEvent {
                tick: 0,
                duration: u64::MAX,
            }],
        )
        .unwrap();
        let cache = TimingCache::new(&timing, RES);

        // 应返回 Duration::MAX 而非 panic。
        let result = cache.tick_to_duration(1);
        assert_eq!(result, Duration::MAX);

        let result_track = timing.tick_to_duration(1, RES);
        assert_eq!(result_track, Duration::MAX);
    }

    #[test]
    fn new_rejects_zero_init_bpm() {
        let result = TimingTrack::new(0.0, vec![], vec![]);
        assert_eq!(result, Err(TimingTrackError::InvalidBpm { bpm: 0.0 }));
    }

    #[test]
    fn new_accepts_negative_init_bpm() {
        // 负 BPM 用于逆向滚动（逆走），时序计算使用 |bpm|。
        let timing = TimingTrack::new(-120.0, vec![], vec![]).unwrap();
        assert!((timing.init_bpm() - (-120.0)).abs() < 1e-9);
    }

    #[test]
    fn new_rejects_nan_init_bpm() {
        let result = TimingTrack::new(f64::NAN, vec![], vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_infinite_init_bpm() {
        let result = TimingTrack::new(f64::INFINITY, vec![], vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn simple_rejects_invalid_bpm() {
        let result = TimingTrack::simple(0.0);
        assert!(result.is_err());
    }

    #[test]
    fn validate_returns_ok_for_valid_track() {
        let timing = TimingTrack::simple(120.0).unwrap();
        assert!(timing.validate().is_ok());
    }

    #[test]
    fn validate_accepts_negative_bpm() {
        let timing = TimingTrack::simple(-120.0).unwrap();
        assert!(timing.validate().is_ok());
    }

    #[test]
    fn validate_returns_err_for_invalid_track() {
        // 通过直接构造绕过 new() 的验证来测试 validate()。
        let timing = TimingTrack {
            init_bpm: 0.0,
            bpm_changes: vec![],
            stops: vec![],
            events_cache: std::sync::OnceLock::new(),
        };
        assert_eq!(
            timing.validate(),
            Err(TimingTrackError::InvalidBpm { bpm: 0.0 })
        );
    }
}
