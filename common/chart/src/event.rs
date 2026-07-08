//! 统一事件类型与格式扩展 trait。
//!
//! 所有计时事件存放在单一的 `Vec<Event<T, C>>` 中，按脉冲排序，
//! 支持统一迭代与二分查找。
//!
//! [`Event`] 将脉冲位置（`tick`）与事件种类（[`EventKind`]）分离，
//! 避免每个事件变体重复定义 `tick`。
//!
//! 泛型参数：
//! - `T` —— 每音符扩展数据（携带于 [`EventKind::Note`]）。
//! - `C` —— 格式特有的自定义事件类型（携带于 [`EventKind::Custom`]）。
//!
//! # 排序
//!
//! 处理器按 `(tick, priority)` 对事件排序。[`EventKind::priority`] 定义了
//! 同一脉冲上各事件的顺序：
//!
//! `Bar(0) → Note/BGA/BGM(1) → BPM(2) → Stop(3) → Scroll(4) → Speed(5) → Custom(6)`

use std::fmt::Debug;

use crate::mode::{Lane, NoteSide};
use crate::note::NoteKind;
use crate::visual::BgaLayer;

// EventKind 枚举

/// 事件种类（不含脉冲位置）。
///
/// [`Event`] 将 `tick` 提取到外层结构体，本枚举仅表达"发生了什么"。
#[derive(Clone, Debug, PartialEq)]
pub enum EventKind<T, C: CustomEvent = NoCustomEvent> {
    /// 可玩音符。
    Note {
        /// 玩家侧。
        side: NoteSide,
        /// 按键 / 转盘 / 踏板。
        lane: Lane,
        /// 音符种类（普通、长音、地雷、不可见）。
        kind: NoteKind,
        /// 指向 [`crate::ChartData::audio_assets`] 的音频素材索引，或 `None`。
        audio_index: Option<u32>,
        /// 格式特有的扩展数据（无扩展时用 `()`）。
        ext: T,
    },
    /// BGM（背景音乐）事件 —— 触发音频，不参与游玩交互。
    Bgm {
        /// 指向 [`crate::ChartData::audio_assets`] 的索引。
        audio_index: u32,
    },
    /// BPM 变更。
    Bpm {
        /// 新的 BPM 值。
        bpm: f64,
    },
    /// 停止 / 暂停事件。
    Stop {
        /// 停止时长（脉冲数）。
        duration: u64,
    },
    /// 滚动速度倍率变更。
    Scroll {
        /// 滚动速度倍率（`1.0` = 标准，负值 = 反向）。
        rate: f64,
    },
    /// 视觉音符间距（SPEED）关键帧变更。
    ///
    /// 通过线性插值控制关键帧之间音符的视觉密度。与影响滚动速度的
    /// [`Scroll`](Self::Scroll) 不同，SPEED 仅影响音符在屏幕上排列的
    /// 紧密程度，与计时无关。
    Speed {
        /// 此关键帧处的间距倍率。
        rate: f64,
    },
    /// BGA（背景动画）显示事件。
    Bga {
        /// 此事件所针对的 BGA 图层。
        layer: BgaLayer,
        /// 指向 [`crate::ChartInfo::bga_resources`] 的索引。
        resource_id: u32,
    },
    /// 用于视觉显示的小节线。
    Bar,
    /// 格式特有的自定义事件。
    Custom(C),
}

impl<T, C: CustomEvent> EventKind<T, C> {
    /// 返回排序优先级（数字越小越优先）。
    ///
    /// 同一脉冲上按优先级升序排列。约定：
    ///
    /// `Bar(0) → Note/BGA/BGM(1) → BPM(2) → Stop(3) → Scroll(4) → Speed(5) → Custom(6)`
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Bar => 0,
            Self::Note { .. } | Self::Bga { .. } | Self::Bgm { .. } => 1,
            Self::Bpm { .. } => 2,
            Self::Stop { .. } => 3,
            Self::Scroll { .. } => 4,
            Self::Speed { .. } => 5,
            Self::Custom(_) => 6,
        }
    }
}

// 手动实现 Eq：所有 f64 字段（Bpm.bpm、Scroll.rate、Speed.rate）保证
// 不含 NaN，因此 PartialEq 满足 Eq 的反射性要求。
impl<T: Eq, C: Eq + CustomEvent> Eq for EventKind<T, C> {}

// Event 结构体

/// 谱面中的单个计时事件。
///
/// 将脉冲位置（`tick`）与事件种类（`kind`）分离。所有事件变体的 tick
/// 统一存储在此，而非各变体各自重复。
///
/// 处理器必须在排序前以期望的同脉冲顺序插入事件，因为稳定排序会保留
/// 插入顺序。
///
/// # 构造
///
/// ```rust
/// use bmsrs_chart::{Event, EventKind, NoteKind};
///
/// let ev: Event = Event::new(480, EventKind::Bar);
/// assert_eq!(ev.tick(), 480);
/// assert_eq!(ev.kind.priority(), 0);
/// ```
///
/// # `Eq` 保证
///
/// `Event` 手动实现 [`Eq`]。含有 `f64` 字段的变体（`Bpm`、`Scroll`、
/// `Speed`）保证其浮点字段不含 `NaN`，因此 `PartialEq` 比较满足
/// `Eq` 的反射性要求。
#[derive(Clone, Debug, PartialEq)]
pub struct Event<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// 脉冲位置。
    tick: u64,
    /// 事件种类。
    pub kind: EventKind<T, C>,
}

impl<T: NoteExt, C: CustomEvent> Event<T, C> {
    /// 创建一个新事件。
    #[must_use]
    pub const fn new(tick: u64, kind: EventKind<T, C>) -> Self {
        Self { tick, kind }
    }

    /// 返回此事件的脉冲位置。
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// 返回排序优先级（委托给 [`EventKind::priority`]）。
    #[must_use]
    pub const fn priority(&self) -> u8 {
        self.kind.priority()
    }

    /// 用于稳定排序的复合键：`(tick, priority)`。
    ///
    /// 收敛"同脉冲事件子序"约定（见 [`EventKind::priority`]）。
    /// 处理器应以本方法作为事件排序的唯一入口，避免排序规则散落多处。
    #[must_use]
    pub const fn sort_key(&self) -> (u64, u8) {
        (self.tick, self.kind.priority())
    }
}

// 手动实现 Eq。
impl<T: NoteExt + Eq, C: CustomEvent + Eq> Eq for Event<T, C> {}

// NoteExt trait

/// 格式特有的每音符扩展数据 trait。
///
/// 内置的 `()` 以零开销实现此 trait。
pub trait NoteExt: Clone + Debug + PartialEq + Eq + Default {}

impl NoteExt for () {}

// CustomEvent trait

/// 格式特有的自定义事件类型 trait（标记 trait）。
pub trait CustomEvent: Clone + Debug + PartialEq + Eq {}

/// 哨兵类型：无自定义事件。
///
/// 当 `C = NoCustomEvent` 时，Custom 变体携带此类型。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoCustomEvent;

impl CustomEvent for NoCustomEvent {}
