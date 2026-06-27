//! **bmson v1.0.0** —— 扁平 schema。
//!
//! 此模块对应 v1.0.0 规范，其中 `Bmson` 是一个扁平对象，同时包含元数据
//! 和谱面数据。
//!
//! # 转换
//!
//! [`Bmson`] ↔ [`crate::Bmson`] 通过 [`From`]。
//!
//! v1 → 根（v2）将 [`BmsonInfo`] 拆分到 [`crate::SongInfo`]、
//! [`crate::ChartInfo`] 和 [`crate::ChartData`]。
//! 根 → v1 将它们合并回去。
//!
//! # Serde
//!
//! 顶层类型使用 v1 特有的字段名（`info`、`sound_channels` / `notes`
//! 作为根键）。叶节点类型从根模块复用。

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{BGA, BarLine, BpmEvent, KeyChannel, MineChannel, ModeHint, ScrollEvent, StopEvent};

/// v1.0.0 schema 中的顶层 bmson 对象。
///
/// ```json
/// {
///   "version": "1.0.0",
///   "info": { "title": "...", ... },
///   "lines": [ ... ],
///   "bpm_events": [ ... ],
///   "stop_events": [ ... ],
///   "sound_channels": [ ... ],
///   "bga": { ... }
/// }
/// ```
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bmson<'a> {
    /// bmson 格式版本（应为 `"1.0.0"`）。
    #[serde(borrow)]
    pub version: &'a str,

    /// 元数据对象。
    pub info: BmsonInfo<'a>,

    /// 小节线位置。`None` → 4/4 自动生成，`Some([])` → 无小节线。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<BarLine>>,

    /// BPM 变更事件（规范允许为 null；null 视为空）。
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub bpm_events: Vec<BpmEvent>,

    /// 停止（暂停）事件（规范允许为 null；null 视为空）。
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub stop_events: Vec<StopEvent>,

    /// 音频通道（v1 在每个通道内使用 `notes`）。
    #[serde(default)]
    pub sound_channels: Vec<SoundChannel<'a>>,

    /// 背景动画数据。
    pub bga: BGA<'a>,

    /// 滚动速度事件。
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,
    /// 地雷通道。
    #[serde(default)]
    pub mine_channels: Vec<MineChannel<'a>>,
    /// 不可见按键通道。
    #[serde(default)]
    pub key_channels: Vec<KeyChannel<'a>>,
}

/// v1 schema 中的元数据对象。
///
/// 包含乐曲和此具体谱面的所有信息。
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BmsonInfo<'a> {
    /// 乐曲标题。
    #[serde(borrow)]
    pub title: &'a str,

    /// 副标题（默认 `""`）。
    #[serde(borrow, default)]
    pub subtitle: &'a str,

    /// 主要艺术家。
    #[serde(borrow)]
    pub artist: &'a str,

    /// 其他贡献者（`["key:value", ...]`）。
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub subartists: Vec<&'a str>,

    /// 流派。
    #[serde(borrow)]
    pub genre: &'a str,

    /// 游戏模式提示（默认 `"beat-7k"`）。
    #[serde(default)]
    pub mode_hint: ModeHint,

    /// 谱面名称 / 难度标签（默认 `""`）。
    #[serde(borrow, default)]
    pub chart_name: &'a str,

    /// 数值难度等级。
    pub level: u64,

    /// 初始 BPM。
    pub init_bpm: f64,

    /// 判定窗口（0–100 量表，默认 100）。
    #[serde(default = "default_100")]
    pub judge_rank: f64,

    /// 血量槽充能量（0–100 量表，默认 100）。
    #[serde(default = "default_100")]
    pub total: f64,

    /// 背景图片（游玩时）。
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_image: Option<&'a Path>,

    /// 过场图 eyecatch（加载界面）。
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<&'a Path>,

    /// 横幅图片（选曲/结算界面）。
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<&'a Path>,

    /// 预览音频路径。
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<&'a Path>,

    /// 游玩开始前显示的标题图片。
    /// 等价于 OADX+ 皮肤系统中的 `#BACKBMP`。
    #[serde(
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
}

/// `#[serde(default)]` 在 `resolution` 等字段上使用的默认值 100.0。
const fn default_100() -> f64 {
    100.0
}

/// v1 schema 中的音频通道。
///
/// 与 [`crate::SoundChannel`] 相同，区别在于音符字段为
/// `notes`（v1 约定）而非 `note_events`（v2）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct SoundChannel<'a> {
    /// 音频文件名。
    #[serde(deserialize_with = "crate::de_path")]
    pub name: &'a Path,
    /// 引用此音频文件的音符。
    #[serde(rename = "notes")]
    pub notes: Vec<crate::NoteEvent>,
}

use crate::{Bmson as RootBmson, ChartData, ChartInfo, SongInfo};

impl<'a> From<Bmson<'a>> for RootBmson<'a> {
    fn from(v1: Bmson<'a>) -> Self {
        let info = v1.info;

        let song_info = SongInfo {
            title: info.title,
            artist: info.artist,
            genre: info.genre,
        };

        let chart_info = ChartInfo {
            subtitle: info.subtitle,
            subartists: info.subartists,
            chart_name: info.chart_name,
            level: info.level,
            back_image: info.back_image,
            eyecatch_image: info.eyecatch_image,
            banner_image: info.banner_image,
            preview_music: info.preview_music,
            title_image: info.title_image,
            bga: v1.bga,
        };

        let chart_data = ChartData {
            mode_hint: info.mode_hint,
            ln_type_hint: crate::LnType::Ln,
            ln_judge_hint: crate::LnJudge::Normal,
            ln_life_hint: crate::LnLife::Normal,
            init_bpm: info.init_bpm,
            judge_multiplier: info.judge_rank / 100.0,
            life_multiplier: info.total / 100.0,
            resolution: info.resolution,
            lines: v1.lines,
            bpm_events: v1.bpm_events,
            stop_events: v1.stop_events,
            sound_channels: v1
                .sound_channels
                .into_iter()
                .map(|ch| crate::SoundChannel {
                    name: ch.name,
                    note_events: ch.notes,
                })
                .collect(),
            judge_deltas: None,
            life_deltas: None,
        };

        Self {
            version: v1.version,
            song_info,
            chart_info,
            chart_data,
            scroll_events: v1.scroll_events,
            mine_channels: v1.mine_channels,
            key_channels: v1.key_channels,
        }
    }
}

impl<'a> From<RootBmson<'a>> for Bmson<'a> {
    fn from(root: RootBmson<'a>) -> Self {
        let info = BmsonInfo {
            title: root.song_info.title,
            subtitle: root.chart_info.subtitle,
            artist: root.song_info.artist,
            subartists: root.chart_info.subartists,
            genre: root.song_info.genre,
            mode_hint: root.chart_data.mode_hint,
            chart_name: root.chart_info.chart_name,
            level: root.chart_info.level,
            init_bpm: root.chart_data.init_bpm,
            judge_rank: root.chart_data.judge_multiplier * 100.0,
            total: root.chart_data.life_multiplier * 100.0,
            back_image: root.chart_info.back_image,
            eyecatch_image: root.chart_info.eyecatch_image,
            banner_image: root.chart_info.banner_image,
            preview_music: root.chart_info.preview_music,
            title_image: root.chart_info.title_image,
            resolution: root.chart_data.resolution,
        };

        Self {
            version: root.version,
            info,
            lines: root.chart_data.lines,
            bpm_events: root.chart_data.bpm_events,
            stop_events: root.chart_data.stop_events,
            sound_channels: root
                .chart_data
                .sound_channels
                .into_iter()
                .map(|ch| SoundChannel {
                    name: ch.name,
                    notes: ch.note_events,
                })
                .collect(),
            bga: root.chart_info.bga,
            scroll_events: root.scroll_events,
            mine_channels: root.mine_channels,
            key_channels: root.key_channels,
        }
    }
}
