//! 音符种类定义。
//!
//! 音符位置（`NoteSide`、`Lane`）直接携带于
//! [`Event::Note`](crate::Event::Note) 中。每音符的格式扩展使用
//! [`NoteExt`](crate::NoteExt) trait。

use std::fmt::Debug;

/// 可玩音符的种类。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum NoteKind {
    /// 普通音符（短音）—— 单次敲击。
    #[default]
    Normal,
    /// 长音 —— 从该音符的脉冲位置开始保持 `duration` 个脉冲。
    Long {
        /// 保持时长（脉冲数）。
        duration: u64,
    },
    /// 地雷 —— 按下会扣血。
    Mine {
        /// 伤害量（由游戏定义的单位）。
        damage: f64,
    },
    /// 不可见音符 —— 触发音频但不显示，也不按常规判定。
    ///
    /// 在 BMS 中这些是 "key" 音符（通道 `31`–`39`、`41`–`49`）。
    /// 在 BMSON 中这些来自 `key_channels`。
    Invisible,
}
