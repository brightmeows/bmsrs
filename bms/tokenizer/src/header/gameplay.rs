//! 游玩行为头部：`#PLAYER`、`#RANK`、`#TOTAL`、`#VOLWAV`、
//! `#LNTYPE`、`#LNOBJ`、`#LNMODE`、`#OCT/FP`、`#OPTION`、`#CHANGEOPTION`、
//! `#BASE`。
//!
//! 本模块还定义了 [`BmsHeaderGameplay`] 所用的域类型：
//! [`PlayerMode`]、[`Rank`]、[`LnType`]、[`LnMode`] 与 [`BmsBaseMode`]。

use std::fmt;
use std::str::FromStr;

use crate::BmsTokenAttr;
use crate::index::{BmsBase, ChangeOptionIndex, ExRankIndex, LnObjIndex};
use crate::{BmsHeader, BmsTryFromError};

/// `#PLAYER` 指定的游玩模式。
///
/// 现代播放器（LR2、nanasi、ruvit、beatoraja）通常**忽略**
/// `#PLAYER`，从谱面中出现的通道推断实际模式。
/// 该命令仅为向后兼容而保留。
///
/// | 值 | 模式 | 血量槽 | 备注 |
/// |-------|------|---------------|-------|
/// | `1` / `SP` | Single Play | 1 | 默认；仅 1P 侧 |
/// | `2` / `CP` | Couple Play | 2 | 双人合作；如今很少支持 |
/// | `3` / `DP` | Double Play | 1 | 一名玩家使用两侧 |
/// | `4` / `BP` | Battle Play | 2 | 两名玩家玩同一谱面；仅 BM98 支持 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum PlayerMode {
    /// 单人（Single Play）。值：`"1"` 或 `"SP"`。
    #[bms_token("1")]
    #[bms_token("SP")]
    #[bms_token("sp")]
    #[bms_token("Sp")]
    Single,
    /// 双人合作（Couple Play）。值：`"2"` 或 `"CP"`。
    #[bms_token("2")]
    #[bms_token("CP")]
    #[bms_token("cp")]
    #[bms_token("Cp")]
    Couple,
    /// Double Play（一人玩两侧）。值：`"3"` 或 `"DP"`。
    #[bms_token("3")]
    #[bms_token("DP")]
    #[bms_token("dp")]
    #[bms_token("Dp")]
    Double,
    /// Battle Play（两人玩同一谱面）。值：`"4"` 或 `"BP"`。
    #[bms_token("4")]
    #[bms_token("BP")]
    #[bms_token("bp")]
    #[bms_token("Bp")]
    Battle,
}

/// `#LNTYPE` 指定的长音记法。
///
/// - **RDM**（`Type1`、`#LNTYPE 1`）：长音从首个非 `00`
///   音符开始，到下一个非 `00` 音符结束。这是现代默认；
///   省略 `#LNTYPE` 即表示 RDM。
/// - **MGQ**（`Type2`、`#LNTYPE 2`）：长音在非 `00` 音符
///   连续时持续，遇到 `00` 时关闭。**已废弃**——没有现代播放器
///   使用 MGQ 记法。
///
/// 两种类型都使用通道 `#xxx51-69`。另一种方式是
/// [`LnObj`](BmsHeaderGameplay::LnObj)（RDM 类型 #2），它消耗一个
/// `#WAV` 索引作为长音终止标记，允许作者在普通的
/// `#xxx11-29` 通道上编辑长音。
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum LnType {
    /// RDM 类型长音（`#LNTYPE 1`）。
    #[bms_token("1")]
    #[bms_token("01")]
    Type1,
    /// MGQ 类型长音（`#LNTYPE 2`）。
    #[bms_token("2")]
    #[bms_token("02")]
    Type2,
}

/// `#LNMODE` 指定的长音模式（beatoraja 扩展）。
///
/// 决定在 beatoraja 中游玩时长音的行为。
/// 存在时，谱面的长音种类被**强制**，不受
/// 玩家 LN MODE 选项影响。
///
/// | 值 | 模式 | 行为 |
/// |-------|------|-----------|
/// | `1` | LN | 标准长音——起始处按键，结束处松键 |
/// | `2` | CN | 充能音符——按住穿过音符；结束无需松键 |
/// | `3` | HCN | 地狱充能音符——类似 CN 但判定更严格 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum LnMode {
    /// 标准长音（`#LNMODE 1`）。
    #[bms_token("1")]
    #[bms_token("01")]
    Ln,
    /// 充能音符（`#LNMODE 2`）。
    #[bms_token("2")]
    #[bms_token("02")]
    Cn,
    /// 地狱充能音符（`#LNMODE 3`）。
    #[bms_token("3")]
    #[bms_token("03")]
    Hcn,
}

/// `#RANK` 指定的判定难度。
///
/// 控制玩家计时的严格程度。标准值
/// 0–4 映射到具名变体；非标准值（例如 fgt++ 相对
/// rank）以 [`Rank::Other`] 保留。
///
/// 省略 `#RANK` 时默认：**`Normal` (2)**（在大多数播放器中）。
/// 值得注意的例外：BMSE 与 iBMSC 默认为 `Easy` (3)。
///
/// | 值 | 标签 | 近似窗口（LR2）| 备注 |
/// |-------|-------|----------------------|-------|
/// | `0` | VERY HARD | ±8 ms | |
/// | `1` | HARD | ±15 ms | |
/// | `2` | NORMAL | ±18 ms | 默认 |
/// | `3` | EASY | ±21 ms | |
/// | `4` | VERY EASY | — | nanasi/beatoraja 扩展 |
///
/// 某些播放器（fgt++、Angolmois、`TechnicalGroove`）接受
/// 0–4 以外的值并将其视为相对倍率。这些由
/// [`Rank::Other`] 捕获。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    /// `#RANK 0`——VERY HARD（LR2 中 ±8 ms）。
    VeryHard,
    /// `#RANK 1`——HARD（LR2 中 ±15 ms）。
    Hard,
    /// `#RANK 2`——NORMAL（LR2 中 ±18 ms）。省略 `#RANK` 时的默认值。
    Normal,
    /// `#RANK 3`——EASY（LR2 中 ±21 ms）。
    Easy,
    /// `#RANK 4`——VERY EASY（nanasi/beatoraja 扩展）。
    VeryEasy,
    /// 为向前兼容而保留的非标准 rank 值。
    Other(u8),
}

impl FromStr for Rank {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: u8 = s.parse()?;
        Ok(match v {
            0 => Self::VeryHard,
            1 => Self::Hard,
            2 => Self::Normal,
            3 => Self::Easy,
            4 => Self::VeryEasy,
            n => Self::Other(n),
        })
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = match self {
            Self::VeryHard => 0,
            Self::Hard => 1,
            Self::Normal => 2,
            Self::Easy => 3,
            Self::VeryEasy => 4,
            Self::Other(n) => *n,
        };
        write!(f, "{v}")
    }
}

// BmsBase 定义于 crate::index::BmsBase（与前
// BmsBaseMode 相同）。此处通过 import 使用；derive BmsTokenAttr 确保
// `#BASE 16`、`#BASE 36`、`#BASE 62` 正确解析。

/// 游玩行为头部。
///
/// 这些命令控制谱面*如何*游玩——判定严格度、
/// 血量槽（生命条）行为、长音解读，以及谱面
/// 选项。
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderGameplay<C> {
    /// `#PLAYER`——游戏模式（Single / Couple / Double / Battle）。
    ///
    /// 现代播放器基本忽略，从通道推断模式。
    #[bms_token("#PLAYER {}")]
    Player(PlayerMode),
    /// `#RANK`——判定难度（VERY HARD … VERY EASY）。
    ///
    /// 省略时默认：`Normal` (2)。
    #[bms_token("#RANK {}")]
    Rank(Rank),
    /// `#DEFEXRANK`——以百分比表示的细粒度判定难度。
    ///
    /// `100` 等于 `#RANK 2`（NORMAL）。两者同时
    /// 存在时覆盖 `#RANK`（最接近 EOF 的行胜出）。支持小数值。
    #[bms_token("#DEFEXRANK {}")]
    DefExRank(f64),
    /// `#EXRANK{id}`——逐位置判定覆盖。
    ///
    /// 被通道 `#xxxA0` 引用。当一个 `#EXRANK` 对象越过
    /// 判定时，判定窗口变为指定的
    /// 百分比。谱面显示的难度标签在 nanasi 中变为
    /// "RANDOM"。
    #[bms_token("#EXRANK{id} {value}")]
    ExRank {
        /// 2 字符索引。
        id: ExRankIndex,
        /// 以百分比表示的判定宽度（NORMAL = 100）。
        value: f64,
    },
    /// `#TOTAL`——血量槽最大增加量（百分比）。
    ///
    /// 所有完美判定的音符会使血量槽每音符增加 `TOTAL / N`
    /// 个百分点，其中 `N` 为可见音符总数。
    /// 例如 `#TOTAL 200` 配 400 个音符，每音符 +0.5%。
    ///
    /// **强烈建议**始终指定——默认值因
    /// 播放器差异巨大（BM98：`200+NOTES`；LR2：`160`；nanasi：
    /// `350`；fgt++：`100+NOTES/8`）。
    ///
    /// 某些播放器（nazo、nazoZZ）支持负值，
    /// 使*完美*判定*降低*血量槽。
    #[bms_token("#TOTAL {}")]
    Total(f64),
    /// `#VOLWAV`——所有音频的主音量百分比。
    ///
    /// `100` = 原始音量。默认：`100`。
    ///
    /// **已废弃**——高度依赖实现与硬件。
    /// 结果因播放器与驱动而异。beatoraja 上限为 100。
    #[bms_token("#VOLWAV {}")]
    VolWav(f64),
    /// `#LNTYPE`——长音记法（RDM 或 MGQ）。
    ///
    /// `1` = RDM（默认）；`2` = MGQ（已废弃）。
    #[bms_token("#LNTYPE {}")]
    LnType(LnType),
    /// `#LNOBJ`——将一个 `#WAV` 索引指定为长音终止标记。
    ///
    /// 当此索引的音符出现在通道 `#xxx11-29` 上时，它作为
    /// 长音的*终点*（前一个可见音符为
    /// 起点）。这是 `#LNTYPE 1` + 通道
    /// `#xxx51-69` 的替代方案——流行的原因是 BMSE 在把 `#xxx51-69`
    /// 对象移到 BGM 时会崩溃。
    ///
    /// **注意**：nanasi 与 fgt++ 有个小写索引
    /// 不被识别为 `#LNOBJ` 标记的 bug——请使用大写。
    #[bms_token("#LNOBJ {}")]
    LnObj(LnObjIndex),
    /// `#LNMODE`——强制 LN / CN / HCN 模式（beatoraja 扩展）。
    ///
    /// 存在时，谱面的长音类型被锁定，不受
    /// 玩家 LN MODE 选项影响。
    #[bms_token("#LNMODE {}")]
    LnMode(LnMode),
    /// `#OCT` / `#FP` / `#OCT/FP`——OCTAVE MODE 标志。
    ///
    /// 原本是 14KEYS → OCT/FP 视觉重映射的 nanasi 标识符。
    /// 数值被丢弃——没有已知播放器使用它。
    /// 无论输入形式如何，`format_header` 始终输出 `#OCT/FP`。
    #[bms_token("#OCT/FP")]
    #[bms_token("#OCT")]
    #[bms_token("#FP")]
    OctFp,
    /// `#OPTION`——从 BMS 文件强制玩家侧选项（nanasi）。
    ///
    /// 值使用厂商前缀（例如 `774:HI-SPEED_x0.77`）。
    /// 多行 `#OPTION` 可共存；同类选项使用
    /// 最接近 EOF 的行。
    #[bms_token("#OPTION {}")]
    Option(C),
    /// `#CHANGEOPTION{id}`——游玩过程中动态改变选项（nanasi）。
    ///
    /// 被通道 `#xxxA6` 引用。并非所有选项都支持动态
    /// 变更（例如 `RANDOM`、`NOTES` 系列不支持）。
    #[bms_token("#CHANGEOPTION{id} {value}")]
    ChangeOption {
        /// 2 字符索引。
        id: ChangeOptionIndex,
        /// 选项字符串（例如 `"774:HIDDEN_STEALTH"`）。
        value: C,
    },
    /// `#BASE`——声明索引命令的进制基数。
    ///
    /// 有效值：`16`（十六进制，256 槽位）、`36`（base-36，1296 槽位，默认）、
    /// `62`（大小写敏感的 base-62，3844 槽位，beatoraja 扩展）。
    /// 未知值回退到 `BmsHeaderFallback`。
    #[bms_token("#BASE {}")]
    #[bms_fallback]
    Base(BmsBase),
}

// From / TryFrom 转换

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderGameplay<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Gameplay(g) => Ok(g),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_mode_single() {
        assert_eq!("1".parse::<PlayerMode>().unwrap(), PlayerMode::Single);
        assert_eq!("SP".parse::<PlayerMode>().unwrap(), PlayerMode::Single);
        assert_eq!("sp".parse::<PlayerMode>().unwrap(), PlayerMode::Single);
    }

    #[test]
    fn player_mode_couple() {
        assert_eq!("2".parse::<PlayerMode>().unwrap(), PlayerMode::Couple);
        assert_eq!("CP".parse::<PlayerMode>().unwrap(), PlayerMode::Couple);
    }

    #[test]
    fn player_mode_double() {
        assert_eq!("3".parse::<PlayerMode>().unwrap(), PlayerMode::Double);
        assert_eq!("DP".parse::<PlayerMode>().unwrap(), PlayerMode::Double);
    }

    #[test]
    fn player_mode_battle() {
        assert_eq!("4".parse::<PlayerMode>().unwrap(), PlayerMode::Battle);
        assert_eq!("BP".parse::<PlayerMode>().unwrap(), PlayerMode::Battle);
    }

    #[test]
    fn player_mode_invalid() {
        assert!("5".parse::<PlayerMode>().is_err());
        assert!("".parse::<PlayerMode>().is_err());
        assert!("abc".parse::<PlayerMode>().is_err());
        assert!("0".parse::<PlayerMode>().is_err());
    }

    #[test]
    fn ln_type_1() {
        assert_eq!("1".parse::<LnType>().unwrap(), LnType::Type1);
        assert_eq!("01".parse::<LnType>().unwrap(), LnType::Type1);
    }

    #[test]
    fn ln_type_2() {
        assert_eq!("2".parse::<LnType>().unwrap(), LnType::Type2);
        assert_eq!("02".parse::<LnType>().unwrap(), LnType::Type2);
    }

    #[test]
    fn ln_type_invalid() {
        assert!("0".parse::<LnType>().is_err());
        assert!("3".parse::<LnType>().is_err());
        assert!("abc".parse::<LnType>().is_err());
    }

    #[test]
    fn ln_mode_ln() {
        assert_eq!("1".parse::<LnMode>().unwrap(), LnMode::Ln);
        assert_eq!("01".parse::<LnMode>().unwrap(), LnMode::Ln);
    }

    #[test]
    fn ln_mode_cn() {
        assert_eq!("2".parse::<LnMode>().unwrap(), LnMode::Cn);
        assert_eq!("02".parse::<LnMode>().unwrap(), LnMode::Cn);
    }

    #[test]
    fn ln_mode_hcn() {
        assert_eq!("3".parse::<LnMode>().unwrap(), LnMode::Hcn);
        assert_eq!("03".parse::<LnMode>().unwrap(), LnMode::Hcn);
    }

    #[test]
    fn ln_mode_invalid() {
        assert!("0".parse::<LnMode>().is_err());
        assert!("4".parse::<LnMode>().is_err());
        assert!("abc".parse::<LnMode>().is_err());
    }

    #[test]
    fn rank_standard_values() {
        assert_eq!("0".parse::<Rank>().unwrap(), Rank::VeryHard);
        assert_eq!("1".parse::<Rank>().unwrap(), Rank::Hard);
        assert_eq!("2".parse::<Rank>().unwrap(), Rank::Normal);
        assert_eq!("3".parse::<Rank>().unwrap(), Rank::Easy);
        assert_eq!("4".parse::<Rank>().unwrap(), Rank::VeryEasy);
    }

    #[test]
    fn rank_non_standard_preserved() {
        assert_eq!("5".parse::<Rank>().unwrap(), Rank::Other(5));
        assert_eq!("255".parse::<Rank>().unwrap(), Rank::Other(255));
    }

    #[test]
    fn rank_non_numeric_is_error() {
        assert!("abc".parse::<Rank>().is_err());
        assert!("".parse::<Rank>().is_err());
    }

    #[test]
    fn rank_display_roundtrip() {
        assert_eq!("0".parse::<Rank>().unwrap().to_string(), "0");
        assert_eq!("4".parse::<Rank>().unwrap().to_string(), "4");
        assert_eq!("5".parse::<Rank>().unwrap().to_string(), "5");
    }
}
