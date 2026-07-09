//! **bmson v0.2.1**（遗留）—— camelCase 字段名，`EventNote` 格式。
//!
//! 在 v1.0.0 之前，bmson 混用 `camelCase` 和 `snake_case` 字段名，
//! 使用统一的 `EventNote` 类型同时表示 BPM 和 STOP 事件，并在 `BarLine`
//! 上包含 `k` 字段。
//!
//! # 转换
//!
//! | 方向 | Trait | 说明 |
//! |---|---|---|
//! | `v0::Bmson` → [`crate::Bmson`] | [`TryFrom`] | 丢弃 `BarLine.k`；按上下文映射 `EventNote`；将 `t`→`ln_type_hint`；缺失的 v2 字段填充默认值 |
//! | [`crate::Bmson`] → `v0::Bmson` | [`TryFrom`] | 将 `BpmEvent`+`StopEvent` 合并回 `EventNote`；将 `ln_type_hint`→`t`；缺失的 v0 字段填充默认值 |
//!
//! # Serde
//!
//! | v0 JSON | Rust 字段 |
//! |---|---|
//! | `info`（Bmson 中的键）| `info` |
//! | `lines` | `lines`（与 v1 相同的键）|
//! | `bpmNotes` | `bpm_notes` |
//! | `stopEvents` / `stopNotes` | `stop_events`（两者都接受）|
//! | `soundChannel` | `sound_channels` |
//! | `bgaHeader` / `bgaNotes` / `layerNotes` / `poorNotes` | 复用的 [`crate::BGA`] 处理 |
//! | `ID`（BGAHeader 内部 → 在 [`crate::BGAHeader`] 中处理）| `id` |
//! | `lnType`（BmsonInfo 内部）| `ln_type` |
//! | `titleImage`（BmsonInfo 内部）| `title_image` |
//! | `t`（Note 内部，beatoraja 扩展）| `t` |

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{LnMode, ModeHint};

/// 遗留 v0.2.1 schema 中的顶层 bmson 对象。
///
/// ```json
/// {
///   "info": { "title": "...", "level": 5 },
///   "lines": [{ "y": 960, "k": 0 }],
///   "bpmNotes": [{ "y": 0, "v": 140 }],
///   "stopNotes": [{ "y": 480, "v": 2 }],
///   "soundChannel": [{ "name": "kick.wav", "notes": [...] }],
///   "bga": { "bgaHeader": [...], "bgaNotes": [], ... }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct Bmson<'a> {
    /// 元数据对象。
    pub info: BmsonInfo<'a>,

    /// 小节线（v0 包含 `k` 字段）。
    /// JSON 键为 `"lines"`（与 v1/v2 相同）。
    pub lines: Option<Vec<BarLine>>,

    /// BPM 变更事件（JSON 中为 `bpmNotes`；规范允许为 null）。
    #[serde(
        rename = "bpmNotes",
        default,
        deserialize_with = "crate::null_to_default"
    )]
    pub bpm_notes: Vec<EventNote>,

    /// 停止事件（JSON 中为 `stopEvents` 或 `stopNotes`；规范允许为 null）。
    #[serde(
        rename = "stopEvents",
        alias = "stopNotes",
        default,
        deserialize_with = "crate::null_to_default"
    )]
    pub stop_events: Vec<EventNote>,

    /// 音频通道（JSON 中为 `soundChannel`）。
    #[serde(rename = "soundChannel", default)]
    pub sound_channels: Vec<SoundChannel<'a>>,

    /// 背景动画数据。
    #[serde(rename = "bga")]
    pub bga: crate::BGA<'a>,

    /// 滚动速度事件。
    #[serde(default)]
    pub scroll_events: Vec<crate::ScrollEvent>,
    /// 地雷通道。
    #[serde(default)]
    pub mine_channels: Vec<crate::MineChannel<'a>>,
    /// 不可见按键通道。
    #[serde(default)]
    pub key_channels: Vec<crate::KeyChannel<'a>>,
}

/// v0.2.1 的元数据对象。
///
/// 许多字段在此为可选，因为早期版本尚无这些字段。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BmsonInfo<'a> {
    /// 乐曲标题。
    #[serde(borrow)]
    pub title: &'a str,

    /// 副标题。
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<&'a str>,

    /// 主要艺术家。
    #[serde(borrow)]
    pub artist: &'a str,

    /// 其他贡献者（`["key:value", ...]`）。
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub subartists: Vec<&'a str>,

    /// 流派。
    #[serde(borrow)]
    pub genre: &'a str,

    /// 游戏模式提示（JSON 中为 `modeHint`）。原始 v0 中不存在。
    #[serde(rename = "modeHint", default, skip_serializing_if = "Option::is_none")]
    pub mode_hint: Option<ModeHint>,

    /// 谱面名称（JSON 中为 `chartName`）。
    #[serde(
        rename = "chartName",
        borrow,
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub chart_name: Option<&'a str>,

    /// 数值难度等级。
    ///
    /// 与后来添加的 v1+ 字段不同，`level` 在 v0 中即已存在。
    pub level: u64,

    /// 初始 BPM（JSON 中为 `initBPM`）。
    #[serde(rename = "initBPM")]
    pub init_bpm: f64,

    /// 判定等级（JSON 中为 `judgeRank`，默认 100）。
    #[serde(rename = "judgeRank", default = "default_100")]
    pub judge_rank: f64,

    /// 血量槽总量（默认 100）。
    #[serde(default = "default_100")]
    pub total: f64,

    /// 背景图片（JSON 中为 `backImage`）。
    #[serde(
        rename = "backImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_image: Option<&'a Path>,

    /// 过场图 eyecatch（JSON 中为 `eyecatchImage`）。
    #[serde(
        rename = "eyecatchImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<&'a Path>,

    /// 横幅图片（JSON 中为 `bannerImage`）。
    #[serde(
        rename = "bannerImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<&'a Path>,

    /// 预览音频（JSON 中为 `previewMusic`）。
    #[serde(
        rename = "previewMusic",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<&'a Path>,

    /// 标题图片（JSON 中为 `titleImage`）。
    #[serde(
        rename = "titleImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub title_image: Option<&'a Path>,

    /// 节拍分辨率（默认 240）。
    #[serde(
        default = "crate::default_resolution",
        deserialize_with = "crate::deserialize_resolution_nonzero"
    )]
    pub resolution: u64,

    /// 长音类型 —— beatoraja 扩展（JSON 中为 `lnType`）。
    #[serde(rename = "lnType", default, skip_serializing_if = "Option::is_none")]
    pub ln_type: Option<LnMode>,
}

/// `#[serde(default)]` 在 `resolution` 等字段上使用的默认值 100.0。
const fn default_100() -> f64 {
    100.0
}

/// v0 schema 中的小节线。
///
/// 含有一个额外的 `k` 字段，在 v1.0.0 中被移除。
/// `k` 字段是可选的 —— 官方示例在某些条目中省略了它。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarLine {
    /// 脉冲偏移。
    pub y: u64,
    /// 遗留字段（v1 中已移除）。语义未指定；
    /// 保留用于往返兼容。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k: Option<u64>,
}

/// v0.2.1 中用于 BPM 变更和停止的统一计时事件。
///
/// 在 v1.0.0 中被拆分为 [`crate::BpmEvent`] 和 [`crate::StopEvent`]。
///
/// | 所属数组 | `v` 的含义 |
/// |---|---|
/// | `bpmNotes` | 新的 BPM 值 |
/// | `stopEvents` | 停止时长（脉冲数）|
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventNote {
    /// 脉冲偏移。
    pub y: u64,
    /// 值：BPM（在 `bpmNotes` 中时）或脉冲时长（在 `stopEvents` 中时）。
    pub v: f64,
}

/// v0 schema 中的音频通道。
///
/// 使用字段名 `"notes"`（而非 v2 的 `"note_events"`）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct SoundChannel<'a> {
    /// 音频文件名。
    #[serde(deserialize_with = "crate::de_path")]
    pub name: &'a Path,
    /// 引用此音频文件的音符。
    pub notes: Vec<crate::NoteEvent>,
}

use crate::{BpmEvent, StopEvent};
use crate::{ChartData, ChartInfo, SongInfo};

/// v0 ↔ 根版本转换失败的错误类型。
#[derive(Clone, Debug, derive_more::Display)]
#[display("v0 conversion error: {message}")]
pub struct TryFromV0Error {
    /// 出错原因的人类可读描述。
    pub message: String,
}

impl TryFromV0Error {
    /// 使用描述性消息创建新的转换错误。
    #[must_use]
    #[inline]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::error::Error for TryFromV0Error {}

impl<'a> TryFrom<Bmson<'a>> for crate::Bmson<'a> {
    type Error = TryFromV0Error;

    fn try_from(v0: Bmson<'a>) -> Result<Self, Self::Error> {
        let info = v0.info;

        if info.init_bpm <= 0.0 {
            return Err(TryFromV0Error::new(format!(
                "init_bpm must be positive, got {bpm}",
                bpm = info.init_bpm
            )));
        }

        let song_info = SongInfo {
            title: info.title,
            artist: info.artist,
            genre: info.genre,
        };

        let chart_info = ChartInfo {
            subtitle: info.subtitle.unwrap_or(""),
            subartists: info.subartists,
            chart_name: info.chart_name.unwrap_or(""),
            level: info.level,
            back_image: info.back_image,
            eyecatch_image: info.eyecatch_image,
            banner_image: info.banner_image,
            preview_music: info.preview_music,
            title_image: info.title_image,
            bga: v0.bga,
        };

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "stop duration in v0 may be float; .round() then as u64 is spec-safe"
        )]
        let chart_data = ChartData {
            mode_hint: info.mode_hint.unwrap_or(crate::ModeHint::Beat7k),
            ln_type_hint: info
                .ln_type
                .map_or(crate::LnType::Ln, crate::ln_mode_to_type_hint),
            ln_judge_hint: crate::LnJudge::Normal,
            ln_life_hint: crate::LnLife::Normal,
            init_bpm: info.init_bpm,
            judge_multiplier: info.judge_rank / 100.0,
            life_multiplier: info.total / 100.0,
            resolution: info.resolution,
            lines: v0.lines.map(convert_bar_lines),
            bpm_events: v0
                .bpm_notes
                .into_iter()
                .map(|en| BpmEvent { y: en.y, bpm: en.v })
                .collect(),
            stop_events: v0
                .stop_events
                .into_iter()
                .map(|en| StopEvent {
                    y: en.y,
                    // 负值经 round() as u64 在 Rust 1.45+ 饱和为 0（非法输入被钳位）。
                    duration: en.v.round() as u64,
                })
                .collect(),
            sound_channels: v0
                .sound_channels
                .into_iter()
                .map(|ch| crate::SoundChannel {
                    name: ch.name,
                    note_events: ch
                        .notes
                        .into_iter()
                        .map(|mut note| {
                            // 将 v0 't' 字段 → v2 'ln_type_hint'（如果尚未设置）。
                            if note.ln_type_hint.is_none() {
                                note.ln_type_hint = note.t.map(|t| match t {
                                    LnMode::Ln => crate::LnType::Ln,
                                    LnMode::Cn => crate::LnType::Cn,
                                    LnMode::Hcn => crate::LnType::Hcn,
                                });
                            }
                            note
                        })
                        .collect(),
                })
                .collect(),
            judge_deltas: None,
            life_deltas: None,
        };

        Ok(Self {
            version: "0.2.1",
            song_info,
            chart_info,
            chart_data,
            scroll_events: v0.scroll_events,
            mine_channels: v0.mine_channels,
            key_channels: v0.key_channels,
        })
    }
}

/// 将 v0 `BarLine`（含额外 `k` 字段）转换为通用 `BarLine`。
fn convert_bar_lines(lines: Vec<BarLine>) -> Vec<crate::BarLine> {
    lines
        .into_iter()
        .map(|bl| crate::BarLine { y: bl.y })
        .collect()
}

impl<'a> TryFrom<crate::Bmson<'a>> for Bmson<'a> {
    type Error = TryFromV0Error;

    fn try_from(root: crate::Bmson<'a>) -> Result<Self, Self::Error> {
        let init_bpm = root.chart_data.init_bpm;
        if init_bpm <= 0.0 {
            return Err(TryFromV0Error::new(format!(
                "init_bpm must be positive, got {init_bpm}"
            )));
        }

        let info = BmsonInfo {
            title: root.song_info.title,
            subtitle: some_if_nonempty(root.chart_info.subtitle),
            artist: root.song_info.artist,
            subartists: root.chart_info.subartists,
            genre: root.song_info.genre,
            mode_hint: Some(root.chart_data.mode_hint),
            chart_name: some_if_nonempty(root.chart_info.chart_name),
            level: root.chart_info.level,
            init_bpm,
            judge_rank: root.chart_data.judge_multiplier * 100.0,
            total: root.chart_data.life_multiplier * 100.0,
            back_image: root.chart_info.back_image,
            eyecatch_image: root.chart_info.eyecatch_image,
            banner_image: root.chart_info.banner_image,
            preview_music: root.chart_info.preview_music,
            title_image: root.chart_info.title_image,
            resolution: root.chart_data.resolution,
            ln_type: None,
        };

        let bpm_notes: Vec<EventNote> = root
            .chart_data
            .bpm_events
            .into_iter()
            .map(|e| EventNote { y: e.y, v: e.bpm })
            .collect();

        #[expect(
            clippy::cast_precision_loss,
            reason = "same as bpm_notes conversion; u64 duration → f64"
        )]
        let stop_events: Vec<EventNote> = root
            .chart_data
            .stop_events
            .into_iter()
            .map(|e| EventNote {
                y: e.y,
                v: e.duration as f64,
            })
            .collect();

        Ok(Self {
            info,
            lines: root.chart_data.lines.map(|lines| {
                lines
                    .into_iter()
                    .map(|bl| BarLine {
                        y: bl.y,
                        k: Some(0),
                    })
                    .collect()
            }),
            bpm_notes,
            stop_events,
            sound_channels: root
                .chart_data
                .sound_channels
                .into_iter()
                .map(|ch| SoundChannel {
                    name: ch.name,
                    notes: ch
                        .note_events
                        .into_iter()
                        .map(|mut note| {
                            // 将 v2 'ln_type_hint' → v0 't'（如果尚未设置）。
                            if note.t.is_none() {
                                note.t = note.ln_type_hint.map(|h| match h {
                                    crate::LnType::Ln => LnMode::Ln,
                                    crate::LnType::Cn => LnMode::Cn,
                                    crate::LnType::Hcn => LnMode::Hcn,
                                });
                            }
                            note
                        })
                        .collect(),
                })
                .collect(),
            bga: root.chart_info.bga,
            scroll_events: root.scroll_events,
            mine_channels: root.mine_channels,
            key_channels: root.key_channels,
        })
    }
}

/// 空字符串返回 `None`，否则返回 `Some(s)`。
///
/// 在从根 [`Bmson`] 转换为 [`v0::Bmson`](Bmson) 时使用，
/// 将空的 `&str` 字段映射为 `None`（使其序列化时表现为缺失）。
#[must_use]
pub(crate) const fn some_if_nonempty(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}
