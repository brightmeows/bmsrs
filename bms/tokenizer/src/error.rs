//! 分词器错误类型——BMS 分词器的统一错误类型。

use std::fmt;
use std::num::NonZeroUsize;
use std::num::ParseFloatError;
use std::num::ParseIntError;

use thiserror::Error;

/// BMS 分词期间可能发生的错误。
///
/// 每个变体都携带原始输入 `value`（作为字符串容器
/// `C`），使调用方可以检视或显示导致失败的原始文本。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BmsTokenizeError<C> {
    /// 通道行中的小节号不是有效的 3 位数值。
    #[error("invalid measure number: \"{value}\"")]
    InvalidMeasure {
        /// 校验失败的原始小节字符串。
        value: C,
    },
    /// 通道行中的通道号不是有效的 2 位数值。
    #[error("invalid channel number: \"{value}\"")]
    InvalidChannel {
        /// 校验失败的原始通道字符串。
        value: C,
    },
    /// 整数字段无法解析。
    #[error("invalid integer: \"{value}\"")]
    InvalidInteger {
        /// 无法解析为整数的原始输入。
        value: C,
    },
    /// 浮点字段无法解析。
    #[error("invalid float: \"{value}\"")]
    InvalidFloat {
        /// 无法解析为浮点数的原始输入。
        value: C,
    },
    /// 输入在其上下文中不是可识别的值。
    ///
    /// 同时涵盖“越界”（值的形状正确但超出有效范围）与
    /// “不可识别”（值不匹配字面量枚举的任何已知选项）。当没有
    /// 特定的有效集合可用时，`expected` 为空字符串。
    #[error("value out of range: \"{value}\" for {context} (expected {expected})")]
    OutOfRange {
        /// 头部命令名（例如 `"#DIFFICULTY"`）。
        context: &'static str,
        /// 越界或不可识别的原始输入。
        value: C,
        /// 对有效范围的描述（例如 `"1-5"`、`"1 or 2"`），或
        /// 当无特定提示可用时为空字符串。
        expected: &'static str,
    },
}

impl<C> BmsTokenizeError<C> {
    /// 将借用的 `BmsTokenizeError<&str>` 转换为此容器类型。
    #[must_use]
    pub(crate) fn from_ref<'a>(err: &BmsTokenizeError<&'a str>) -> Self
    where
        C: From<&'a str>,
    {
        match err {
            BmsTokenizeError::InvalidMeasure { value } => Self::InvalidMeasure {
                value: C::from(*value),
            },
            BmsTokenizeError::InvalidChannel { value } => Self::InvalidChannel {
                value: C::from(*value),
            },
            BmsTokenizeError::InvalidInteger { value } => Self::InvalidInteger {
                value: C::from(*value),
            },
            BmsTokenizeError::InvalidFloat { value } => Self::InvalidFloat {
                value: C::from(*value),
            },
            BmsTokenizeError::OutOfRange {
                context,
                value,
                expected,
            } => Self::OutOfRange {
                context,
                value: C::from(*value),
                expected,
            },
        }
    }
}

/// 从 `FromStr::Err` 到 [`BmsTokenizeError`] 的转换。
///
/// `#[derive(BmsTokenAttr)]` 的 derive 宏调用此 trait，
/// 将任何解析错误转换为分词器的统一错误类型。
/// 已为 [`ParseIntError`]、[`ParseFloatError`]、
/// [`ParseBmsValueError`]，以及——对于已产生 `BmsTokenizeError` 的
/// 自定义 `FromStr` 实现——[`BmsTokenizeError`] 本身（恒等）提供了实现。
pub trait IntoTokensError<C> {
    /// 将此错误转换为 `BmsTokenizeError`。
    ///
    /// * `context` —— 头部命令名（例如 `"#PLAYER"`）。
    /// * `value` —— 解析失败的原始输入字符串。
    fn into_error(self, context: &'static str, value: C) -> BmsTokenizeError<C>;
}

impl<C> IntoTokensError<C> for BmsTokenizeError<C> {
    fn into_error(self, context: &'static str, value: C) -> Self {
        match self {
            Self::OutOfRange {
                context: "",
                expected,
                ..
            } => Self::OutOfRange {
                context,
                value,
                expected,
            },
            other => other,
        }
    }
}

impl<C> IntoTokensError<C> for ParseIntError {
    fn into_error(self, _context: &'static str, value: C) -> BmsTokenizeError<C> {
        BmsTokenizeError::InvalidInteger { value }
    }
}

impl<C> IntoTokensError<C> for ParseFloatError {
    fn into_error(self, _context: &'static str, value: C) -> BmsTokenizeError<C> {
        BmsTokenizeError::InvalidFloat { value }
    }
}

impl<C> IntoTokensError<C> for ParseBmsValueError {
    fn into_error(self, context: &'static str, value: C) -> BmsTokenizeError<C> {
        BmsTokenizeError::OutOfRange {
            context,
            value,
            expected: self.0,
        }
    }
}

/// 输入无法解析为预期的 BMS 值类型。
///
/// 由 `#[derive(BmsTokenAttr)]` 在字面量模式下生成的
/// [`std::str::FromStr`] 实现返回。
///
/// 当非空时，该字符串提供关于预期值的可读提示
/// （例如 `"expected 1 or 2"`）。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub struct ParseBmsValueError(pub &'static str);

impl fmt::Display for ParseBmsValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "invalid BMS value")
        } else {
            write!(f, "expected {}", self.0)
        }
    }
}

/// token 类型间 `TryFrom` 转换的错误类型。
///
/// 当从 [`BmsToken`](crate::BmsToken) 或 `(NonZeroUsize, Result<BmsToken, BmsTokenizeError>)` 元组
/// 中提取头部或消息时使用。
///
/// **注意**：子枚举提取（如 `BmsHeader::try_unwrap_gameplay`）不再通过此类型报告错误，
/// 改为返回 [`derive_more::TryUnwrapError`]。`WrongHeaderType` 变体已在 `0.1.0` 周期中移除。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BmsTryFromError<C> {
    /// 源 `(NonZeroUsize, Result<BmsToken, …>)` 含有 `Err`。
    #[error("tokenization error on line {line}: {error}")]
    TokenizationError {
        /// 错误发生的从 1 起行号。
        line: NonZeroUsize,
        /// 底层的分词错误。
        error: BmsTokenizeError<C>,
    },
    /// `BmsToken` 是 `Message`，而非 `Header`。
    #[error("expected a header, but the token is a channel message")]
    NotAHeader,
    /// `BmsToken` 是 `Header`，而非 `Message`。
    #[error("expected a message, but the token is a header")]
    NotAMessage,
}
