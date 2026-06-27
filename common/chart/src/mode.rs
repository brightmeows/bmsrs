//! 标识音符按键的公开位置类型。
//!
//! 音符位置是 `(NoteSide, Lane)` 二元组 —— 谱面模型中最底层的公开类型。
//! 每个音符都携带此二元组及其 [`NoteKind`](crate::NoteKind)。

use std::num::NonZeroU8;

/// 音符所属的游玩区域（playfield）一侧，以从 1 开始的索引表示。
///
/// 以 [`NonZeroU8`] 存储，使编号可扩展：当前格式仅使用 1 号和 2 号侧
/// （双人 BMS/BMSON），但未来格式或对战模式可在不改变此类型的前提下
/// 引入更多侧。
///
/// 任意索引请用 [`NoteSide::new`]；常见的双人场景请用
/// [`NoteSide::P1`] / [`NoteSide::P2`] 常量。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoteSide(NonZeroU8);

impl NoteSide {
    /// 1 号侧（1P）。
    pub const P1: Self = Self(NonZeroU8::MIN);

    /// 2 号侧（2P）。
    pub const P2: Self = match NonZeroU8::new(2) {
        Some(n) => Self(n),
        None => panic!("2 is non-zero"),
    };

    /// 从非零索引构造一侧。任意正值均合法，因此该类型对未来多于两侧的
    /// 格式保持开放。
    #[must_use]
    pub const fn new(value: NonZeroU8) -> Self {
        Self(value)
    }

    /// 从 1 开始的侧索引。
    #[must_use]
    pub const fn get(self) -> NonZeroU8 {
        self.0
    }

    /// 以普通 `u8` 形式返回侧索引，用于查表。
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0.get()
    }
}

/// 音符所在的按键，与玩家侧无关。
///
/// `Key` 与 `Scratch` 携带从 1 开始的索引（因此 `Key(1)` 是其所在侧的
/// 第一个常规按键），使用 [`NonZeroU8`] 以利用 niche 优化。该索引可
/// 区分多个转盘（例如 DSC/FPP 双转盘）以及同一侧内的多个按键。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// 常规可玩按键。所在侧内从 1 开始的索引，按该模式族物理布局从左到右排列。
    Key(NonZeroU8),
    /// 转盘 / 搓盘。从 1 开始的索引（1 为主转盘，双转盘模式中 2 为第二个转盘）。
    Scratch(NonZeroU8),
    /// 脚踏板（nanasi / Angolmois / OCT-FP 踏板模式）。
    FootPedal,
}
