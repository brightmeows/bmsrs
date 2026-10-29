//! 音符种类定义。
//!
//! 音符位置（`NoteSide`、`Lane`）直接携带于
//! [`EventKind::Note`](crate::EventKind::Note) 中。每音符的格式扩展使用
//! [`NoteExt`](crate::NoteExt) trait。

use std::fmt::Debug;

use crate::audio::AudioIndex;

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

/// 音符的音频层集合。
///
/// 大多数音符只有一层音频；BMSON 允许不同 `SoundChannel` 在相同位置
/// 融合为一个音符，此时该音符携带多层音频。内部用三态表示，单音频
/// 不产生额外分配，`Many` 至少包含两个元素。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoteAudio(NoteAudioInner);

/// [`NoteAudio`] 的内部三态表示。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum NoteAudioInner {
    #[default]
    /// 无音频层。
    None,
    /// 单层音频。
    One(AudioIndex),
    /// 多层音频，至少两个元素。
    Many(Box<[AudioIndex]>),
}

impl NoteAudio {
    /// 无音频层的音符。
    #[must_use]
    pub const fn none() -> Self {
        Self(NoteAudioInner::None)
    }

    /// 只有一层音频的音符。
    #[must_use]
    pub const fn one(index: AudioIndex) -> Self {
        Self(NoteAudioInner::One(index))
    }

    /// 从任意音频索引集合构造；零层得到 [`none`](Self::none)，一层得到
    /// [`one`](Self::one)，两层及以上折叠为 `Many`。
    #[must_use]
    pub fn from_indices(indices: impl IntoIterator<Item = AudioIndex>) -> Self {
        let collected: Vec<AudioIndex> = indices.into_iter().collect();
        match collected.as_slice() {
            [] => Self::none(),
            [index] => Self::one(*index),
            _ => Self(NoteAudioInner::Many(collected.into_boxed_slice())),
        }
    }

    /// 以切片形式返回全部音频索引。
    #[must_use]
    pub fn as_slice(&self) -> &[AudioIndex] {
        match &self.0 {
            NoteAudioInner::None => &[],
            NoteAudioInner::One(index) => std::slice::from_ref(index),
            NoteAudioInner::Many(indices) => indices,
        }
    }

    /// 遍历全部音频索引。
    pub fn iter(&self) -> impl Iterator<Item = AudioIndex> + '_ {
        self.as_slice().iter().copied()
    }

    /// 第一层音频索引；无音频时为 [`None`]。
    #[must_use]
    pub fn primary(&self) -> Option<AudioIndex> {
        self.as_slice().first().copied()
    }

    /// 音频层数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// 是否没有音频层。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}

impl From<Option<AudioIndex>> for NoteAudio {
    fn from(value: Option<AudioIndex>) -> Self {
        value.map_or_else(Self::none, Self::one)
    }
}

/// 音符在游玩中的角色。
///
/// 与 [`NoteKind`] 正交：`up: true` 的释放音可以是普通音符，也可以是
/// 长音音符。角色属于核心模型，擦除格式扩展后仍然保留。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoteRole {
    /// 普通可判定音符。
    #[default]
    Normal,
    /// 释放音 / BSS / MSS 端点。
    Release,
}

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

/// 谱面级或按音符的长音类型提示。
///
/// `Ln` 和 `Cn` 对应 BMSON v2 标准值 `"ln"`/`"cn"`。
/// `Hcn` 是 beatoraja 扩展，不属于 v2 标准。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LnTypeHint {
    /// 长音（LN）—— 仅在按下时判定。
    #[default]
    Ln,
    /// 充电音（CN）—— 在按下和释放时都判定。
    Cn,
    /// 地狱充电音（HCN）—— beatoraja 扩展，非标准。
    Hcn,
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
