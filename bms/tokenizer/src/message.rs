//! BMS 消息（通道数据）行解析。
//!
//! # 格式
//!
//! 通道数据行形式为 `#ADDR:body`，其中：
//!
//! - `ADDR` 是一个字符串，由**小节**（数字位，0 起索引）
//!   后接**通道**（最后 1–2 个有效的 Base62 字符
//!   （`0-9A-Za-z`））组成。例如在 `#00111:...` 中，
//!   地址 `00111` 的小节为 `001`、通道为 `11`。
//! - `body` 是原始值字符串——2 字符的对象索引由
//!   下游（解析器）从拼接的原始存储中解析。
//!   未知或扩展通道（如 `SC`、`SP`、`1G`）保存在
//!   [`channel`](BmsMessage::channel) 字段与原始存储中。
//!
//! # 通道语义（节选）
//!
//! 完整的通道映射是解析器的职责——此表仅列出最常见的通道供参考：
//!
//! | 通道 | 用途 |
//! |---------|---------|
//! | `01` | BGM（可跨多行）|
//! | `02` | 小节长度变更 |
//! | `03` | BPM 变更（十六进制整数，`[01-FF]`）|
//! | `04` | BGA BASE 图层 |
//! | `06` | BGA POOR（miss）图层 |
//! | `07` | BGA LAYER（黑色 = 透明）|
//! | `08` | 扩展 BPM 变更 |
//! | `09` | STOP 序列 |
//! | `0A` | BGA LAYER2（SCROLL）|
//! | `11-19` | 1P 可见音符 |
//! | `21-29` | 2P 可见音符 |
//! | `31-39` | 1P 不可见音符 |
//! | `41-49` | 2P 不可见音符 |
//! | `51-69` | 长音通道 |
//! | `D1-D9` | 1P 地雷 |
//! | `E1-E9` | 2P 地雷 |
//! | `SC` | SCROLL（扩展）|
//! | `SP` | SPEED（扩展）|
//!
//! # 示例
//!
//! ```text
//! #00111:11223344 → addr="00111", body="11223344"
//!                    track=1, channel="11"
//! #0010A:01       → addr="0010A", body="01"
//!                    track=1, channel="0A"
//! #000SC:         → addr="000SC", body=""
//!                    track=0, channel="SC"
//! ```

use std::fmt;

use crate::channel::{BmsChannel, classify_channel};
use crate::index::{ChannelIndex, is_base62};
use crate::{BmsToken, BmsTokenizeError, BmsTryFromError};

/// BMS 文件中的通道数据行（`#ADDR:body`）。
///
/// 格式描述见模块级文档。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsMessage<C> {
    /// 原始地址字符串（`:` 之前）。
    pub addr: C,
    /// 原始正文字符串（`:` 之后）。
    pub body: C,

    /// 0 起索引的小节号，从 [`addr`](BmsMessage::addr) 中
    /// 通道后缀之前的数字位提取。
    pub track: u16,
    /// 通道号——[`addr`](BmsMessage::addr) 中最后 1–2 个有效的 Base62 字符
    /// （`0-9A-Za-z`），归类为 [`BmsChannel`] 枚举。
    pub channel: BmsChannel,
}

impl<C> TryFrom<BmsToken<C>> for BmsMessage<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(token: BmsToken<C>) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Message(m) => Ok(m),
            BmsToken::Header(_) => Err(BmsTryFromError::NotAMessage),
        }
    }
}

/// 尝试将单行解析为 BMS 通道消息。
///
/// 若该行看起来不像通道消息（例如是头部、注释或空行），
/// 返回 `Ok(None)`。
/// 若该行看起来像通道消息但没有有效的通道后缀，
/// 返回 `Err(...)`。
///
/// # Errors
///
/// 当地址在通道位置含有非 Base62 字符，或通道后缀不可识别时，
/// 返回 [`BmsTokenizeError::InvalidChannel`]。
#[expect(
    clippy::string_slice,
    reason = "BMS message lines are ASCII-only (hex digits, Base62 chars, colons); byte indexing is safe"
)]
pub fn parse_message_line<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a>(
    line: &'a str,
) -> Result<Option<BmsMessage<C>>, BmsTokenizeError<C>> {
    if line.is_empty() || !line.starts_with('#') {
        return Ok(None);
    }

    // 至少需要：# + 1 字符 + :  （如 "#1:"）
    if line.len() < 3 {
        return Ok(None);
    }

    let rest = &line[1..]; // 去掉 '#'

    // 在 ':' 处拆分得到 addr 与 body
    let Some((addr, body)) = rest.split_once(':') else {
        return Ok(None); // 无冒号——非消息行
    };

    if addr.is_empty() || !addr.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        // addr 必须以数字开头（小节号前缀）才能算作消息行；
        // 否则是像 #SWBGA01 30:... 这样恰好含冒号的头部。
        return Ok(None);
    }

    // 从 addr 末尾解析通道：
    // 取最后 1–2 个连续的有效 Base62 字符。
    #[expect(
        clippy::indexing_slicing,
        reason = "addr is non-empty (checked above); `len` guards indices"
    )]
    let (channel_str, prefix) = {
        let bytes = addr.as_bytes();
        let len = bytes.len();
        let last = bytes[len - 1];
        if !is_base62(last) {
            return Err(BmsTokenizeError::InvalidChannel {
                value: C::from(addr),
            });
        }
        if len >= 2 {
            let second_last = bytes[len - 2];
            if is_base62(second_last) {
                (&addr[len - 2..], &addr[..len - 2])
            } else {
                (&addr[len - 1..], &addr[..len - 1])
            }
        } else {
            (&addr[len - 1..], &addr[..len - 1])
        }
    };

    // 规范化为大写；通道 ID 大小写不敏感，以 Base36
    // （大写字母数字）存储。
    let channel_upper = channel_str.to_ascii_uppercase();
    let channel_idx: ChannelIndex =
        channel_upper
            .as_str()
            .try_into()
            .map_err(|_e| BmsTokenizeError::InvalidChannel {
                value: C::from(addr),
            })?;
    let channel = classify_channel(channel_idx);

    // 小节：从 prefix 提取所有 ASCII 数字字符，构建 u16。
    // 0 起索引；空 prefix → track = 0。
    let track: u16 = prefix
        .chars()
        .filter(char::is_ascii_digit)
        .fold(0u16, |acc, c| {
            acc.saturating_mul(10)
                .saturating_add(u16::from(c as u8 - b'0'))
        });

    Ok(Some(BmsMessage {
        addr: C::from(addr),
        body: C::from(body),
        track,
        channel,
    }))
}
