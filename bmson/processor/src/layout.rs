//! BMSON 模式族布局：通过 [`BmsonLayout`] 将 `x` 值映射到轨道。
//!
//! 每个模式族表示为一个零大小 newtype，将音频通道的 `x` 值映射为
//! `(NoteSide, Lane)` 对。有状态的解码器（例如用于 `generic-nkeys` 的
//! [`GenericLayout`]）是独立结构体，自带一个固有的 `map_x` 方法。

use std::num::NonZeroU8;

use bmsrs_chart::mode::{Lane, NoteSide};

/// BMSON 侧映射：将音频通道的 `x` 值解码为 `(NoteSide, Lane)` 对。
///
/// 返回 `None` 将丢弃该音符。只有无状态模式族实现此 trait；有状态解码器
/// （例如 [`GenericLayout`]）自带 `map_x` 方法，通过
/// [`crate::BmsonProcessor::process_nkeys`] 使用。
pub trait BmsonLayout {
    /// 将 BMSON 玩家通道的 `x` 映射到音符位置。
    #[must_use]
    fn map_x(x: u64) -> Option<(NoteSide, Lane)>;
}

/// 从一个在调用处已知 ≥ 1 的值构造 `NonZeroU8`。
const fn nz(n: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(n)
}

/// Beat 模式族（BMSON）：覆盖 `beat-5k`、`beat-7k`、`beat-10k`、`beat-14k`
/// （以及 `dj-*` 别名）。
///
/// 物理布局（从左到右）：每侧 `KEY1-5 | SC | KEY6-7`。
///
/// 按 bmson 规范，`beat-10k` 谱面必须让 `x ∈ {6, 7, 14, 15}` 留空
/// （在每侧 5K 模式下这些槽位不存在）。本模式族在出现时将它们映射为
/// `KEY6`/`KEY7`，从而保留而不丢弃不合规范的输入。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Beat;

impl Beat {
    /// 将 BMSON `x` 值解码为 beat 模式族的位置。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x bounded by match arms to ≤16"
    )]
    #[must_use]
    pub fn from_bmson(x: u64) -> Option<(NoteSide, Lane)> {
        match x {
            1..=7 => Some((NoteSide::P1, Lane::Key(nz(x as u8)?))),
            8 => Some((NoteSide::P1, Lane::Scratch(nz(1)?))),
            9..=15 => Some((NoteSide::P2, Lane::Key(nz((x - 8) as u8)?))),
            16 => Some((NoteSide::P2, Lane::Scratch(nz(1)?))),
            _ => None,
        }
    }
}

impl BmsonLayout for Beat {
    fn map_x(x: u64) -> Option<(NoteSide, Lane)> {
        Self::from_bmson(x)
    }
}

/// PMS 模式族（BMSON）：单人 9 键布局，覆盖 `popn-9k` 与 `popn-5k`（子集）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pms;

impl Pms {
    /// 将 BMSON `x` 值解码为 PMS 模式族的位置（`popn-9k`/`5k`）。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x bounded by match arm to ≤9"
    )]
    #[must_use]
    pub fn from_bmson(x: u64) -> Option<(NoteSide, Lane)> {
        match x {
            1..=9 => Some((NoteSide::P1, Lane::Key(nz(x as u8)?))),
            _ => None,
        }
    }
}

impl BmsonLayout for Pms {
    fn map_x(x: u64) -> Option<(NoteSide, Lane)> {
        Self::from_bmson(x)
    }
}

/// 用于 BMSON `generic-nkeys` 的通用 n-keys 模式族。`keys` 为通道数；
/// 通道 `1..=keys` 从左到右依次映射到 `Lane::Key(1..=keys)`。
///
/// 本模式族带有运行时状态（按键数），因此**不**实现无状态的
/// [`BmsonLayout`]。请通过 [`crate::BmsonProcessor::process_nkeys`] 调用。
///
/// 其 `keys` 为 `u16`，可能超出 [`Lane::Key`] 的 `u8` 范围；超过 255 的
/// 通道会被拒绝。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericLayout {
    /// 按键数（全部位于玩家 1）。
    pub keys: u16,
}

impl GenericLayout {
    /// 针对当前按键数，将 BMSON `x` 值映射为音符位置。
    #[must_use]
    pub fn map_x(&self, x: u64) -> Option<(NoteSide, Lane)> {
        if !(1..=u64::from(self.keys)).contains(&x) {
            return None;
        }
        let n = nz(u8::try_from(x).ok()?)?;
        Some((NoteSide::P1, Lane::Key(n)))
    }
}
