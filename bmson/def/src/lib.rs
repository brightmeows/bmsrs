//! **bmson** 谱面格式的类型定义与转换工具。
//!
//! Bmson 是一种基于 JSON 的 BMS 谱面序列化格式。
//! 该格式经历了三个版本的演进：
//!
//! | 版本 | 状态 | 区分特征 |
//! |---|---|---|
//! | v0.2.1（遗留）| [`v0`] 子模块 | CamelCase 字段名；统一的 `EventNote`；`BarLine` 含 `k` 字段 |
//! | v1.0.0 | [`v1`] 子模块 | `snake_case` 字段名；扁平的 `Bmson` 对象 |
//! | v2.0.0-rc1 | 本模块 | 拆分为 `SongInfo`+`ChartInfo`+`ChartData` |
//!
//! # 版本转换
//!
//! 每个版本模块都实现了 [`From`] / [`TryFrom`]，用于转换到
//! 根（v2）`Bmson`：
//!
//! - [`v1::Bmson`] → [`Bmson`] —— `From`（不会失败，按合理默认值映射）
//! - [`v0::Bmson`] → [`Bmson`] —— `TryFrom`（部分映射有损）
//! - [`Bmson`] → [`v1::Bmson`] —— `From`
//! - [`Bmson`] → [`v0::Bmson`] —— `TryFrom`（往返转换可能丢失扩展字段）
//!
//! # 模块布局
//!
//! | 模块 | 内容 |
//! |---|---|
//! | [`v0`] | v0.2.1 特有类型（`EventNote`、带 `k` 的 `BarLine` 等）|
//! | [`v1`] | v1.0.0 特有类型（扁平 `Bmson`、`BmsonInfo` 等）|
//! | 本模块 | v2.0.0-rc1 类型（`Bmson`、`SongInfo`、`ChartInfo`、`ChartData`）|
//!
//! 跨版本共享的类型（[`NoteEvent`]、[`BpmEvent`]、[`BGA`] 等）从 crate 根
//! re-export，定义在内部 `common` 模块中。

pub mod v0;
pub mod v1;

mod common;

pub use common::{
    BGA, BGAEvent, BGAHeader, BarLine, BpmEvent, KeyChannel, KeyNote, LnJudge, LnLife, LnMode,
    LnType, MineChannel, MineNote, ModeHint, NoteEvent, ScrollEvent, SoundChannel, StopEvent,
};

pub use common::{
    de_opt_path, de_path, default_multiplier, default_resolution, deserialize_resolution_nonzero,
    null_to_default,
};

use serde::{Deserialize, Serialize};
use std::path::Path;

/// DJ.NEXT 播放器引入的自定义判定窗口偏移。
///
/// 每个字段指定一个**额外**量（毫秒），加到播放器该判定等级的默认窗口上。
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgementDeltas {
    /// PERFECT 窗口的额外偏移（毫秒）。
    pub perfect: u64,
    /// GREAT 窗口的额外偏移（毫秒）。
    pub great: u64,
    /// GOOD 窗口的额外偏移（毫秒）。
    pub good: u64,
    /// MISS 窗口的额外偏移（毫秒）。
    pub miss: u64,
}

/// DJ.NEXT 播放器引入的自定义血量槽增量。
///
/// 每个字段指定该判定等级的血量变化（以血量槽百分比表示）。
/// 负值表示扣除血量。
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifeDeltas {
    /// PERFECT 时的血量变化（百分比，可为负）。
    pub perfect: f64,
    /// GREAT 时的血量变化（百分比，可为负）。
    pub great: f64,
    /// GOOD 时的血量变化（百分比，可为负）。
    pub good: f64,
    /// MISS 时的血量变化（百分比，可为负）。
    pub miss: f64,
}

/// bmson 谱面的根对象（v2.0.0-rc1 schema）。
///
/// 在 v2 中，v1/v0 的扁平 `Bmson` 被拆分为三个子对象：
///
/// | 对象 | 职责 |
/// |---|---|---|
/// | [`SongInfo`] | 乐曲元数据（标题、艺术家、流派）|
/// | [`ChartInfo`] | 谱面级元数据（难度、图片、BGA）|
/// | [`ChartData`] | 实际谱面数据（音符、计时、音频通道）|
///
///
/// # beatoraja 扩展
///
/// 可选字段 `scroll_events`、`mine_channels` 和 `key_channels`
/// 是 beatoraja 特有的扩展，不属于 bmson 核心规范。
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bmson<'a> {
    /// bmson 格式版本字符串。
    ///
    /// 必须是合法的 `SemVer` 字符串。v2 文件的值为 `"2.0.0"`。
    /// 如果 `version` 缺失（`null`），播放器应拒绝该文件或将其视为旧版本格式。
    ///
    #[serde(borrow)]
    pub version: &'a str,

    /// 乐曲级元数据（标题、艺术家、流派）。
    #[serde(rename = "song_info")]
    pub song_info: SongInfo<'a>,

    /// 谱面级元数据（难度、图片、BGA）。
    #[serde(rename = "chart_info")]
    pub chart_info: ChartInfo<'a>,

    /// 谱面数据（音符、计时、音频通道）。
    #[serde(rename = "chart_data")]
    pub chart_data: ChartData<'a>,

    /// 滚动速度变更事件（beatoraja 0.7.6+）。
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,

    /// 地雷（landmine）通道（beatoraja 扩展）。
    #[serde(default)]
    pub mine_channels: Vec<MineChannel<'a>>,

    /// 不可见（"key"）通道（beatoraja 扩展）。
    #[serde(default)]
    pub key_channels: Vec<KeyChannel<'a>>,
}

/// 乐曲级元数据（v2.0.0-rc1）。
///
/// 从 v1 的旧 `BmsonInfo` 中提取。仅包含描述**乐曲本身**的字段
/// （而非某个具体谱面）。
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongInfo<'a> {
    /// 乐曲标题。
    ///
    /// 播放器必须原样显示，不得按 `()` 或 `--` 等分隔符拆分。
    ///
    #[serde(borrow)]
    pub title: &'a str,

    /// 主要艺术家。
    ///
    /// 通常是音乐作曲者。可包含多个名字，以 `vs`、`feat.` 等分隔。
    ///
    #[serde(borrow)]
    pub artist: &'a str,

    /// 乐曲流派。
    ///
    #[serde(borrow)]
    pub genre: &'a str,
}

/// 谱面级元数据（v2.0.0-rc1）。
///
/// 承载某首乐曲的一个具体谱面编排的难度信息、显示素材和 BGA。
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartInfo<'a> {
    /// 谱面副标题。
    ///
    /// 以较小字体显示在 [`SongInfo::title`] 下方。
    /// 可包含 `\n` 实现多行副标题。
    ///
    #[serde(borrow, default)]
    pub subtitle: &'a str,

    /// 主要艺术家以外的贡献者。
    ///
    /// 每个条目格式为 `"key:value"`，其中 `key` 取值之一为：
    /// `music`、`vocal`、`chart`、`image`、`movie`、`other`。
    /// 如果省略 `key`，则默认为 `other`。
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub subartists: Vec<&'a str>,

    /// 谱面名称 / 难度标签。
    ///
    /// 示例：`"BEGINNER"`、`"HYPER"`、`"ANOTHER"`。
    ///
    #[serde(borrow, default)]
    pub chart_name: &'a str,

    /// 数值难度等级。
    ///
    /// 必须 ≥ 0。`beat-7k` 模式下通常取值范围为 1–12。
    ///
    pub level: u64,

    /// **游玩时**显示的背景图片。
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_image: Option<&'a Path>,

    /// **乐曲加载时**显示的过场图（eyecatch）。
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<&'a Path>,

    /// **选曲和结算界面**使用的横幅图片。
    ///
    /// 推荐宽高比：15 : 4（例如 600×160）。
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<&'a Path>,

    /// 短预览音频文件路径。
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<&'a Path>,

    /// **游玩开始前**显示的标题图片。
    ///
    /// 等价于 OADX+ 皮肤系统中的 `#BACKBMP`。
    /// 如果缺失，播放器以默认字体显示标题。
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub title_image: Option<&'a Path>,

    /// 背景动画数据。
    #[serde(rename = "bga")]
    pub bga: BGA<'a>,
}

/// 实际谱面数据（v2.0.0-rc1）。
///
/// 包含播放谱面所需的一切：计时、音符、音频通道、长音提示，
/// 以及可选的 DJ.NEXT 扩展。
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct ChartData<'a> {
    /// 游戏模式提示。
    ///
    /// 播放器应检查此字段，确认谱面与其输入布局兼容。
    ///
    #[serde(default)]
    pub mode_hint: ModeHint,

    /// 谱面级长音（long-note）类型提示。
    ///
    /// | 值 | 含义 |
    /// |---|---|
    /// | `ln` | 仅在按下时判定 |
    /// | `cn` | 在按下和释放时都判定 |
    ///
    /// 可通过 [`NoteEvent::ln_type_hint`] 按音符覆盖。
    ///
    #[serde(default)]
    pub ln_type_hint: LnType,

    /// 谱面级长音判定提示。
    ///
    /// | 值 | 含义 |
    /// |---|---|
    /// | `normal` | 仅判定音符本身 |
    /// | `ticks` | 按住期间额外判定若干脉冲 |
    ///
    #[serde(default)]
    pub ln_judge_hint: LnJudge,

    /// 谱面级长音血量槽（life）提示。
    ///
    /// | 值 | 含义 |
    /// |---|---|
    /// | `normal` | 仅音符本身恢复血量 |
    /// | `ticks` | 按住期间额外脉冲恢复血量 |
    ///
    #[serde(default)]
    pub ln_life_hint: LnLife,

    /// 乐曲开始时的初始 BPM（每分钟拍数）。
    ///
    /// 如果省略则视为文件格式错误。
    ///
    pub init_bpm: f64,

    /// 判定窗口倍率（v2）。
    ///
    /// 默认值：`1.00`。
    ///
    /// | 值 | 窗口宽度 |
    /// |---|---|
    /// | `1.00` | 默认（因播放器而异）|
    /// | `>1.00` | 更宽（更易）|
    /// | `<1.00` | 更窄（更难）|
    ///
    /// 在 v1 中此字段为 `judge_rank`（0–100 量表）；转换公式为
    /// `judge_multiplier = judge_rank / 100.0`。
    ///
    #[serde(default = "default_multiplier")]
    pub judge_multiplier: f64,

    /// 血量槽倍率（v2）。
    ///
    /// 默认值：`1.00`。
    ///
    /// 大于 1.00 的值使血量槽充能更快；小于 1.00 的值使充能更慢。
    /// 必须 ≥ 0。在 v1 中此字段为 `total`（0–100 量表）；转换公式为
    /// `life_multiplier = total / 100.0`。
    ///
    #[serde(default = "default_multiplier")]
    pub life_multiplier: f64,

    /// 每四分音符的脉冲数（节拍分辨率）。
    ///
    /// 默认值：`240`。必须 > 0。
    ///
    /// 240 是 48（常见 BMS 分辨率）与 5 的最小公倍数，可支持五连音节奏。
    ///
    #[serde(
        default = "default_resolution",
        deserialize_with = "deserialize_resolution_nonzero"
    )]
    pub resolution: u64,

    /// 小节线位置（小节边界）。
    ///
    /// `None` → 自动生成 4/4 小节线。
    /// `Some([])` → 不显示小节线。
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<BarLine>>,

    /// BPM 变更事件。
    ///
    /// 规范将此字段标记为可空（`BpmEvent[]?`）；
    /// `null` 视为空数组。
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub bpm_events: Vec<BpmEvent>,

    /// 停止（暂停）事件。
    ///
    /// 规范将此字段标记为可空（`StopEvent[]?`）；
    /// `null` 视为空数组。
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub stop_events: Vec<StopEvent>,

    /// 音频通道 —— 每个通道将一个音频文件与其音符绑定在一起。
    #[serde(default)]
    pub sound_channels: Vec<SoundChannel<'a>>,

    /// 自定义判定窗口偏移（DJ.NEXT 扩展）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_deltas: Option<JudgementDeltas>,

    /// 自定义血量槽增量（DJ.NEXT 扩展）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_deltas: Option<LifeDeltas>,
}

/// bmson 版本检测或格式转换时可能发生的错误。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BmsonError {
    /// 版本字符串存在但无法识别。
    ///
    /// 仅支持以 `"0"`（v0.2.1 遗留）、`"1"`（v1.0.0）或 `"2"`（v2.0.0-rc1）
    /// 开头的版本。
    #[error("unknown bmson version: {0}")]
    UnknownVersion(String),

    /// 从遗留 v0.2.1 格式的转换失败。
    #[error("v0 conversion error: {0}")]
    V0Conversion(String),
}

/// 检测到的 bmson 格式版本，通过检查 JSON 谱面文件的 `"version"` 字段确定。
///
/// | 变体 | 检测依据 |
/// |---|---|
/// | [`V0`](DetectedVersion::V0) | 无 `"version"` 字段（遗留）|
/// | [`V1`](DetectedVersion::V1) | `"version"` 以 `"1"` 开头 |
/// | [`V2`](DetectedVersion::V2) | `"version"` 以 `"2"` 开头 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedVersion {
    /// v0.2.1（遗留，无 `version` 字段）。
    V0,
    /// v1.0.0（扁平 schema）。
    V1,
    /// v2.0.0-rc1（拆分 schema）。
    V2,
}

/// 通过扫描 JSON 谱面文件的 `"version"` 字段来检测 bmson 格式版本。
///
/// 此函数执行**轻量级字符串扫描**而非完整 JSON 解析，因此适合作为
/// 分发到版本特有反序列化器之前的第一步。
///
/// # 检测逻辑
///
/// 1. 在 JSON 文本中搜索字面量 `"version"` 键。
/// 2. 如果找到，提取冒号后的字符串值。
/// 3. 如果值以 `'2'` 开头 → [`V2`](DetectedVersion::V2)。
/// 4. 如果值以 `'1'` 开头 → [`V1`](DetectedVersion::V1)。
/// 5. 如果值以 `'0'` 开头 → [`V0`](DetectedVersion::V0)。
/// 6. 否则 → [`BmsonError::UnknownVersion`]。
/// 7. 如果 `"version"` 不存在 → [`V0`](DetectedVersion::V0)（遗留）。
///
/// # 示例
///
/// ```rust
/// # use bmson_def::DetectedVersion;
/// let json = r#"{"version":"2.0.0","song_info":{}}"#;
/// assert_eq!(bmson_def::detect_version(json).unwrap(), DetectedVersion::V2);
/// ```
///
/// # Errors
///
/// 当找到 `"version"` 字段但其值不是字符串，或不以 `'0'`、`'1'`、`'2'`
/// 开头时，返回 [`BmsonError::UnknownVersion`]。
#[expect(
    clippy::string_slice,
    reason = "JSON bytes for \"version\" key and ASCII version strings; byte indexing is safe"
)]
pub fn detect_version(json: &str) -> Result<DetectedVersion, BmsonError> {
    // 扫描字面量子串以查找 `"version"` 键。
    let Some(key_pos) = json.find("\"version\"") else {
        return Ok(DetectedVersion::V0);
    };

    let mut rest = &json[key_pos + 9..];
    // 跳过空白，预期 `:`。
    rest = rest.trim_start();
    rest = rest.strip_prefix(':').ok_or_else(|| {
        BmsonError::UnknownVersion("malformed version field: expected ':'".into())
    })?;
    // 跳过空白，预期开头的 `"`。
    rest = rest.trim_start();
    rest = rest.strip_prefix('"').ok_or_else(|| {
        BmsonError::UnknownVersion("malformed version field: expected string".into())
    })?;
    // 查找闭合的 `"`。
    let end = rest
        .find('"')
        .ok_or_else(|| BmsonError::UnknownVersion("unterminated version string".into()))?;
    let version = &rest[..end];

    if version.starts_with('2') {
        Ok(DetectedVersion::V2)
    } else if version.starts_with('1') {
        Ok(DetectedVersion::V1)
    } else if version.starts_with('0') {
        Ok(DetectedVersion::V0)
    } else {
        Err(BmsonError::UnknownVersion(version.to_owned()))
    }
}

/// 将 [`LnMode`]（数值 1/2/3，含 HCN）映射到 [`LnType`]（ln/cn，不含 HCN）。
///
/// HCN 和 Ln 均映射为 `None`（前者无等价、后者为默认值），
/// 由调用方通过 `unwrap_or(LnType::Ln)` 处理。
#[must_use]
pub(crate) const fn ln_mode_to_type_hint(lt: LnMode) -> Option<LnType> {
    match lt {
        LnMode::Cn => Some(LnType::Cn),
        LnMode::Hcn | LnMode::Ln => None,
    }
}
