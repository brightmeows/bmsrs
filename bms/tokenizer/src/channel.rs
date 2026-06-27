//! 类型化的 BMS 通道枚举。
//!
//! 将 BMS 消息行（`#xxxYY:values`）中的通道号归类为
//! 已知的语义变体、音符通道，以及未知/保留通道。

use std::fmt;

use crate::index::ChannelIndex;

/// 已分类的 BMS 通道标识符。
///
/// 每个已知的非音符通道是一个单元变体。音符通道携带其原始
/// [`ChannelIndex`] 供下游解析（玩家侧、类型、按键号是
/// 解析器关心的事）。未知通道也携带原始索引，使自定义 /
/// 引擎特有的通道能在 token 流中被保留。
///
/// 内部 [`ChannelIndex`] 按 Base36（`0-9A-Z`）校验——构造时
/// 输入被规范化为大写。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BmsChannel {
    // 已知非音符通道（十六进制，01–0E）
    /// 通道 `01`——BGM。
    Bgm,
    /// 通道 `02`——小节长度。
    MeasureLength,
    /// 通道 `03`——BPM 变更（十六进制整数 `01`–`FF`）。
    BpmChange,
    /// 通道 `04`——BGA BASE 图层。
    BgaBase,
    /// 通道 `05`——扩展对象 / SEEK。
    Seek,
    /// 通道 `06`——BGA POOR（miss）图层。
    BgaPoor,
    /// 通道 `07`——BGA LAYER 叠加层。
    BgaLayer,
    /// 通道 `08`——扩展 BPM（引用 `#BPMxx` / `#EXBPMxx`）。
    ExtendedBpm,
    /// 通道 `09`——STOP 序列（引用 `#STOPxx`）。
    Stop,
    /// 通道 `0A`——BGA LAYER2。
    BgaLayer2,
    /// 通道 `0B`——BGA BASE 不透明度。
    BgaBaseOpacity,
    /// 通道 `0C`——BGA LAYER 不透明度。
    BgaLayerOpacity,
    /// 通道 `0D`——BGA LAYER2 不透明度。
    BgaLayer2Opacity,
    /// 通道 `0E`——BGA POOR 不透明度。
    BgaPoorOpacity,

    // 已知非音符通道（十六进制，97–A6）
    /// 通道 `97`——BGM 音量。
    BgmVolume,
    /// 通道 `98`——KEY 音量。
    KeyVolume,
    /// 通道 `99`——TEXT（引用 `#TEXTxx`）。
    Text,
    /// 通道 `A0`——JUDGE / EXRANK。
    Judge,
    /// 通道 `A1`——BGA BASE aRGB。
    BgaArgbBase,
    /// 通道 `A2`——BGA LAYER aRGB。
    BgaArgbLayer,
    /// 通道 `A3`——BGA LAYER2 aRGB。
    BgaArgbLayer2,
    /// 通道 `A4`——BGA POOR aRGB。
    BgaArgbPoor,
    /// 通道 `A5`——BGA KEYBOUND。
    BgaKeyBound,
    /// 通道 `A6`——OPTION。
    Option,

    // 扩展非音符通道（非十六进制）
    /// 通道 `SC`——SCROLL（滚动速度倍率）。
    Scroll,
    /// 通道 `SP`——SPEED（视觉音符间距）。
    Speed,

    // 音符通道
    /// 处于某一已知可玩范围内的音符通道：
    /// `11–1Z`、`21–2Z`、`31–3Z`、`41–4Z`、`51–5Z`、`61–6Z`、
    /// `D1–D9`、`E1–E9`。
    ///
    /// 原始 [`ChannelIndex`] 携带完整通道字符串，使解析器能
    /// 提取玩家侧、音符类型与按键号。
    Note(ChannelIndex),

    // 未知 / 保留
    /// 未知或保留通道（如 `00`、`0F`、`10`、`20`、
    /// `70–96`、`ZZ` 等）。
    Unknown(ChannelIndex),
}

// 构造

impl BmsChannel {
    /// 从原始通道字符串创建 `BmsChannel`。
    ///
    /// 输入在解析前规范化为大写。若字符串不是有效的
    /// 1–2 字符 Base36 标识符则返回 `None`。
    #[must_use]
    pub fn from_raw(s: &str) -> Option<Self> {
        let upper = s.to_ascii_uppercase();
        let idx: ChannelIndex = upper.as_str().try_into().ok()?;
        Some(classify_channel(idx))
    }
}

/// 将一个已校验的 [`ChannelIndex`] 归类为
/// [`BmsChannel`] 变体。
///
/// 每个有效的 `ChannelIndex` 都恰好映射到一个变体——不存在
/// 可能失败路径。
///
/// 输入索引应包含大写字符（调用方应在构造索引前规范化）。
#[must_use]
#[expect(
    clippy::unreachable,
    reason = "match arms confirmed by bytes.len() check above"
)]
pub fn classify_channel(ch: ChannelIndex) -> BmsChannel {
    let bytes = ch.as_bytes();

    match bytes.len() {
        1 => {
            // 单字符通道：一个十六进制位 0–F。
            // 输入已为大写（由调用方规范化）。
            let &[b] = bytes else {
                unreachable!("bytes.len() == 1 confirmed by match")
            };
            match b {
                b'1' => BmsChannel::Bgm,
                b'2' => BmsChannel::MeasureLength,
                b'3' => BmsChannel::BpmChange,
                b'4' => BmsChannel::BgaBase,
                b'5' => BmsChannel::Seek,
                b'6' => BmsChannel::BgaPoor,
                b'7' => BmsChannel::BgaLayer,
                b'8' => BmsChannel::ExtendedBpm,
                b'9' => BmsChannel::Stop,
                b'A' => BmsChannel::BgaLayer2,
                b'B' => BmsChannel::BgaBaseOpacity,
                b'C' => BmsChannel::BgaLayerOpacity,
                b'D' => BmsChannel::BgaLayer2Opacity,
                b'E' => BmsChannel::BgaPoorOpacity,
                // '0'（保留）、'F'（无定义通道）以及其他
                // 任何值都归入 Unknown。
                _ => BmsChannel::Unknown(ch),
            }
        }
        2 => {
            // 输入已为大写（由调用方规范化）。
            let &[first, second] = bytes else {
                unreachable!("bytes.len() == 2 confirmed by match")
            };

            // 精确的已知通道匹配。
            match (first, second) {
                (b'0', b'1') => return BmsChannel::Bgm,
                (b'0', b'2') => return BmsChannel::MeasureLength,
                (b'0', b'3') => return BmsChannel::BpmChange,
                (b'0', b'4') => return BmsChannel::BgaBase,
                (b'0', b'5') => return BmsChannel::Seek,
                (b'0', b'6') => return BmsChannel::BgaPoor,
                (b'0', b'7') => return BmsChannel::BgaLayer,
                (b'0', b'8') => return BmsChannel::ExtendedBpm,
                (b'0', b'9') => return BmsChannel::Stop,
                (b'0', b'A') => return BmsChannel::BgaLayer2,
                (b'0', b'B') => return BmsChannel::BgaBaseOpacity,
                (b'0', b'C') => return BmsChannel::BgaLayerOpacity,
                (b'0', b'D') => return BmsChannel::BgaLayer2Opacity,
                (b'0', b'E') => return BmsChannel::BgaPoorOpacity,
                (b'9', b'7') => return BmsChannel::BgmVolume,
                (b'9', b'8') => return BmsChannel::KeyVolume,
                (b'9', b'9') => return BmsChannel::Text,
                (b'A', b'0') => return BmsChannel::Judge,
                (b'A', b'1') => return BmsChannel::BgaArgbBase,
                (b'A', b'2') => return BmsChannel::BgaArgbLayer,
                (b'A', b'3') => return BmsChannel::BgaArgbLayer2,
                (b'A', b'4') => return BmsChannel::BgaArgbPoor,
                (b'A', b'5') => return BmsChannel::BgaKeyBound,
                (b'A', b'6') => return BmsChannel::Option,
                (b'S', b'C') => return BmsChannel::Scroll,
                (b'S', b'P') => return BmsChannel::Speed,
                _ => {}
            }

            // 音符通道检测。
            //
            // 范围：
            //   '1'..='6' + second != '0' → 可玩 / 长音
            //   'D'/'E'   + '1'..='9'     → 地雷
            if first.is_ascii_digit() && (b'1'..=b'6').contains(&first) && second != b'0' {
                return BmsChannel::Note(ch);
            }
            if matches!(first, b'D' | b'E')
                && second.is_ascii_digit()
                && (b'1'..=b'9').contains(&second)
            {
                return BmsChannel::Note(ch);
            }

            BmsChannel::Unknown(ch)
        }
        _ => {
            // `as_bytes()` 保证返回 1 或 2 字节；此分支不可达。
            unreachable!("ChannelIndex::as_bytes() returns 1 or 2 bytes")
        }
    }
}

// 访问器

impl BmsChannel {
    /// 返回此通道的十六进制 `u8` 值（若适用）。
    ///
    /// 单元变体返回其规范十六进制值（例如
    /// [`BmsChannel::Bgm`] → `Some(0x01)`）。非十六进制通道
    /// （[`Scroll`](Self::Scroll)、[`Speed`](Self::Speed)）返回 `None`。
    /// [`Note`](Self::Note) 与 [`Unknown`](Self::Unknown) 委托给
    /// 内部的 [`crate::index::BmsIndex::as_u8_hex`]。
    #[must_use]
    pub fn as_u8_hex(&self) -> Option<u8> {
        match self {
            Self::Bgm => Some(0x01),
            Self::MeasureLength => Some(0x02),
            Self::BpmChange => Some(0x03),
            Self::BgaBase => Some(0x04),
            Self::Seek => Some(0x05),
            Self::BgaPoor => Some(0x06),
            Self::BgaLayer => Some(0x07),
            Self::ExtendedBpm => Some(0x08),
            Self::Stop => Some(0x09),
            Self::BgaLayer2 => Some(0x0A),
            Self::BgaBaseOpacity => Some(0x0B),
            Self::BgaLayerOpacity => Some(0x0C),
            Self::BgaLayer2Opacity => Some(0x0D),
            Self::BgaPoorOpacity => Some(0x0E),
            Self::BgmVolume => Some(0x97),
            Self::KeyVolume => Some(0x98),
            Self::Text => Some(0x99),
            Self::Judge => Some(0xA0),
            Self::BgaArgbBase => Some(0xA1),
            Self::BgaArgbLayer => Some(0xA2),
            Self::BgaArgbLayer2 => Some(0xA3),
            Self::BgaArgbPoor => Some(0xA4),
            Self::BgaKeyBound => Some(0xA5),
            Self::Option => Some(0xA6),
            Self::Scroll | Self::Speed => None,
            Self::Note(raw) | Self::Unknown(raw) => raw.as_u8_hex(),
        }
    }
}

// Display

impl fmt::Display for BmsChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bgm => f.write_str("01"),
            Self::MeasureLength => f.write_str("02"),
            Self::BpmChange => f.write_str("03"),
            Self::BgaBase => f.write_str("04"),
            Self::Seek => f.write_str("05"),
            Self::BgaPoor => f.write_str("06"),
            Self::BgaLayer => f.write_str("07"),
            Self::ExtendedBpm => f.write_str("08"),
            Self::Stop => f.write_str("09"),
            Self::BgaLayer2 => f.write_str("0A"),
            Self::BgaBaseOpacity => f.write_str("0B"),
            Self::BgaLayerOpacity => f.write_str("0C"),
            Self::BgaLayer2Opacity => f.write_str("0D"),
            Self::BgaPoorOpacity => f.write_str("0E"),
            Self::BgmVolume => f.write_str("97"),
            Self::KeyVolume => f.write_str("98"),
            Self::Text => f.write_str("99"),
            Self::Judge => f.write_str("A0"),
            Self::BgaArgbBase => f.write_str("A1"),
            Self::BgaArgbLayer => f.write_str("A2"),
            Self::BgaArgbLayer2 => f.write_str("A3"),
            Self::BgaArgbPoor => f.write_str("A4"),
            Self::BgaKeyBound => f.write_str("A5"),
            Self::Option => f.write_str("A6"),
            Self::Scroll => f.write_str("SC"),
            Self::Speed => f.write_str("SP"),
            Self::Note(raw) | Self::Unknown(raw) => fmt::Display::fmt(raw, f),
        }
    }
}
