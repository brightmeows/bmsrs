//! 统一逻辑 lane 与格式特定表示的互转。
//!
//! 编辑命令以统一逻辑 lane（按键从 1 起左到右、转盘、踏板）寻址；
//! BMS 侧映射为 `(player, lane byte)` 通道形状，bmson 侧映射为
//! `x` 玩家通道编号。映射表与 `bms-processor` 的布局族保持一致。

use bms_tokenizer::BmsChannel;

/// 统一逻辑 lane。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LaneRef {
    /// 常规按键，1 起从左到右。
    Key(u8),
    /// 转盘，1 为主转盘。
    Scratch(u8),
}

impl LaneRef {
    /// 从 BME 布局的 `(player, lane byte)` 还原。
    ///
    /// 与 `bms-processor` 的 `Bme` 布局一致：
    /// 1–5 → Key1–5，6 → Scratch1，8 → Key6，9 → Key7，10–35 → 扩展。
    #[must_use]
    pub const fn from_bme_lane_byte(lane_byte: u8) -> Option<Self> {
        match lane_byte {
            6 => Some(Self::Scratch(1)),
            8 => Some(Self::Key(6)),
            9 => Some(Self::Key(7)),
            1..=5 | 10..=35 => Some(Self::Key(lane_byte)),
            _ => None,
        }
    }

    /// 映射到 BME 布局的 lane byte。
    #[must_use]
    pub fn to_bme_lane_byte(self) -> Option<u8> {
        match self {
            Self::Key(6) => Some(8),
            Self::Key(7) => Some(9),
            Self::Key(n) if (1..=5).contains(&n) || (8..=35).contains(&n) => Some(n),
            Self::Scratch(1) => Some(6),
            Self::Key(_) | Self::Scratch(_) => None,
        }
    }

    /// 从 bmson `x`（beat 族：1–7 按键、8 转盘）还原。
    #[must_use]
    pub fn from_bmson_beat_x(x: u64) -> Option<Self> {
        match x {
            1..=7 => Some(Self::Key(u8::try_from(x).ok()?)),
            8 => Some(Self::Scratch(1)),
            0 | 9..=u64::MAX => None,
        }
    }

    /// 映射到 bmson `x`（beat 族）。
    #[must_use]
    pub fn to_bmson_beat_x(self) -> Option<u64> {
        match self {
            Self::Key(n) if (1..=7).contains(&n) => Some(u64::from(n)),
            Self::Scratch(1) => Some(8),
            _ => None,
        }
    }

    /// 从 bmson `x`（popn 族：1–9 按键）还原。
    #[must_use]
    pub fn from_bmson_popn_x(x: u64) -> Option<Self> {
        u8::try_from(x)
            .ok()
            .filter(|n| (1..=9).contains(n))
            .map(Self::Key)
    }
}

/// 从 BMS 通道提取 `(player, LaneRef)`；非音符通道返回 `None`。
#[must_use]
pub fn note_channel_to_lane(ch: BmsChannel) -> Option<(u8, LaneRef)> {
    let BmsChannel::Note(idx) = ch else {
        return None;
    };
    let text = idx.to_string();
    let bytes = text.as_bytes();
    let player = match bytes.first() {
        Some(b'1') => 1,
        Some(b'2') => 2,
        _ => return None,
    };
    #[expect(
        clippy::string_slice,
        reason = "channel text is ASCII by the Base36 construction invariant"
    )]
    let lane_text = &text[1..];
    let lane_byte: u8 = lane_text.parse().ok()?;
    let lane = LaneRef::from_bme_lane_byte(lane_byte)?;
    Some((player, lane))
}

/// 推断谱面按键数（不含转盘）。
///
/// 取可见音符 lane 中最大 Key 编号；beat 常见值归并到 5/7/14。
#[must_use]
pub fn infer_keys(lanes: impl IntoIterator<Item = LaneRef>) -> u8 {
    lanes
        .into_iter()
        .filter_map(|lane| match lane {
            LaneRef::Key(n) => Some(n),
            LaneRef::Scratch(_) => None,
        })
        .max()
        .unwrap_or(0)
}

/// 解析用户输入的通道号字符串为 [`BmsChannel`]。
#[must_use]
pub fn parse_channel(s: &str) -> Option<BmsChannel> {
    bms_tokenizer::BmsChannel::from_raw(s)
}
