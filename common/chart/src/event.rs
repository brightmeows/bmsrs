//! 统一事件枚举与格式扩展 trait。
//!
//! 所有计时事件存放在单一的 `Vec<Event<T, C>>` 中，按脉冲排序，
//! 支持统一迭代与二分查找。
//!
//! 泛型参数：
//! - `T` —— 每音符扩展数据（携带于 [`Event::Note`]）。
//! - `C` —— 格式特有的自定义事件类型（携带于 [`Event::Custom`]）。
//!
//! # 排序
//!
//! 处理器使用稳定排序按脉冲对事件排序，因此同一脉冲上各事件的相对顺序
//! 由插入顺序决定。约定如下：
//!
//! `Bar → Note/BGA/BGM → BPM → Stop → Scroll → Custom`

use std::fmt::Debug;

use crate::mode::{Lane, NoteSide};
use crate::note::NoteKind;
use crate::visual::BgaLayer;

// Event 枚举

/// 谱面中的单个计时事件。
///
/// 每个变体都含有一个 `tick` 字段（或方法），给出其在谱面时间线上的绝对
/// 位置。统一访问请用 [`Event::tick`]。
///
/// 处理器必须在排序前以期望的同脉冲顺序插入事件，因为稳定排序会保留
/// 插入顺序。
#[derive(Clone, Debug, PartialEq)]
pub enum Event<T, C: CustomEvent = NoCustomEvent> {
    /// 可玩音符。
    Note {
        /// 脉冲位置。
        tick: u64,
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
        /// 脉冲位置。
        tick: u64,
        /// 指向 [`crate::ChartData::audio_assets`] 的索引。
        audio_index: u32,
    },
    /// BPM 变更。
    Bpm {
        /// 脉冲位置。
        tick: u64,
        /// 新的 BPM 值。
        bpm: f64,
    },
    /// 停止 / 暂停事件。
    Stop {
        /// 脉冲位置。
        tick: u64,
        /// 停止时长（脉冲数）。
        duration: u64,
    },
    /// 滚动速度倍率变更。
    Scroll {
        /// 脉冲位置。
        tick: u64,
        /// 滚动速度倍率（`1.0` = 标准，负值 = 反向）。
        rate: f64,
    },
    /// 视觉音符间距（SPEED）关键帧变更。
    ///
    /// 通过线性插值控制关键帧之间音符的视觉密度。与影响滚动速度的
    /// [`Scroll`](Self::Scroll) 不同，SPEED 仅影响音符在屏幕上排列的
    /// 紧密程度，与计时无关。
    Speed {
        /// 脉冲位置。
        tick: u64,
        /// 此关键帧处的间距倍率。
        rate: f64,
    },
    /// BGA（背景动画）显示事件。
    Bga {
        /// 脉冲位置。
        tick: u64,
        /// 此事件所针对的 BGA 图层。
        layer: BgaLayer,
        /// 指向 [`crate::ChartInfo::bga_resources`] 的索引。
        resource_id: u32,
    },
    /// 用于视觉显示的小节线。
    Bar {
        /// 脉冲位置。
        tick: u64,
    },
    /// 格式特有的自定义事件。
    Custom(C),
}

impl<T, C: CustomEvent> Event<T, C> {
    /// 统一返回此事件的脉冲位置，与变体无关。
    #[must_use]
    pub fn tick(&self) -> u64 {
        match self {
            Self::Custom(c) => c.tick(),
            Self::Note { tick, .. }
            | Self::Bgm { tick, .. }
            | Self::Bpm { tick, .. }
            | Self::Stop { tick, .. }
            | Self::Scroll { tick, .. }
            | Self::Speed { tick, .. }
            | Self::Bga { tick, .. }
            | Self::Bar { tick } => *tick,
        }
    }
}

// NoteExt trait

/// 格式特有的每音符扩展数据 trait。
///
/// 内置的 `()` 以零开销实现此 trait。
pub trait NoteExt: Clone + Debug + PartialEq + Default {}

impl NoteExt for () {}

// CustomEvent trait

/// 格式特有的自定义事件类型 trait。
///
/// 自定义事件参与统一的已排序时间线。处理器必须在稳定排序前以期望的
/// 同脉冲顺序插入它们。
pub trait CustomEvent: Clone + Debug + PartialEq {
    /// 此自定义事件的脉冲位置。
    fn tick(&self) -> u64;
}

/// 哨兵类型：无自定义事件。
///
/// 当 `C = NoCustomEvent` 时，`Custom` 变体永远不会被构造。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoCustomEvent;

impl CustomEvent for NoCustomEvent {
    fn tick(&self) -> u64 {
        0
    }
}
