//! 音乐游戏谱面的纯仿真层。
//!
//! [`Player<T, C>`] 包装一个 [`Chart<T, C>`]，提供基于时间的查询接口，
//! 供游戏渲染与调度使用。它是纯仿真层，无 I/O、渲染、判定或计分。
//!
//! # 用法
//!
//! ```
//! use std::num::NonZeroU8;
//! use std::time::Duration;
//! use bmsrs_chart::{
//!     Chart, SongInfo, ChartInfo, ChartData, Event, EventKind, Lane, LnJudgeHint, LnLifeHint,
//!     LnTypeHint, NoteKind, NoteSide, TimingTrack,
//! };
//! use bmsrs_player::Player;
//!
//! let chart: Chart = Chart {
//!     song: SongInfo::default(),
//!     chart: ChartInfo::default(),
//!     data: ChartData {
//!         resolution: 240,
//!         timing: TimingTrack::simple(120.0).unwrap(),
//!         judge_multiplier: 1.0,
//!         life_multiplier: 1.0,
//!         ln_type_hint: LnTypeHint::default(),
//!         ln_judge_hint: LnJudgeHint::default(),
//!         ln_life_hint: LnLifeHint::default(),
//!         judge_deltas: None,
//!         life_deltas: None,
//!         events: vec![Event::new(
//!             480,
//!             EventKind::Note {
//!                 side: NoteSide::P1,
//!                 lane: Lane::Key(NonZeroU8::new(1).unwrap()),
//!                 kind: NoteKind::Normal,
//!                 audio_index: None,
//!                 ext: (),
//!             },
//!         )],
//!         audio_assets: vec![],
//!     },
//! };
//!
//! let mut player = Player::new(chart).unwrap();
//! assert_eq!(player.current_tick(), 0);
//! player.advance(Duration::from_secs(1));
//! assert_eq!(player.current_time(), Duration::from_secs(1));
//! ```

pub(crate) mod scroll_cache;
mod speed_cache;

use std::ops::Bound;
use std::ops::RangeBounds;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BgaResource, Chart, ChartDataError, CustomEvent, Event, EventKind, Lane,
    NoCustomEvent, NoteExt, NoteKind, NoteSide, TimingCache,
};

use crate::scroll_cache::ScrollCache;
use crate::speed_cache::SpeedCache;

/// 跟踪 [`Chart`] 播放进度的有状态仿真器。
///
/// 播放器维护当前脉冲位置，提供对给定范围内音符、BGM 与视觉事件的查询。
/// 所有时间推进均以实际时间 [`Duration`] 表示；脉冲位置通过内部预计算的
/// 计时缓存派生得到。
///
/// 播放器是纯仿真层：无音频播放、渲染、输入处理、判定或计分。
pub struct Player<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// 正在播放的谱面。
    chart: Chart<T, C>,
    /// 预计算的计时缓存，用于 O(log n) 查询。
    cache: TimingCache,
    /// 预计算的滚动速度缓存，用于 O(log n) 查询。
    scroll_cache: ScrollCache,
    /// 预计算的 SPEED 间距插值缓存。
    speed_cache: SpeedCache,
    /// 当前播放位置（脉冲）。
    current_tick: u64,
}

impl<T: NoteExt, C: CustomEvent> Player<T, C> {
    /// 由谱面创建一个新播放器，从脉冲 0 开始。
    ///
    /// 内部自动校验谱面数据并排序事件——调用方无需预处理。
    ///
    /// # Errors
    ///
    /// 若谱面数据不合法（`resolution == 0` 或初始 BPM 零/非有限值），
    /// 返回 [`ChartDataError`]。
    pub fn new(mut chart: Chart<T, C>) -> Result<Self, ChartDataError> {
        chart.data.validate()?;
        chart.data.sort_events();
        let resolution = chart.data.resolution;
        let cache = TimingCache::new(&chart.data.timing, resolution);
        let scroll_cache = ScrollCache::build(&chart.data.events, resolution);
        let speed_cache = SpeedCache::build(&chart.data.events);
        Ok(Self {
            chart,
            cache,
            scroll_cache,
            speed_cache,
            current_tick: 0,
        })
    }

    // 时间控制

    /// 按实际时间增量 `delta` 推进播放。
    ///
    /// 播放器的脉冲位置更新为 `current_time + delta` 所对应的脉冲。
    /// 停止（STOP）期间的时间不会推进脉冲。
    pub fn advance(&mut self, delta: Duration) {
        let new_time = self.current_time() + delta;
        // PERF: 走预计算的 TimingCache（O(log n) 二分）而非 TimingTrack 的
        // O(n) 线性扫描——advance 在播放循环中每帧调用，是真正的热路径。
        // 两者语义已由 chart crate 的等价性测试保证一致。
        self.current_tick = self.cache.duration_to_tick(new_time);
    }

    /// 跳转到指定的绝对实际时间。
    pub fn seek(&mut self, target: Duration) {
        // PERF: 同 advance，使用 TimingCache 的 O(log n) 快路径。
        self.current_tick = self.cache.duration_to_tick(target);
    }

    /// 将播放重置到脉冲 0。
    pub const fn reset(&mut self) {
        self.current_tick = 0;
    }

    // 时间查询

    /// 当前播放位置（脉冲）。
    #[must_use]
    pub const fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// 以实际时间 [`Duration`] 表示的当前播放位置。
    #[must_use]
    pub fn current_time(&self) -> Duration {
        self.cache.tick_to_duration(self.current_tick)
    }

    /// 使用缓存的计时数据将脉冲位置换算为实际时间 [`Duration`]。
    ///
    /// 比直接调用
    /// [`TimingTrack::tick_to_duration`](bmsrs_chart::TimingTrack::tick_to_duration)
    /// 更快：使用 O(log n) 二分查找而非 O(n) 遍历。
    #[must_use]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
        self.cache.tick_to_duration(tick)
    }

    /// 将实际时间 [`Duration`] 换算为最接近的脉冲位置。
    ///
    /// 使用预计算的计时缓存，性能为 O(log n)。
    #[must_use]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        self.cache.duration_to_tick(duration)
    }

    /// 谱面总时长。
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.chart.data.duration()
    }

    /// 播放位置处的当前 BPM。
    #[must_use]
    pub fn current_bpm(&self) -> f64 {
        self.cache.bpm_at_tick(self.current_tick)
    }

    // 事件查询

    /// 计算可见时间窗口对应的脉冲范围。
    ///
    /// `reaction` 为判定线下方的可见历史时长，`lookahead` 为判定线上方的
    /// 预见未来时长。返回 `(start_tick, end_tick)`，可直接传给
    /// [`events_in_range`](Self::events_in_range)。
    ///
    /// 内部执行两次 [`TimingCache::duration_to_tick`]（O(log n)），
    /// 封装了 `current_time ± duration → tick` 的换算逻辑。
    #[must_use]
    pub fn visible_tick_range(&self, reaction: Duration, lookahead: Duration) -> (u64, u64) {
        let current = self.current_time();
        let start = current.checked_sub(reaction).unwrap_or(Duration::ZERO);
        let end = current.checked_add(lookahead).unwrap_or(Duration::MAX);
        (
            self.cache.duration_to_tick(start),
            self.cache.duration_to_tick(end),
        )
    }

    /// 返回 `range` 范围内的全部事件。
    ///
    /// 事件按脉冲升序排列（由谱面保证）。
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    #[must_use]
    pub fn events_in_range(&self, range: impl RangeBounds<u64>) -> &[Event<T, C>] {
        let (start, end) = self.event_range_indices(range);
        &self.chart.data.events[start..end]
    }

    /// 返回 `range` 范围内 Note 事件的迭代器。
    pub fn notes_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Note { .. }))
    }

    /// 返回 `range` 范围内位于 `(side, lane)` 的 Note 事件的迭代器。
    pub fn notes_in_lane(
        &self,
        side: NoteSide,
        lane: Lane,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.notes_in_range(range).filter(
            move |e| matches!(e.kind, EventKind::Note { side: s, lane: l, .. } if s == side && l == lane),
        )
    }

    /// 返回 `range` 范围内与判定相关的 Note 事件的迭代器。
    ///
    /// 与判定相关的音符为普通音符与长音（不含不可见音符与地雷，
    /// 它们的处理方式不同）。
    pub fn notes_for_judgement(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.notes_in_range(range).filter(|e| {
            matches!(
                e.kind,
                EventKind::Note {
                    kind: NoteKind::Normal | NoteKind::Long { .. },
                    ..
                }
            )
        })
    }

    /// 返回 `range` 范围内 BGM 事件的迭代器。
    pub fn bgm_in_range(&self, range: impl RangeBounds<u64>) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Bgm { .. }))
    }

    /// 返回音频素材表。
    #[must_use]
    pub fn audio_assets(&self) -> &[AudioAsset] {
        &self.chart.data.audio_assets
    }

    // 视觉查询

    /// 返回 `tick` 处的滚动速度倍率。
    ///
    /// 若同一脉冲上存在多个滚动事件，取最后一个生效。
    /// 若此前未发生任何滚动事件，返回 `1.0`。
    ///
    /// 使用预计算的滚动速度缓存，查询复杂度为 O(log n)。
    #[must_use]
    pub fn scroll_rate_at(&self, tick: u64) -> f64 {
        self.scroll_cache.rate_at(tick)
    }

    /// 返回 `tick` 处的累积滚动位置（以节拍为单位）。
    ///
    /// 位置 = 滚动速度对时间的积分。默认无 SCROLL 事件时，
    /// 位置 = `tick / resolution`（标准节拍对齐）。
    #[must_use]
    pub fn scroll_position_at(&self, tick: u64) -> f64 {
        self.scroll_cache.position_at(tick)
    }

    /// 返回 `tick` 处的 SPEED 插值间距倍率。
    ///
    /// 在相邻 `EventKind::Speed` 关键帧之间执行线性插值。
    /// 首个关键帧之前 → `1.0`；最后一个之后 → 最后一个关键帧的值。
    #[must_use]
    pub fn spacing_at(&self, tick: u64) -> f64 {
        self.speed_cache.spacing_at(tick)
    }

    /// 返回 `range` 范围内 Bar 事件的迭代器。
    pub fn bar_lines_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Bar))
    }

    /// 返回 `range` 范围内 BGA 事件的迭代器。
    ///
    /// 如有需要，调用方可进一步按 [`bmsrs_chart::BgaLayer`] 过滤。
    pub fn bga_events_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Bga { .. }))
    }

    /// 返回 BGA 资源。
    #[must_use]
    pub fn bga_resources(&self) -> &[BgaResource] {
        &self.chart.chart.bga_resources
    }

    // 谱面访问

    /// 借用底层谱面。
    #[must_use]
    pub const fn chart(&self) -> &Chart<T, C> {
        &self.chart
    }

    /// 消费播放器并返回谱面。
    #[must_use]
    pub fn into_chart(self) -> Chart<T, C> {
        self.chart
    }

    // 私有辅助

    /// 将 [`RangeBounds<u64>`] 映射为 `self.chart.data.events` 中的
    /// `(起始下标, 结束下标)`。
    fn event_range_indices(&self, range: impl RangeBounds<u64>) -> (usize, usize) {
        let start_tick = match range.start_bound() {
            Bound::Included(t) => *t,
            Bound::Excluded(t) => t.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end_tick = match range.end_bound() {
            Bound::Included(t) => t.saturating_add(1),
            Bound::Excluded(t) => *t,
            Bound::Unbounded => u64::MAX,
        };
        let start = self
            .chart
            .data
            .events
            .partition_point(|e| e.tick() < start_tick);
        let end = self
            .chart
            .data
            .events
            .partition_point(|e| e.tick() < end_tick);
        (start, end)
    }
}
