//! BMS 头部命令解析，按语义域分类。

mod control_flow;
mod display;
mod gameplay;
mod metadata;
mod res_def_audio;
mod res_def_visual;
mod timing;

pub use control_flow::BmsHeaderControlFlow;
pub use display::{BmsHeaderDisplay, DifficultyLevel, ParseDifficultyError, PoorBgaMode};
pub use gameplay::{BmsHeaderGameplay, LnMode, LnType, PlayerMode, Rank};
pub use metadata::BmsHeaderMetadata;
pub use res_def_audio::{BmsHeaderResDefAudio, ExWavParams, WavCmdKind, WavCmdParams};
pub use res_def_visual::{
    ArgbParams, AtBgaParams, BgaParams, BmsHeaderResDefVisual, ExBmpParams, SwBgaParams,
};
pub use timing::{BmsHeaderTiming, StpParams};

use std::fmt;

use crate::BmsTokenAttr;
use crate::BmsTokenizeError;
use crate::BmsTryFromError;

/// BMS 文件中的头部命令，按语义域分类。
///
/// 分发顺序遵循下方变体声明顺序。
/// 标注了 `#[bms_fallback]` 的变体不参与分发，
/// 而是捕获任何未匹配具体变体的内容。
#[derive(Debug, Clone, PartialEq, BmsTokenAttr, derive_more::From)]
pub enum BmsHeader<C> {
    /// 音频资源定义（`#WAV`、`#EXWAV`、`#WAVCMD` 等）。
    ResDefAudio(BmsHeaderResDefAudio<C>),
    /// 计时定义（`#BPM`、`#STOP`、`#SCROLL`、`#SPEED` 等）。
    Timing(BmsHeaderTiming),
    /// 视觉资源定义（`#BMP`、`#BGA`、`#ARGB` 等）。
    ResDefVisual(BmsHeaderResDefVisual<C>),
    /// 控制流命令（`#RANDOM`、`#SWITCH`、`#IF` 等）。
    ControlFlow(BmsHeaderControlFlow),
    /// 游玩行为（`#PLAYER`、`#RANK`、`#TOTAL`、`#LNTYPE` 等）。
    Gameplay(BmsHeaderGameplay<C>),
    /// 显示与难度标记（`#STAGEFILE`、`#DIFFICULTY` 等）。
    Display(BmsHeaderDisplay<C>),
    /// 乐曲/谱面标识（`#TITLE`、`#ARTIST`、`#GENRE` 等）。
    Metadata(BmsHeaderMetadata<C>),
    /// 无法识别或引擎特有的头部命令。
    #[bms_fallback]
    Fallback(BmsHeaderFallback<C>),
}

/// 无法识别的头部命令的兜底类型。
///
/// 捕获原始命令名与值，使下游消费方（解析器、工具）
/// 能处理分词器不认识的引擎扩展。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsHeaderFallback<C> {
    /// 文件中出现的原始命令名（例如 `"MYEXT"`）。
    pub command: C,
    /// 空格分隔符之后的值。
    pub value: C,
}

// From / TryFrom 转换

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderFallback<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Fallback(f) => Ok(f),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}

/// 将单行头部行解析为 `BmsHeader`。
///
/// `prefixes` 控制哪些起始字符被识别为头部标记
/// （默认：`#` 和 `%`）。不以任一配置前缀开头的行被跳过。
///
/// 若该行不是头部（空行、注释、通道数据）则返回 `Ok(None)`。
///
/// # Errors
///
/// 当头部命令被识别但其值无法解析为预期类型时，
/// 返回 `Err(BmsTokenizeError)`。
/// 为 C 显式解析父 trait（避免 `BmsStr` blanket impl 的 `E0283`）。
/// 在测试中，使用非泛型的 `parse_header_line_default` 包装。
#[expect(
    clippy::string_slice,
    reason = "BMS header lines are ASCII-only; byte indexing at whitespace boundaries is safe"
)]
pub fn parse_header_line<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a>(
    line: &'a str,
    prefixes: &[char],
) -> Result<Option<BmsHeader<C>>, BmsTokenizeError<C>> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    // 以 `##` 开头的行是注释。
    if trimmed.starts_with("##") {
        return Ok(None);
    }

    // 从配置列表中确定前缀字符。
    // trimmed 非空（已在上面检查）。
    let Some(first) = trimmed.chars().next() else {
        return Ok(None);
    };
    if !prefixes.contains(&first) {
        return Ok(None);
    }
    let prefix = first;
    let rest = &trimmed[prefix.len_utf8()..];

    if rest.is_empty() {
        return Ok(None);
    }

    // 在首个空白处拆分命令名与值。
    let split_pos = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let command = &rest[..split_pos];
    let value = rest[split_pos..].trim();

    if command.is_empty() {
        return Ok(None);
    }

    // 若原始命令含冒号，则它是通道消息，而非头部。
    if command.contains(':') {
        return Ok(None);
    }

    // `%` 命令：仅 `%URL` 与 `%EMAIL` 是有效的 BMS 头部——
    // 其他一切都是引擎特有的扩展。未知的 `%` 命令
    // 不得回退（否则会让 `%TITLE` 冒充
    // `#TITLE`）。
    if prefix == '%'
        && !command.eq_ignore_ascii_case("URL")
        && !command.eq_ignore_ascii_case("EMAIL")
    {
        return Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
            command: C::from(command),
            value: C::from(value),
        })));
    }

    // 分发给子枚举的 `try_match_header` 函数。
    // `command` 携带输入生命周期，使索引切片错误
    // 能存储命令的正确部分。
    if let Some(header) = BmsHeader::try_match_header(command, value)? {
        return Ok(Some(header));
    }

    // 无匹配项 → Fallback。
    Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
        command: C::from(command),
        value: C::from(value),
    })))
}
