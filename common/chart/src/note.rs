//! 音符种类定义。
//!
//! 音符位置（`NoteSide`、`Lane`）直接携带于
//! [`Event::Note`](crate::Event::Note) 中。每音符的格式扩展使用
//! [`NoteExt`](crate::NoteExt) trait。

use std::fmt::Debug;

/// 地雷伤害值。
///
/// 保证不含 NaN，支持 [`Eq`] 比较。
#[derive(Clone, Copy, Debug, Default)]
pub struct Damage(f64);

impl Damage {
    /// 从 `f64` 创建伤害值。
    ///
    /// # Panic（仅 debug 构建）
    ///
    /// debug 构建中若 `value` 为 NaN 则 panic。
    #[must_use]
    pub fn new(value: f64) -> Self {
        debug_assert!(!value.is_nan(), "damage must not be NaN");
        Self(value)
    }

    /// 返回原始伤害值。
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for Damage {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Damage {}

/// 可玩音符的种类。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
        damage: Damage,
    },
    /// 不可见音符 —— 触发音频但不显示，也不按常规判定。
    ///
    /// 在 BMS 中这些是 "key" 音符（通道 `31`–`39`、`41`–`49`）。
    /// 在 BMSON 中这些来自 `key_channels`。
    Invisible,
}

/// 谱面级或按音符的长音类型提示（BMSON LN/CN）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LnTypeHint {
    /// 长音（LN）—— 仅在按下时判定。
    #[default]
    Ln,
    /// 充电音（CN）—— 在按下和释放时都判定。
    Cn,
}

/// 谱面级或按音符的长音判定提示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LnJudgeHint {
    /// 仅判定音符本身。
    #[default]
    Normal,
    /// 按住期间额外判定若干脉冲。
    Ticks,
}

/// 谱面级或按音符的长音血量提示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LnLifeHint {
    /// 仅音符本身恢复血量。
    #[default]
    Normal,
    /// 按住期间额外脉冲恢复血量。
    Ticks,
}
