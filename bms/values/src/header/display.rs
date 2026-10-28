//! 显示与难度头部：`#STAGEFILE`、`#BANNER`、`#BACKBMP`、
//! `#CHARFILE`、`#PLAYLEVEL`、`#DIFFICULTY`、`#PREVIEW`。
//!
//! 本模块还定义了 [`BmsHeaderDisplay`] 所用的域类型：
//! [`DifficultyLevel`] 与 [`PoorBgaMode`]。

use std::str::FromStr;

use thiserror::Error;

use crate::BmsTokenAttr;
use crate::IntoTokensError;

/// `#DIFFICULTY`（值 1–5）指定的难度分类。
///
/// 用于在选曲界面排序与筛选谱面。常见
/// 映射：
///
/// | 值 | 典型标签 |
/// |-------|---------------|
/// | `1` | BEGINNER / EASY / LIGHT |
/// | `2` | NORMAL / STANDARD |
/// | `3` | HYPER / HARD |
/// | `4` | ANOTHER / EX |
/// | `5` | INSANE / BLACK ANOTHER |
///
/// 允许省略 `#DIFFICULTY`，但意味着谱面无法按
/// 难度分类筛选。
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display, derive_more::Deref)]
#[display("{}", _0)]
pub struct DifficultyLevel(u8);

impl DifficultyLevel {
    /// 原始数值（1–5）。
    #[must_use]
    pub const fn get(&self) -> u8 {
        self.0
    }
}

/// 当 `#DIFFICULTY` 值不在有效范围（1–5）内时返回的错误。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid #DIFFICULTY value: {0} (expected 1-5)")]
pub struct ParseDifficultyError(pub String);

impl IntoTokensError for ParseDifficultyError {
    fn into_error(self, context: &'static str, value: String) -> crate::BmsTokenizeError {
        crate::BmsTokenizeError::OutOfRange {
            context,
            value,
            expected: "1-5",
        }
    }
}

impl FromStr for DifficultyLevel {
    type Err = ParseDifficultyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: u8 = s.parse().map_err(|_e| ParseDifficultyError(s.to_owned()))?;
        if !(1..=5).contains(&v) {
            return Err(ParseDifficultyError(s.to_owned()));
        }
        Ok(Self(v))
    }
}

/// `#POORBGA` 指定的 poor BGA 显示模式。
///
/// 控制 miss / poor 图片（通道 `#xxx06`）的显示方式：
///
/// | 值 | 行为 |
/// |-------|-----------|
/// | `0` | **默认**——miss 时整个 BGA 切换到 `#xxx06` 片刻，然后返回正常图片序列。 |
/// | `1` | **叠加**——`#xxx06` 图片叠加到当前 BGA 之上（类似 beatmaniaIIDX 的 miss 角色动画）。 |
/// | `2` | **隐藏**——永不显示 miss 图片；正常 BGA 不受干扰地继续。 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum PoorBgaMode {
    /// `#POORBGA 0`——使用默认 BGA 显示行为。
    #[bms_token("0")]
    Default,
    /// `#POORBGA 1`——在当前 BGA 之上叠加 poor BGA。
    #[bms_token("1")]
    Overlay,
    /// `#POORBGA 2`——显示 poor BGA 时隐藏当前 BGA。
    #[bms_token("2")]
    Hidden,
}

/// 显示与难度头部。
///
/// 这些命令控制实际游玩音符*之外*玩家*所见*的内容——
/// 加载画面、横幅、难度标签等。
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderDisplay {
    /// `#STAGEFILE`——加载期间显示的启动画面图片（通常 640×480）。
    ///
    /// 可选。省略时播放器显示其默认加载画面。
    #[bms_token("#STAGEFILE {}")]
    StageFile(String),
    /// `#BANNER`——选曲与结算界面的横幅图片（300×80）。
    ///
    /// 可选。支持相对路径（仅下级）。路径长度
    /// 限制为 260 字节。
    #[bms_token("#BANNER {}")]
    Banner(String),
    /// `#BACKBMP`——游玩界面的背景图片（通常 640×480）。
    ///
    /// 原始规范：图片填充游玩区域背景。在某些
    /// LR2 皮肤中，它被用作标题卡。尺寸与行为
    /// 取决于皮肤。
    #[bms_token("#BACKBMP {}")]
    BackBmp(String),
    /// `#CHARFILE`——pop'n music 风格的角色文件（pomu2 扩展）。
    ///
    /// 一个 `.chp` 文件，定义游玩期间显示的动画角色。
    /// 仅 pomu2 与 PMChr-V 支持。
    #[bms_token("#CHARFILE {}")]
    CharFile(String),
    /// `#PLAYLEVEL`——选曲列表中显示的难度数值。
    ///
    /// 显示格式因播放器而异（星星、条形图、整数）。
    /// 通常为整数，但某些播放器接受字符串（例如
    /// `#PLAYLEVEL 安心`）。省略时默认：`3`（BM98 约定）。
    ///
    /// 值 `0` 在 BM98 和若干其他播放器中有特殊含义：
    /// 它显示为问号（`?`）而非数值，常用于难度随
    /// `#RANDOM`/`#SWITCH` 变化的谱面。
    #[bms_token("#PLAYLEVEL {}")]
    PlayLevel(f64),
    /// `#DIFFICULTY`——用于谱面筛选的难度*分类*（1–5）。
    #[bms_token("#DIFFICULTY {}")]
    Difficulty(DifficultyLevel),
    /// `#PREVIEW`——选曲界面播放的音频文件
    /// （beatoraja 扩展）。
    ///
    /// 省略时，beatoraja 自动发现谱面文件夹中的
    /// `preview*.wav` / `preview*.ogg`。
    #[bms_token("#PREVIEW {}")]
    Preview(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_valid_range() {
        for i in 1..=5u8 {
            let d: DifficultyLevel = i.to_string().parse().unwrap();
            assert_eq!(d.get(), i);
        }
    }

    #[test]
    fn difficulty_out_of_range() {
        assert!("0".parse::<DifficultyLevel>().is_err());
        assert!("6".parse::<DifficultyLevel>().is_err());
        assert!("100".parse::<DifficultyLevel>().is_err());
    }

    #[test]
    fn difficulty_non_numeric() {
        assert!("abc".parse::<DifficultyLevel>().is_err());
        assert!("".parse::<DifficultyLevel>().is_err());
    }
}
