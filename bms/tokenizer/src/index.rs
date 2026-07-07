//! 类型安全的 BMS 索引包装类型。
//!
//! BMS 使用 1–2 字符索引来引用资源（WAV、BMP）、
//! 计时定义（BPM、STOP、SCROLL、SPEED）、通道号，
//! 以及其他索引命令。
//!
//! [`BmsIndex`] 是带 Base62 字符校验的原始存储类型。
//! 每种语义索引是对 `BmsIndex` 的 newtype 包装
//! （例如 [`WavIndex`]、[`BpmIndex`]），提供编译期类型安全。
//!
//! # 字符集
//!
//! 大多数索引接受 Base62 字符（`0-9A-Za-z`）。[`ChannelIndex`]
//! 类型较特殊——它按 Base36（`0-9A-Z`）校验，因为 BMS 消息行中的
//! 通道号使用十六进制。
//!
//! [`BmsBase`] 枚举与 [`BmsIndex::is_valid_for`] 方法支持
//! 在需要时进行运行时字符集校验。
//!
//! # 存储
//!
//! ID 以 `[u8; 2]` 存储，每个字节为原始 ASCII 字符。
//! 对于单字符 ID（如 `"A"`），第二个字节为 `0`。

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use derive_more::{Deref, Display, From, FromStr};
use thiserror::Error;

use crate::{BmsTokenAttr, IntoTokensError};

// 字符集校验辅助函数

/// Base-62 字符：`0`–`9`、`A`–`Z`、`a`–`z`。
#[inline]
pub const fn is_base62(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
}

/// Base-36 大写字符：`0`–`9`、`A`–`Z`。
#[inline]
const fn is_base36(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z')
}

/// Base-16 十六进制字符：`0`–`9`、`A`–`F`、`a`–`f`。
#[inline]
const fn is_base16(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

// BmsBase —— 运行时字符集枚举

/// BMS 索引校验用的字符集。
///
/// 它取代了此前编译期的 `BmsCharset` 类型参数。
/// 运行时校验请使用 [`BmsIndex::is_valid_for`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
#[non_exhaustive]
pub enum BmsBase {
    /// 十六进制（`0`–`9`、`A`–`F`、`a`–`f`；每位置 16 个值）。
    #[bms_token("16")]
    Base16,
    /// Base-36 大写（`0`–`9`、`A`–`Z`；每位置 36 个值）。
    #[bms_token("36")]
    Base36,
    /// Base-62（`0`–`9`、`A`–`Z`、`a`–`z`；每位置 62 个值）。
    /// 这是大多数 BMS 索引的默认字符集。
    #[bms_token("62")]
    Base62,
}

// BmsIndex —— 原始存储（无泛型）

/// 已校验的 1–2 字符 BMS 索引，以原始 ASCII 字节存储。
///
/// 通过 [`TryFrom<&str>`] / [`FromStr`] 构造时以最宽松的 Base62
/// （`0-9A-Za-z`）字符集校验。运行时字符集校验见 [`BmsBase`] 与
/// [`is_valid_for`](Self::is_valid_for)。
///
/// # 存储
///
/// `[u8; 2]`——当 ID 为单字符时，`bytes[1]` 为 `0`。
#[derive(Clone, Copy, Debug)]
pub struct BmsIndex {
    /// 字符的原始 ASCII 字节。
    ///
    /// # 不变式
    ///
    /// 当 ID 为单字符（如 `"A"`）时，`bytes[1]` 为 `0`。
    /// 双字符 ID 同时存储两个字节——两者都不会是 `0`，因为
    /// 所有字符集校验器都排除了 `NUL`。
    bytes: [u8; 2],
}

// 自定义比较 trait：大小写敏感。
//
// 标准 BMS（36 进制）在解析器中将索引规范化为大写
// （`BmsIndex::normalize(BmsBase::Base36)`），因此所有存储的键均为
// 大写。Base62 模式保留原始大小写。由于所有插入与查找路径都
// 一致地规范化，此处的比较始终**大小写敏感**——这对
// `#BASE 62`（`"aa"` 与 `"AA"` 必须是不同键）是必需的。

impl PartialEq for BmsIndex {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for BmsIndex {}

impl PartialOrd for BmsIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BmsIndex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl Hash for BmsIndex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "guarded by match on bytes.len() or bytes[1] sentinel check"
)]
impl BmsIndex {
    /// 返回此 ID 的 ASCII 字符串表示（借用自 self）。
    ///
    /// # Panics
    ///
    /// 实际上永不 panic——字节在构造时已校验为 ASCII，
    /// 因此 UTF-8 转换不会失败。
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "bytes are ASCII by the BmsIndex construction invariant"
    )]
    pub fn as_str(&self) -> &str {
        let len = if self.bytes[1] == 0 { 1 } else { 2 };
        std::str::from_utf8(&self.bytes[..len]).expect("bytes validated ASCII on construction")
    }

    /// 返回原始 ASCII 字节（1 或 2 字节）。
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        let len = if self.bytes[1] == 0 { 1 } else { 2 };
        &self.bytes[..len]
    }

    /// 将 Base-36 字符转换为数值索引。
    ///
    /// `"00"` → `0`，`"ZZ"` → `1295`。单字符 ID 如
    /// `"A"` 返回 `10`（不乘进位）。
    ///
    /// 若字符无法解码则返回 `None`（当 ID 通过
    /// [`TryFrom`] / [`FromStr`] 构造时不应发生）。
    #[must_use]
    pub fn to_index(&self) -> Option<u16> {
        let hi = base36_digit_value(self.bytes[0])?;
        if self.bytes[1] == 0 {
            Some(hi)
        } else {
            let lo = base36_digit_value(self.bytes[1])?;
            Some(hi * 36 + lo)
        }
    }

    /// 将十六进制字符转换为 `u8` 值。
    ///
    /// 双字符 ID 如 `"0A"` 返回 `10`。单字符 ID 如
    /// `"A"` 返回 `10`（不位移）。
    ///
    /// 若任一字节不是有效的十六进制字符则返回 `None`。
    #[must_use]
    pub fn as_u8_hex(&self) -> Option<u8> {
        let hi = hex_digit_value(self.bytes[0])?;
        if self.bytes[1] == 0 {
            Some(hi)
        } else {
            let lo = hex_digit_value(self.bytes[1])?;
            Some((hi << 4) | lo)
        }
    }

    /// 检查此索引中所有字符对于给定字符集是否有效。
    #[must_use]
    pub fn is_valid_for(&self, base: BmsBase) -> bool {
        let check: fn(u8) -> bool = match base {
            BmsBase::Base16 => is_base16,
            BmsBase::Base36 => is_base36,
            BmsBase::Base62 => is_base62,
        };
        check(self.bytes[0]) && (self.bytes[1] == 0 || check(self.bytes[1]))
    }

    /// 从已校验字节创建（供 newtype 内部使用）。
    #[inline]
    #[must_use]
    const fn from_valid(bytes: [u8; 2]) -> Self {
        Self { bytes }
    }

    /// 返回针对给定进制规范化后的副本。
    ///
    /// 对于 [`BmsBase::Base16`] 和 [`BmsBase::Base36`]（标准 BMS
    /// 字符集），这会将字节转为大写，使 `"AA"` 与 `"aa"` 映射到
    /// 同一索引。对于 [`BmsBase::Base62`]，保留原始大小写
    /// （大小写敏感）。
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "to_ascii_uppercase is not const"
    )]
    pub fn normalize(&self, base: BmsBase) -> Self {
        match base {
            BmsBase::Base16 | BmsBase::Base36 => Self {
                bytes: [
                    self.bytes[0].to_ascii_uppercase(),
                    self.bytes[1].to_ascii_uppercase(),
                ],
            },
            BmsBase::Base62 => *self,
        }
    }
}

impl fmt::Display for BmsIndex {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[expect(clippy::indexing_slicing, reason = "guarded by match on bytes.len()")]
impl TryFrom<&str> for BmsIndex {
    type Error = BmsIndexError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        match bytes.len() {
            1 if is_base62(bytes[0]) => Ok(Self {
                bytes: [bytes[0], 0],
            }),
            2 if is_base62(bytes[0]) && is_base62(bytes[1]) => Ok(Self {
                bytes: [bytes[0], bytes[1]],
            }),
            _ => Err(BmsIndexError {
                input: s.to_owned(),
            }),
        }
    }
}

impl FromStr for BmsIndex {
    type Err = BmsIndexError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

// 数位解码辅助函数

/// 将单个十六进制 ASCII 字节解码为数值（0–15）。
const fn hex_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// 将单个 Base-36 ASCII 字节解码为数值（0–35）。
pub fn base36_digit_value(b: u8) -> Option<u16> {
    match b {
        b'0'..=b'9' => Some(u16::from(b - b'0')),
        b'A'..=b'Z' => Some(u16::from(b - b'A') + 10),
        b'a'..=b'z' => Some(u16::from(b - b'a') + 10),
        _ => None,
    }
}

/// 将 Base36 双字符字符串解码为 u16 值。
///
/// 输入必须恰好为 2 字符，且每个字符为有效的 Base36 字符。
/// 与 [`BmsIndex::to_index`] 语义一致但不依赖 `BmsIndex` 类型。
#[must_use]
pub fn base36_decode(s: &str) -> Option<u16> {
    let bytes = s.as_bytes();
    if bytes.len() != 2 {
        return None;
    }
    let hi = base36_digit_value(bytes[0])?;
    let lo = base36_digit_value(bytes[1])?;
    Some(u16::from(hi) * 36 + u16::from(lo))
}

// 错误类型

/// 当 BMS 索引字符串不是所需字符集下有效的 1–2 字符序列时
/// 返回的错误。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid BMS index: {input}")]
pub struct BmsIndexError {
    /// 校验失败的原始字符串。
    pub input: String,
}

impl<C> IntoTokensError<C> for BmsIndexError {
    #[inline]
    fn into_error(self, _context: &'static str, value: C) -> crate::BmsTokenizeError<C> {
        crate::BmsTokenizeError::InvalidInteger { value }
    }
}

// Newtype 包装——每种语义索引类型一个

// 每个 newtype：
//   - 包装 BmsIndex
//   - 提供 Deref<Target = BmsIndex> 以透明地访问方法
//   - 构造时校验所需的字符集
//   - 实现 Display、FromStr、TryFrom<&str>、Clone、Copy 等

/// 生成一个委托给 `FromStr` 的 `TryFrom<&str>` 实现。
macro_rules! impl_try_from_str {
    ($ty:ty) => {
        impl TryFrom<&str> for $ty {
            type Error = BmsIndexError;

            #[inline]
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

// 标准 Base62 newtype

/// `#WAV{id}` / `#EXWAV{id}` 的索引——音频定义引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct WavIndex(pub BmsIndex);
impl_try_from_str!(WavIndex);

/// `#BMP{id}` / `#BGA{id}` / `#@BGA{id}` / `#SWBGA{id}` /
/// `#ARGB{id}` / `#EXBMP{id}` 的索引——图片 / BGA 定义引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct BmpIndex(pub BmsIndex);
impl_try_from_str!(BmpIndex);

/// `#BPM{id}` / `#EXBPM{id}` 的索引——扩展 BPM 定义引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct BpmIndex(pub BmsIndex);
impl_try_from_str!(BpmIndex);

/// `#STOP{id}` 的索引——停止定义引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct StopIndex(pub BmsIndex);
impl_try_from_str!(StopIndex);

/// `#SCROLL{id}` 的索引——滚动速度定义引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ScrollIndex(pub BmsIndex);
impl_try_from_str!(ScrollIndex);

/// `#SPEED{id}` 的索引——速度/间距定义引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct SpeedIndex(pub BmsIndex);
impl_try_from_str!(SpeedIndex);

/// `#EXRANK{id}` 的索引——逐位置判定覆盖引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ExRankIndex(pub BmsIndex);
impl_try_from_str!(ExRankIndex);

/// `#SEEK{id}` 的索引——视频定位位置引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct SeekIndex(pub BmsIndex);
impl_try_from_str!(SeekIndex);

/// `#LNOBJ` 的索引——用作长音终止标记的 WAV 索引。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct LnObjIndex(pub BmsIndex);
impl_try_from_str!(LnObjIndex);

/// `#TEXT{id}` / `#SONG{id}` 的索引——定时屏幕文字引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct TextIndex(pub BmsIndex);
impl_try_from_str!(TextIndex);

/// `#CHANGEOPTION{id}` 的索引——动态选项变更引用。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ChangeOptionIndex(pub BmsIndex);
impl_try_from_str!(ChangeOptionIndex);

/// 消息正文值中 2 字符对象 ID 的索引。
///
/// 这是 [`BmsMessage`](crate::BmsMessage) 正文字符串中以宽松方式
/// 解析的对象索引——每对连续的有效 Base62 字符构成一个对象 ID。
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display, From, FromStr,
)]
#[display("{}", _0)]
pub struct ObjectIndex(pub BmsIndex);
impl_try_from_str!(ObjectIndex);

// 特殊 newtype：ChannelIndex（Base36 校验）

/// BMS 消息行中的通道号（`#xxxYY:values`）。
///
/// 按默认 Base62 不同，它按 Base36（`0-9A-Z`）校验。
/// 使用 [`as_u8_hex`](BmsIndex::as_u8_hex) 转换为字节值。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, Display)]
#[display("{}", _0)]
pub struct ChannelIndex(pub BmsIndex);

#[expect(clippy::indexing_slicing, reason = "guarded by match on bytes.len()")]
impl FromStr for ChannelIndex {
    type Err = BmsIndexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        match bytes.len() {
            1 if is_base36(bytes[0]) => Ok(Self(BmsIndex::from_valid([bytes[0], 0]))),
            2 if is_base36(bytes[0]) && is_base36(bytes[1]) => {
                Ok(Self(BmsIndex::from_valid([bytes[0], bytes[1]])))
            }
            _ => Err(BmsIndexError {
                input: s.to_owned(),
            }),
        }
    }
}

impl_try_from_str!(ChannelIndex);

// 测试
