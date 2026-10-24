//! BMS（Be-Music Script）格式的单行语法。
//!
//! 本 crate 提供 BMS 解析管道的最底层：把**一行**文本解析为结构化 token。
//! 整文件的分行、注释状态机、编码与行级编辑由 `bms-document` 负责。
//!
//! # 结构
//!
//! - [`enum@BmsToken`] 表示 BMS 文件中一行有意义的内容。
//! - [`BmsHeader`] 涵盖所有头部命令，按语义域分类。
//! - [`BmsMessage`] 涵盖通道数据行（`#xxxYY:values`）。
//! - [`parse_header_line`] / [`parse_message_line`] 是单行入口。
//!
//! 子模块为私有；所有公开类型均从 crate 根重新导出。
//!
//! 类型化值（数值转换、解析出的枚举）始终为 owned。

use std::fmt;
use std::num::NonZeroUsize;

mod channel;
mod error;
mod header;
mod index;
mod message;

pub use bms_tokenizer_derive::{BmsIndexNewtype, BmsTokenAttr};
pub use channel::BmsChannel;
pub use error::{BmsTokenizeError, BmsTryFromError, IntoTokensError, ParseBmsValueError};
pub use header::parse_header_line;
pub use header::{
    ArgbParams, AtBgaParams, BgaParams, DifficultyLevel, ExBmpParams, ExWavParams, LnMode, LnType,
    ParseDifficultyError, PlayerMode, PoorBgaMode, Rank, StpParams, SwBgaParams, WavCmdKind,
    WavCmdParams,
};
pub use header::{
    BmsHeader, BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderFallback, BmsHeaderGameplay,
    BmsHeaderMetadata, BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming,
};
pub use index::{
    BmpIndex, BmsBase, BmsIndex, BmsIndexError, BpmIndex, ChangeOptionIndex, ChannelIndex,
    ExRankIndex, LnObjIndex, ObjectIndex, ScrollIndex, SeekIndex, SpeedIndex, StopIndex, TextIndex,
    WavIndex, base36_digit_value,
};
pub use message::BmsMessage;
pub use message::parse_message_line;

/// BMS 头部值的统一 trait。
///
/// 将解析（从输入字符串）与格式化（还原为 BMS 值字符串）
/// 合并为单一契约。实现了 [`std::str::FromStr`] +
/// [`std::fmt::Display`] 的类型可获得 blanket 实现——简单的数值或
/// 标识符类型无需手动实现。
///
/// # 格式化
///
/// 此 trait 以 [`std::fmt::Display`] 作为父 trait，而非提供自己的
/// 格式化方法。调用方使用 `.to_string()` 获取 BMS
/// 表示；这使该 trait 与标准库的格式化基础设施保持兼容。
pub trait BmsValue: fmt::Display + Sized {
    /// 将 `s` 解析为 `Self`。
    ///
    /// 返回 `None` 表示解析失败——调用方可以让输入
    /// 回退到 `BmsHeaderFallback`，而非将失败视为硬错误。
    #[must_use]
    fn parse(s: &str) -> Option<Self>;
}

// 覆盖基本类型（f64、u8、i32）、BmsIndex、PoorBgaMode、DifficultyLevel，
// 以及任何已实现 FromStr + Display 的类型。

impl<T: std::str::FromStr + fmt::Display> BmsValue for T {
    #[inline]
    fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

// From / TryFrom 转换

impl TryFrom<BmsToken> for BmsHeader {
    type Error = BmsTryFromError;

    #[inline]
    fn try_from(token: BmsToken) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Header(h) => Ok(h),
            BmsToken::Message(_) => Err(BmsTryFromError::NotAHeader),
        }
    }
}

impl TryFrom<(NonZeroUsize, Result<Self, BmsTokenizeError>)> for BmsToken {
    type Error = BmsTryFromError;

    #[inline]
    fn try_from(pair: (NonZeroUsize, Result<Self, BmsTokenizeError>)) -> Result<Self, Self::Error> {
        let (line, result) = pair;
        result.map_err(|error| BmsTryFromError::TokenizationError { line, error })
    }
}

/// 分词 BMS 文件产生的单个 token。
#[derive(Debug, Clone, PartialEq, derive_more::From)]
pub enum BmsToken {
    /// 头部命令（元数据、游玩、计时、资源等）。
    Header(BmsHeader),
    /// 通道数据行（`#xxxYY:values`）。
    Message(BmsMessage),
}
