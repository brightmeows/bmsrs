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
//!     Chart, SongInfo, ChartInfo, ChartData, Event, Lane, NoteKind, NoteSide,
//!     TimingTrack,
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
//!         timing: TimingTrack::new(120.0, vec![], vec![]),
//!         events: vec![Event::Note {
//!             tick: 0,
//!             side: NoteSide::P1,
//!             lane: Lane::Key(NonZeroU8::new(1).unwrap()),
//!             kind: NoteKind::Normal,
//!             audio_index: None,
//!             ext: (),
//!         }],
//!         audio_assets: vec![],
//!         ..Default::default()
//!     },
//! };
//! assert_eq!(chart.song.title, "Test");
//! assert_eq!(chart.data.events.len(), 1);
//! ```

pub mod audio;
pub mod event;
pub mod mode;
pub mod note;
pub mod timing;
pub mod visual;

pub use audio::AudioAsset;
pub use event::{CustomEvent, Event, NoCustomEvent, NoteExt};
pub use mode::{Lane, NoteSide};
pub use note::{Damage, NoteKind};
pub use timing::{BpmChange, StopEvent, TimingCache, TimingTrack};
pub use visual::{BgaLayer, BgaResource};

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
}

/// 游玩数据 —— 对应 BMSON v2 的 `ChartData`。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartData<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    // 手动实现 Eq：judge_multiplier 与 life_multiplier 保证不含 NaN。
    /// 每个四分音符的脉冲数（节拍分辨率）。
    pub resolution: u64,
    /// 用于脉冲 ↔ 秒换算的计时轨。
    pub timing: TimingTrack,
    /// 判定窗口倍率（`1.0` = 标准）。
    pub judge_multiplier: f64,
    /// 血量槽倍率（`1.0` = 标准）。
    pub life_multiplier: f64,
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
        self.timing
            .tick_to_duration(self.last_tick(), self.resolution)
    }
}

/// 顶层谱面 —— 对应 BMSON v2 的 `Bmson` 根对象。
///
/// 包含乐曲元数据、谱面元数据与游玩数据。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chart<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// 乐曲级元数据（标题、艺术家、流派）。
    pub song: SongInfo,
    /// 谱面级元数据与资源。
    pub chart: ChartInfo,
    /// 游玩数据（计时、事件、音频）。
    pub data: ChartData<T, C>,
}

impl<T: NoteExt + Eq, C: CustomEvent + Eq> Eq for Chart<T, C> {}
