//! 计时定义头部：`#BPM`、`#BPMxx`/`#EXBPMxx`、`#BASEBPM`、
//! `#STOPxx`、`#SCROLLxx`、`#SPEEDxx`、`#STP`。

use std::fmt;

use crate::BmsTokenAttr;
use crate::BmsValue;
use crate::index::{BpmIndex, ScrollIndex, SpeedIndex, StopIndex};
use crate::{BmsHeader, BmsTryFromError};

/// `#STP` 的参数——bemaniaDX 式停止（绝对时间，毫秒）。
///
/// 值格式：`xxx[.yyy] zzzz`
/// - `xxx` = 小节号（0–999，3 位零填充）
/// - `.yyy` = 小节内位置（0–999，可选；解释为
///   一个小节的 `yyy/1000`）
/// - `zzzz` = 停止时长（毫秒）
///
/// 与 `#STOPxx`（以 192 分音符为单位，因此依赖 BPM）不同，
/// `#STP` 始终按固定的 wall-clock 时长停止，与 BPM 无关。
///
/// 同一位置的多行 `#STP` 是累加的。
///
/// **注意**：bemaniaDX 会忽略超过一定数量（限制未指定）的
/// `#STP` 行。`yyy ≥ 960` 的值可能在 bemaniaDX 中被忽略或导致
/// 冻结。Angolmois 与 Sonorous 的怪癖更少。
#[derive(Debug, Clone, PartialEq)]
pub struct StpParams {
    /// 小节号（0–999，3 位零填充）。
    pub measure: u16,
    /// 小节内位置（0–999，即 `xxx.yyy` 中的 `yyy`）。
    ///
    /// 解释为一个小节的 `yyy/1000`。≥ 960 的值可能
    /// 在 bemaniaDX 中被忽略或导致冻结。
    pub position: u16,
    /// 时长（毫秒）。
    pub duration_ms: f64,
}

impl fmt::Display for StpParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.position == 0 {
            write!(f, "{:03} {}", self.measure, self.duration_ms)
        } else {
            write!(
                f,
                "{:03}.{} {}",
                self.measure, self.position, self.duration_ms
            )
        }
    }
}

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C> for StpParams {
    #[expect(
        clippy::string_slice,
        reason = "pos_part is ASCII digits and dots from BMS format; byte indexing is safe"
    )]
    fn parse(s: &'a str) -> Option<Self> {
        let (pos_part, dur_part) = s.split_once(' ')?;
        let dur_ms: f64 = dur_part.trim().parse().ok()?;

        let (measure_str, position_str) = pos_part.find('.').map_or((pos_part, "0"), |dot| {
            (&pos_part[..dot], &pos_part[dot + 1..])
        });

        let measure: u16 = measure_str.parse().ok()?;
        let position: u16 = position_str.parse().ok()?;
        if measure > 999 || position > 999 {
            return None;
        }

        Some(Self {
            measure,
            position,
            duration_ms: dur_ms,
        })
    }
}

/// 计时定义头部。
///
/// 这些命令控制事件*何时*发生——节拍、停止，以及滚动
/// 技巧。
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderTiming {
    /// `#BPM`——全局初始 BPM。
    ///
    /// 省略时默认：`130`（规范），但播放器各异（nanasi：
    /// `150`；nazo/BMSE：`120`；fgt++：`30`；fgt#/pomu2：`0`）。
    /// 在大多数播放器中支持小数值。
    #[bms_token("#BPM {}")]
    Bpm(f64),
    /// `#BPM{id}`——扩展 BPM 定义（bemaniaDX 起源）。
    ///
    /// 被通道 `#xxx08` 引用。支持小数与
    /// 超出 255 的 BPM 值，这是基本 `#xxx03` 通道（十六进制整数）
    /// 无法表示的。
    ///
    /// 负值在某些播放器中导致反向滚动，但
    /// 这是事实约定——规范未定义。
    ///
    /// `#EXBPM{id}` 是功能等价的别名（nanasi，为绕过
    /// BMSC 的一个解析 bug）。
    #[bms_token("#BPM{id} {value}")]
    BpmDef {
        /// 2 字符索引（例如 `"01"`、`"2A"`）。
        id: BpmIndex,
        /// BPM 值（可为小数或负数）。
        value: f64,
    },
    /// `#BASEBPM`——用于自动 HI-SPEED 计算的参考 BPM（LR 起源）。
    ///
    /// 在谱面有短暂极端 BPM 尖峰时使用。通常
    /// 播放器的自动速度使用最大 BPM，但 `#BASEBPM` 让谱师
    /// 指定更实用的参考值。
    #[bms_token("#BASEBPM {}")]
    BaseBpm(f64),
    /// `#STOP{id}`——DDR 式停止（192 分音符单位）。
    ///
    /// 被通道 `#xxx09` 引用。值以 4/4 小节的
    /// 192 分音符为单位，因此实际的 wall-clock 停止时长
    /// 取决于该位置的 BPM：`duration = value * 60 /
    /// (BPM * 192)`。
    ///
    /// 当 STOP 与 BPM 变更发生在同一位置时，先应用
    /// BPM 变更，再依据新 BPM 计算停止。
    ///
    /// 负值在某些播放器（LR2、
    /// nanasi 等）中导致前跳，另一些则忽略。小数值
    /// 被大多数播放器截断（floor）；仅少数接受。
    #[bms_token("#STOP{id} {value}")]
    StopDef {
        /// 2 字符索引。
        id: StopIndex,
        /// 停止时长（192 分音符单位，可为小数）。
        value: f64,
    },
    /// `#SCROLL{id}`——滚动速度倍率（beatoraja 扩展）。
    ///
    /// 被通道 `#xxxSC` 引用。独立于 BPM 倍率
    /// 放大视觉滚动速度。默认为 `1.0`。负值
    /// 导致反向滚动。
    #[bms_token("#SCROLL{id} {value}")]
    ScrollDef {
        /// 2 字符索引。
        id: ScrollIndex,
        /// 滚动速度倍率。
        value: f64,
    },
    /// `#SPEED{id}`——速度/间距倍率（pomu2 起源）。
    ///
    /// 被通道 `#xxxSP` 引用。与 `#SCROLL`（缩放
    /// 滚动速度）不同，`#SPEED` 改变音符间的视觉间距
    /// 而不影响滚动速度——类似 "sudden+" / "hidden+"
    /// 调整。支持位置间插值。
    #[bms_token("#SPEED{id} {value}")]
    SpeedDef {
        /// 2 字符索引。
        id: SpeedIndex,
        /// 速度倍率值。
        value: f64,
    },
    /// `#EXBPM{id}`——`#BPM{id}` 的别名（nanasi 起源）。
    ///
    /// 功能上与 `#BPM{id}` 相同。存在是因为 BMSC 有个
    /// bug，会把 `#BPMxx` 与全局 `#BPM` 混淆。也
    /// 被通道 `#xxx08` 引用。
    #[bms_token("#EXBPM{id} {value}")]
    ExBpm {
        /// 2 字符索引。
        id: BpmIndex,
        /// BPM 值。
        value: f64,
    },
    /// `#STP`——bemaniaDX 式停止（绝对毫秒）。
    ///
    /// 与 `#STOPxx`（依赖 BPM 的 192 分音符单位）不同，它定义了
    /// 在指定小节位置上具有固定 wall-clock 时长的停止。
    ///
    /// 解析失败会回退到 `Fallback` 头部，因为值
    /// 格式 `xxx[.yyy] zzzz` 是非标准的。
    #[bms_token("#STP {params}")]
    #[bms_fallback]
    Stp {
        /// 解析出的步进计时参数。
        params: StpParams,
    },
}

// From / TryFrom 转换

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderTiming {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Timing(t) => Ok(t),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stp_params_with_position() {
        let p = <StpParams as BmsValue<'_, &str>>::parse("001.128 500").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 128);
        assert!((p.duration_ms - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stp_params_without_position() {
        let p = <StpParams as BmsValue<'_, &str>>::parse("001 500.5").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 0);
        assert!((p.duration_ms - 500.5).abs() < f64::EPSILON);
    }

    #[test]
    fn stp_params_position_999_accepted() {
        let p = <StpParams as BmsValue<'_, &str>>::parse("001.999 500").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 999);
        assert!((p.duration_ms - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stp_params_position_over_999_rejected() {
        assert!(<StpParams as BmsValue<'_, &str>>::parse("001.1000 500").is_none());
    }

    #[test]
    fn stp_params_measure_0_accepted() {
        let p = <StpParams as BmsValue<'_, &str>>::parse("000 500").unwrap();
        assert_eq!(p.measure, 0);
    }

    #[test]
    fn stp_params_measure_999_accepted() {
        let p = <StpParams as BmsValue<'_, &str>>::parse("999 500").unwrap();
        assert_eq!(p.measure, 999);
    }

    #[test]
    fn stp_params_measure_over_999_rejected() {
        assert!(<StpParams as BmsValue<'_, &str>>::parse("1000 500").is_none());
    }

    #[test]
    fn stp_params_invalid_format_rejected() {
        assert!(<StpParams as BmsValue<'_, &str>>::parse("invalid").is_none());
        assert!(<StpParams as BmsValue<'_, &str>>::parse("").is_none());
    }

    #[test]
    fn stp_params_display_with_position() {
        let p = StpParams {
            measure: 1,
            position: 128,
            duration_ms: 500.0,
        };
        assert_eq!(p.to_string(), "001.128 500");
    }

    #[test]
    fn stp_params_display_without_position() {
        let p = StpParams {
            measure: 1,
            position: 0,
            duration_ms: 500.5,
        };
        assert_eq!(p.to_string(), "001 500.5");
    }
}
