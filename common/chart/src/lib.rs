//! 格式无关的音乐游戏谱面数据模型。
//!
//! [`Chart`] 是核心类型：由格式处理器（`bmson-processor`、`bms-processor`）
//! 产生的中间表示，供播放器（`bmsrs-player`）消费。
//!
//! # 结构
//!
//! 谱面拆分为三个子结构体，与 BMSON v2 格式对应：
//!
//! | 组件 | BMSON v2 对应 | 内容 |
//! |------|--------------|------|
//! | [`SongInfo`] | `SongInfo` | 乐曲级元数据（标题、艺术家、流派）|
//! | [`ChartInfo`] | `ChartInfo` | 谱面级元数据 + BGA 资源 |
//! | [`ChartData`] | `ChartData` | 游玩数据（计时、事件、音频）|
//!
//! # 时间模型
//!
//! 所有事件位置均为绝对 [`u64`] 脉冲（tick）。全局
//! [`resolution`](ChartData::resolution) 定义每个四分音符的脉冲数
//! （默认 240）。实际时间（wall-clock）通过
//! [`TimingTrack::tick_to_duration`] 换算得到。
//!
//! # 统一事件时间线
//!
//! 所有计时事件存放在单一的 [`events`](ChartData::events) 向量中，
//! 按脉冲排序。[`Event`] 枚举的每个变体代表一种事件
//! （音符、BGM、BPM 变更、停止、滚动、BGA、小节线或格式特有的自定义事件）。
//!
//! # 泛型参数
//!
//! - `T: NoteExt` —— 每音符扩展数据（默认 `()`）。
//! - `C: CustomEvent` —— 格式特有的自定义事件类型
//!   （默认 [`NoCustomEvent`]）。
//!
//! # Example
//!
//! ```
//! use std::num::NonZeroU8;
//! use bmsrs_chart::{
//!     Chart, SongInfo, ChartInfo, ChartData, Event, EventKind, Lane, LnJudgeHint, LnLifeHint,
//!     LnTypeHint, NoteKind, NoteSide, TimingTrack,
//! };
//!
//! let chart: Chart = Chart {
//!     song: SongInfo {
//!         title: "Test".into(),
//!         ..Default::default()
//!     },
//!     chart: ChartInfo::default(),
//!     data: ChartData {
//!         resolution: 240,
//!         timing: TimingTrack::simple(120.0, 240).unwrap(),
//!         judge_multiplier: 1.0,
//!         life_multiplier: 1.0,
//!         ln_type_hint: LnTypeHint::default(),
//!         ln_judge_hint: LnJudgeHint::default(),
//!         ln_life_hint: LnLifeHint::default(),
//!         judge_deltas: None,
//!         life_deltas: None,
//!         events: vec![Event::new(
//!             0,
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
//! assert_eq!(chart.song.title, "Test");
//! assert_eq!(chart.data.events.len(), 1);
//! ```

use std::fmt;

pub mod audio;
pub mod event;
pub mod mode;
pub mod note;
pub mod timing;
pub mod visual;

pub use audio::AudioAsset;
pub use event::{CustomEvent, Event, EventKind, NoCustomEvent, NoteExt};
pub use mode::{Lane, NoteSide};
pub use note::{Damage, LnJudgeHint, LnLifeHint, LnTypeHint, NoteKind};
#[expect(deprecated, reason = "TimingCache 仍保留供旧调用方使用")]
pub use timing::TimingCache;
pub use timing::{BpmChange, BpmLookup, StopEvent, TimingTrack, TimingTrackError};
pub use visual::{BgaLayer, BgaResource, CropRect, VideoAsset};

/// 乐曲级元数据 —— 对应 BMSON v2 的 `SongInfo`。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SongInfo {
    /// 乐曲标题。
    pub title: String,
    /// 主要艺术家。
    pub artist: String,
    /// 乐曲流派。
    pub genre: String,
    /// 其他贡献者（例如 `["music:composer", "chart:charter"]`）。
    pub subartists: Vec<String>,
}

/// 谱面级元数据与资源 —— 对应 BMSON v2 的 `ChartInfo`。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChartInfo {
    /// 乐曲副标题（可为空）。
    pub subtitle: String,
    /// 谱面名称 / 难度标签（例如 `"HYPER"`、`"ANOTHER"`）。
    pub chart_name: String,
    /// 数值难度等级（beat 模式通常为 1–12）。
    pub level: u64,
    /// 背景图片路径（游玩时显示）。
    pub back_image: Option<String>,
    /// 过场图片路径（加载时显示）。
    pub eyecatch_image: Option<String>,
    /// 横幅图片路径（选曲时显示）。
    pub banner_image: Option<String>,
    /// 预览音频路径（选曲时的短音频片段）。
    pub preview_music: Option<String>,
    /// BGA 资源声明（图片 / 视频文件）。
    pub bga_resources: Vec<BgaResource>,
    /// 背景视频（BMS `#VIDEOFILE` / `#MOVIE`）。BMSON 无此概念，为 `None`。
    pub video: Option<VideoAsset>,
}

/// 自定义判定窗口偏移（DJ.NEXT 扩展）。
///
/// 每个字段指定该判定等级窗口的额外偏移（毫秒）。源自 BMSON 的
/// `judgement_deltas`，BMS 谱面无此概念（处理器填 `None`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JudgementDeltas {
    /// PERFECT 窗口的额外偏移（毫秒）。
    pub perfect: u64,
    /// GREAT 窗口的额外偏移（毫秒）。
    pub great: u64,
    /// GOOD 窗口的额外偏移（毫秒）。
    pub good: u64,
    /// MISS 窗口的额外偏移（毫秒）。
    pub miss: u64,
}

/// 自定义血量槽增量（DJ.NEXT 扩展）。
///
/// 每个字段指定该判定等级的血量变化（百分比），负值表示扣除。
/// 源自 BMSON 的 `life_deltas`，BMS 谱面无此概念（处理器填 `None`）。
///
/// 保证不含 NaN（BMSON 经 serde 反序列化，JSON 不含 NaN）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LifeDeltas {
    // 手动实现 Eq：保证不含 NaN。
    /// PERFECT 时的血量变化（百分比，可为负）。
    pub perfect: f64,
    /// GREAT 时的血量变化（百分比，可为负）。
    pub great: f64,
    /// GOOD 时的血量变化（百分比，可为负）。
    pub good: f64,
    /// MISS 时的血量变化（百分比，可为负）。
    pub miss: f64,
}

// LifeDeltas 保证不含 NaN，故可安全实现 Eq。
impl Eq for LifeDeltas {}

/// `ChartData` 验证错误类型。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ChartDataError {
    /// `resolution` 为 0（必须为正）。
    ZeroResolution,
    /// 初始 BPM 无效（0、NaN 或无穷大；允许负值用于逆走谱面）。
    InvalidBpm {
        /// 无效的 BPM 值。
        bpm: f64,
    },
}

impl fmt::Display for ChartDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroResolution => write!(f, "resolution must be positive, got 0"),
            Self::InvalidBpm { bpm } => {
                write!(f, "invalid initial BPM: {bpm} (must be non-zero finite)")
            }
        }
    }
}

impl std::error::Error for ChartDataError {}

/// 游玩数据 —— 对应 BMSON v2 的 `ChartData`。
///
/// 不实现 [`Default`]：合法状态要求 `resolution > 0` 且
/// [`timing`](TimingTrack) 的初始 BPM 为正，没有有意义的零值默认。
/// 调用方必须显式提供这些值（各处理器均以字面量构造）。
#[derive(Clone, Debug, PartialEq)]
pub struct ChartData<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// 每个四分音符的脉冲数（节拍分辨率）。
    pub resolution: u64,
    /// 用于脉冲 ↔ 秒换算的计时轨。
    pub timing: TimingTrack,
    /// 判定窗口倍率（`1.0` = 标准）。
    pub judge_multiplier: f64,
    /// 血量槽倍率（`1.0` = 标准）。
    pub life_multiplier: f64,
    /// 谱面级长音类型提示（BMSON LN/CN）。
    pub ln_type_hint: LnTypeHint,
    /// 谱面级长音判定提示。
    pub ln_judge_hint: LnJudgeHint,
    /// 谱面级长音血量提示。
    pub ln_life_hint: LnLifeHint,
    /// 自定义判定窗口偏移（DJ.NEXT 扩展，BMS 为 `None`）。
    pub judge_deltas: Option<JudgementDeltas>,
    /// 自定义血量槽增量（DJ.NEXT 扩展，BMS 为 `None`）。
    pub life_deltas: Option<LifeDeltas>,
    /// 全部计时事件，按脉冲升序排列。
    pub events: Vec<Event<T, C>>,
    /// 音符与 BGM 事件引用的音频素材。
    pub audio_assets: Vec<AudioAsset>,
}

// judge_multiplier 与 life_multiplier 保证不含 NaN。
impl<T: NoteExt + Eq, C: CustomEvent + Eq> Eq for ChartData<T, C> {}

impl<T: NoteExt, C: CustomEvent> ChartData<T, C> {
    /// 返回谱面数据中最后一个事件的脉冲位置。
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        self.events.last().map_or(0, Event::tick)
    }

    /// 返回谱面数据的总时长。
    #[must_use]
    #[inline]
    pub fn duration(&self) -> std::time::Duration {
        self.timing.tick_to_duration(self.last_tick())
    }

    /// 验证谱面数据的关键不变量。
    ///
    /// 检查项：
    /// - `resolution > 0`
    /// - `timing` 的初始 BPM 有效（委托 [`TimingTrack::validate`]，
    ///   允许负 BPM 用于逆走谱面）
    ///
    /// # Errors
    ///
    /// 若任何检查失败，返回 [`ChartDataError`]。
    pub fn validate(&self) -> Result<(), ChartDataError> {
        if self.resolution == 0 {
            return Err(ChartDataError::ZeroResolution);
        }
        self.timing.validate().map_err(|e| match e {
            TimingTrackError::InvalidBpm { bpm } => ChartDataError::InvalidBpm { bpm },
            TimingTrackError::ZeroResolution => ChartDataError::ZeroResolution,
        })
    }

    /// 确保事件按脉冲升序排列。
    ///
    /// [`Player`] 的二分查找（`partition_point`）依赖事件已排序。
    /// 构造后调用一次以保证后续查询正确。
    ///
    /// 排序键为 [`Event::sort_key`]：同脉冲内按优先级排序
    /// （Bar < Note/BGA/BGM < BPM < Stop < Scroll < Speed < Custom）。
    /// 稳定排序保留同优先级的插入顺序。
    ///
    /// [`Player`]: https://docs.rs/bmsrs-player/latest/bmsrs_player/struct.Player.html
    pub fn sort_events(&mut self) {
        self.events.sort_by_key(Event::sort_key);
    }

    /// 通过映射函数转换所有事件，返回新的 `ChartData`，其余字段不变。
    ///
    /// `NoteExt` 类型可改变（例如从 `BmsonNoteExt` 剥离为 `()`），
    /// `CustomEvent` 类型保持不变。
    ///
    /// 若需同时改变 `CustomEvent` 类型或过滤事件，请用
    /// [`filter_map_events`](Self::filter_map_events)。
    ///
    /// # 示例
    ///
    /// ```rust
    /// # use bmsrs_chart::{ChartData, Event, NoteExt, NoCustomEvent, EventKind};
    /// # fn example(data: ChartData<(), NoCustomEvent>) {
    /// let mapped: ChartData<(), NoCustomEvent> = data.map_events(|e| {
    ///     Event::new(e.tick(), e.kind.map_ext(|_| ()))
    /// });
    /// # }
    /// ```
    #[must_use]
    pub fn map_events<U: NoteExt>(
        self,
        f: impl FnMut(Event<T, C>) -> Event<U, C>,
    ) -> ChartData<U, C> {
        ChartData {
            resolution: self.resolution,
            timing: self.timing,
            judge_multiplier: self.judge_multiplier,
            life_multiplier: self.life_multiplier,
            ln_type_hint: self.ln_type_hint,
            ln_judge_hint: self.ln_judge_hint,
            ln_life_hint: self.ln_life_hint,
            judge_deltas: self.judge_deltas,
            life_deltas: self.life_deltas,
            events: self.events.into_iter().map(f).collect(),
            audio_assets: self.audio_assets,
        }
    }

    /// 通过映射+过滤函数转换所有事件，可同时改变 `T` 和 `C` 两个泛型参数。
    ///
    /// 返回 [`None`] 的事件被丢弃。其余字段不变。
    ///
    /// 与 [`map_events`](Self::map_events) 的区别：
    /// - `map_events` 仅改变 `T: NoteExt`，保留 `C: CustomEvent` 不变；
    /// - `filter_map_events` 可同时改变两者，且支持丢弃事件。
    ///
    /// 典型场景：将 `Chart<T, BmsCustomEvent>` 转换为 `Chart<U, NoCustomEvent>`，
    /// 同时丢弃所有 `Custom` 事件。
    ///
    /// # 示例
    ///
    /// ```rust
    /// # use bmsrs_chart::{
    /// #     ChartData, Event, EventKind, NoCustomEvent, NoteExt,
    /// #     CustomEvent,
    /// # };
    /// # fn example(data: ChartData<(), NoCustomEvent>) {
    /// // 丢弃 Custom 事件，将其余事件的 C 类型转换为 NoCustomEvent
    /// let mapped: ChartData<(), NoCustomEvent> = data.filter_map_events(|e| {
    ///     let tick = e.tick();
    ///     match e.kind {
    ///         EventKind::Custom(_) => None,
    ///         kind => Some(Event::new(tick, kind.map_custom(|_| NoCustomEvent))),
    ///     }
    /// });
    /// # }
    /// ```
    #[must_use]
    pub fn filter_map_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> ChartData<U, D> {
        ChartData {
            resolution: self.resolution,
            timing: self.timing,
            judge_multiplier: self.judge_multiplier,
            life_multiplier: self.life_multiplier,
            ln_type_hint: self.ln_type_hint,
            ln_judge_hint: self.ln_judge_hint,
            ln_life_hint: self.ln_life_hint,
            judge_deltas: self.judge_deltas,
            life_deltas: self.life_deltas,
            events: self.events.into_iter().filter_map(f).collect(),
            audio_assets: self.audio_assets,
        }
    }
}

/// 顶层谱面 —— 对应 BMSON v2 的 `Bmson` 根对象。
///
/// 包含乐曲元数据、谱面元数据与游玩数据。
///
/// 不实现 [`Default`]，因为内嵌的 [`ChartData`] 无合法默认。
/// 乐曲级与谱面级元数据（[`SongInfo`]、[`ChartInfo`]）仍实现 [`Default`]。
#[derive(Clone, Debug, PartialEq)]
pub struct Chart<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// 乐曲级元数据（标题、艺术家、流派）。
    pub song: SongInfo,
    /// 谱面级元数据与资源。
    pub chart: ChartInfo,
    /// 游玩数据（计时、事件、音频）。
    pub data: ChartData<T, C>,
}

impl<T: NoteExt + Eq, C: CustomEvent + Eq> Eq for Chart<T, C> {}

impl<T: NoteExt, C: CustomEvent> Chart<T, C> {
    /// 通过映射函数转换所有事件，返回新的 `Chart`，其余字段不变。
    ///
    /// 见 [`ChartData::map_events`]。
    #[must_use]
    pub fn map_events<U: NoteExt>(self, f: impl FnMut(Event<T, C>) -> Event<U, C>) -> Chart<U, C> {
        Chart {
            song: self.song,
            chart: self.chart,
            data: self.data.map_events(f),
        }
    }

    /// 通过映射+过滤函数转换所有事件，可同时改变 `T` 和 `C` 两个泛型参数。
    ///
    /// 见 [`ChartData::filter_map_events`]。
    #[must_use]
    pub fn filter_map_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> Chart<U, D> {
        Chart {
            song: self.song,
            chart: self.chart,
            data: self.data.filter_map_events(f),
        }
    }
}
