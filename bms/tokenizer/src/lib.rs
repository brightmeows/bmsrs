//! BMS（Be-Music Script）格式的分词器。
//!
//! 本 crate 提供 BMS 解析管道的第一阶段：
//! 将原始 BMS 文本转换为结构化的 token 流。
//!
//! # 架构
//!
//! - [`enum@BmsToken`] 表示 BMS 文件中一行有意义的内容。
//! - [`BmsHeader`] 涵盖所有头部命令，按语义域分类。
//! - [`BmsMessage`] 涵盖通道数据行（`#xxxYY:values`）。
//! - [`BmsTokenizer`] 是主入口，可配置错误处理策略。
//!
//! 子模块为私有；所有公开类型均从 crate 根重新导出。
//!
//! # 字符串存储
//!
//! 字符串数据使用类型参数 `C`（默认为 `&str` 实现零拷贝，
//! 或 `String` 为 owned）。类型化值（数值转换、解析出的枚举）
//! 始终为 owned。
//! [`ErrorStrategy::FailFast`] 在首个错误处停止——适用于
//! 偏好即时反馈的交互式校验场景。

use std::fmt;
use std::num::NonZeroUsize;

mod channel;
pub mod encoding;
mod error;
mod header;
mod index;
mod message;
mod preprocess;

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
pub use preprocess::preprocess;

pub use encoding::BmsEncoding;

/// BMS 头部值的统一 trait。
///
/// 将解析（从输入字符串）与格式化（还原为 BMS 值字符串）
/// 合并为单一契约。实现了 [`std::str::FromStr`] +
/// [`std::fmt::Display`] 的类型可获得 blanket 实现——简单的数值或
/// 标识符类型无需手动实现。
///
/// # 生命周期
///
/// `'a` 生命周期是输入字符串的生命周期——实现可以
/// 借用它而无需分配（例如 `ExBmpParams<'a>`）。仅 owned 的
/// 类型可以用任意 `'a` 安全地实现此 trait。
///
/// # 类型参数
///
/// `C` 是下游使用的字符串容器类型（例如 `&str`、`String`、
/// `Cow<'_, str>`）。该参数的存在使消费方可以在
/// 零拷贝与 owned 存储之间选择。
///
/// # 格式化
///
/// 此 trait 以 [`std::fmt::Display`] 作为父 trait，而非提供自己的
/// 格式化方法。调用方使用 `.to_string()` 获取 BMS
/// 表示；这使该 trait 与标准库的格式化基础设施保持兼容。
pub trait BmsValue<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a = &'a str>:
    fmt::Display + Sized
{
    /// 将 `s` 解析为 `Self`。
    ///
    /// 返回 `None` 表示解析失败——调用方可以让输入
    /// 回退到 `BmsHeaderFallback`，而非将失败视为硬错误。
    #[must_use]
    fn parse(s: &'a str) -> Option<Self>;
}

// 覆盖基本类型（f64、u8、i32）、BmsIndex、PoorBgaMode、DifficultyLevel，
// 以及任何已实现 FromStr + Display 的类型。

impl<'a, C, T> BmsValue<'a, C> for T
where
    C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a,
    T: std::str::FromStr + fmt::Display,
{
    #[inline]
    fn parse(s: &'a str) -> Option<Self> {
        s.parse().ok()
    }
}

// From / TryFrom 转换

impl<C> TryFrom<BmsToken<C>> for BmsHeader<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(token: BmsToken<C>) -> Result<Self, Self::Error> {
        match token {
            BmsToken::Header(h) => Ok(h),
            BmsToken::Message(_) => Err(BmsTryFromError::NotAHeader),
        }
    }
}

impl<C> TryFrom<(NonZeroUsize, Result<Self, BmsTokenizeError<C>>)> for BmsToken<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(
        pair: (NonZeroUsize, Result<Self, BmsTokenizeError<C>>),
    ) -> Result<Self, Self::Error> {
        let (line, result) = pair;
        result.map_err(|error| BmsTryFromError::TokenizationError { line, error })
    }
}

/// 分词 BMS 文件产生的单个 token。
#[derive(Debug, Clone, PartialEq, derive_more::From)]
pub enum BmsToken<C> {
    /// 头部命令（元数据、游玩、计时、资源等）。
    Header(BmsHeader<C>),
    /// 通道数据行（`#xxxYY:values`）。
    Message(BmsMessage<C>),
}

/// BMS 分词的错误处理策略。
///
/// 控制分词器如何处理解析失败的行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// 处理每一行；每行的结果单独包装。
    /// 分词会越过错误继续，使调用方可以一次检视所有失败。
    #[default]
    CollectAll,
    /// 在首个错误处停止，返回至（并包含）
    /// 该行的结果。后续行不被检视。
    FailFast,
}

/// [`tokenize`](BmsTokenizer::tokenize) 返回的 owned token 向量类型。
///
/// 等同于 `Vec<(NonZeroUsize, Result<BmsToken<String>, BmsTokenizeError<String>>)>`。
type TokenizeOwnedResult = Vec<(
    NonZeroUsize,
    Result<BmsToken<String>, BmsTokenizeError<String>>,
)>;

/// 在 `s` 中查找第一个不在 `"..."` 字符串字面量内的 `//`。
///
/// 返回 `//` 的起始位置；若不存在行内注释则返回 `None`。
///
/// `//` 仅当前一字符为 ASCII 空白（或位于行首）时才视为注释起始，
/// 避免 URL 中的 `//`（如 `https://`）被误判为注释。
#[expect(
    clippy::indexing_slicing,
    reason = "i < bytes.len() 由 while 循环边界保证"
)]
fn find_inline_comment(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                in_string = !in_string;
                i += 1;
            }
            b'/' if !in_string
                && i + 1 < bytes.len()
                && bytes[i + 1] == b'/'
                && (i == 0 || bytes[i - 1].is_ascii_whitespace()) =>
            {
                return Some(i);
            }
            _ => {
                i += 1;
            }
        }
    }
    None
}

/// 采用 builder 风格配置的 BMS 分词器。
///
/// # 示例
///
/// ```
/// # use bms_tokenizer::{BmsTokenizer, ErrorStrategy, BmsEncoding};
/// let tokens: Vec<_> = BmsTokenizer::new()
///     .error_strategy(ErrorStrategy::CollectAll)
///     .encoding(BmsEncoding::ShiftJis)
///     .tokenize::<_, &str>("#TITLE My Song\n#00101:11");
/// ```
#[derive(Debug, Clone)]
pub struct BmsTokenizer {
    /// 控制如何处理解析错误。
    error_strategy: ErrorStrategy,
    /// 允许的头部命令前缀字符。
    header_prefixes: Vec<char>,
    /// 预设的编码（可选）。设置后 `tokenize_bytes` 跳过检测直接使用此编码。
    encoding: Option<BmsEncoding>,
}

impl Default for BmsTokenizer {
    fn default() -> Self {
        Self {
            error_strategy: ErrorStrategy::default(),
            header_prefixes: vec!['#', '%'],
            encoding: None,
        }
    }
}

impl BmsTokenizer {
    /// 创建一个使用默认配置
    /// （[`ErrorStrategy::CollectAll`]、前缀 `#` 和 `%`）的 `BmsTokenizer`。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置错误处理策略。
    #[must_use]
    pub const fn error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }

    /// 设置允许的头部前缀字符。
    ///
    /// 仅以这些字符之一开头的行才会被视为
    /// 潜在的头部命令。默认：`['#', '%']`。
    #[must_use]
    pub fn header_prefixes(mut self, prefixes: &[char]) -> Self {
        self.header_prefixes = prefixes.to_vec();
        self
    }

    /// 预设文件编码。设置后在调用 [`tokenize_bytes`](Self::tokenize_bytes) 时
    /// 跳过自动检测，直接使用此编码解码。
    ///
    /// 不影响 [`tokenize`](Self::tokenize)（后者始终接受 UTF-8 `&str`）。
    #[must_use]
    pub const fn encoding(mut self, encoding: BmsEncoding) -> Self {
        self.encoding = Some(encoding);
        self
    }

    /// 将 BMS 字符串分词为带行号的结果集合。
    ///
    /// 每个元素是一个 `(从 1 开始的行号, 结果)` 元组。
    /// 空行与注释被跳过（不产生输出项）。
    ///
    /// 支持 LF（`\n`）、CRLF（`\r\n`）以及独立的 CR（`\r`）行尾。
    ///
    /// # 类型参数
    ///
    /// - `Out` —— 输出集合类型（例如 `Vec`、`Box<[_]>`）。
    /// - `C` —— 字符串容器类型。默认为 `&'a str`
    ///   以实现零拷贝分词。
    ///
    /// # 错误策略
    ///
    /// - [`ErrorStrategy::CollectAll`]（默认）：处理所有行。
    ///   错误以逐元素的 [`Result::Err`] 嵌入。
    /// - [`ErrorStrategy::FailFast`]：在首个错误处停止。集合
    ///   包含至（并包含）错误行为止的结果。
    ///
    /// # Panics
    ///
    /// 输入行数超过 `usize` 上限时 panic（实际 BMS 文件不可能达到此上限）。
    #[must_use]
    pub fn tokenize<'a, Out, C>(&self, input: &'a str) -> Out
    where
        Out: FromIterator<(NonZeroUsize, Result<BmsToken<C>, BmsTokenizeError<C>>)>,
        C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a,
    {
        let mut results = Vec::new();
        let mut line_number: usize = 0;

        for raw_line in input.lines() {
            for segment in raw_line.split('\r') {
                line_number += 1;

                let trimmed = segment.trim();
                if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with(';') {
                    continue;
                }

                // 处理行内 // 注释（跳过 "..." 字符串内的）
                let line = match find_inline_comment(trimmed) {
                    Some(pos) => {
                        // 安全：pos 位于 ASCII `//` 处，保证为字符边界
                        let Some(head) = trimmed.get(..pos) else {
                            continue;
                        };
                        let trimmed_head = head.trim_end();
                        if trimmed_head.is_empty() {
                            continue;
                        }
                        trimmed_head
                    }
                    None => trimmed,
                };

                // `line_number` 首次使用前从 0 自增，此处永远 >= 1。
                #[expect(
                    clippy::expect_used,
                    reason = "line_number always >= 1, guaranteed by increment-before-use"
                )]
                let nz_line = NonZeroUsize::new(line_number)
                    .expect("line_number always >= 1, guaranteed by increment-before-use");

                let result: Result<BmsToken<C>, BmsTokenizeError<C>> =
                    match parse_message_line::<C>(line) {
                        Ok(Some(msg)) => Ok(BmsToken::Message(msg)),
                        Ok(None) => match parse_header_line::<C>(line, &self.header_prefixes) {
                            Ok(Some(hdr)) => Ok(BmsToken::Header(hdr)),
                            Ok(None) => continue,
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    };

                if self.error_strategy == ErrorStrategy::FailFast && result.is_err() {
                    results.push((nz_line, result));
                    return results.into_iter().collect();
                }

                results.push((nz_line, result));
            }
        }

        results.into_iter().collect()
    }

    /// 将 BMS 字符串分词为 owned token（`C = String`）。
    ///
    /// 等价于 `tokenize::<Vec<_>, String>(input)`，适合需要在分词完成后
    /// 释放原输入字符串的场景（如读取文件到 `String` 后分词）。
    ///
    /// # Panics
    ///
    /// 参见 [`tokenize`](Self::tokenize) 的 Panics 说明——条件相同。
    #[must_use]
    pub fn tokenize_owned(&self, input: &str) -> TokenizeOwnedResult {
        self.tokenize(input)
    }

    /// 将 BMS 字节数据分词为 owned token。
    ///
    /// 内部自动检测编码（除非通过 [`encoding`](Self::encoding) 预设），
    /// 解码为 UTF-8 后调用 [`tokenize`](Self::tokenize)。
    ///
    /// 始终产生 `C = String` 的 owned token。
    ///
    /// # Panics
    ///
    /// 参见 [`tokenize`](Self::tokenize) 的 Panics 说明——条件相同。
    #[must_use]
    pub fn tokenize_bytes(&self, input: &[u8]) -> TokenizeOwnedResult {
        let encoding = self.encoding.unwrap_or_else(|| BmsEncoding::detect(input));
        let decoded = encoding.decode(input);
        self.tokenize(&decoded)
    }
}
