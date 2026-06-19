//! **bmson v1.0.0** — the flat schema.
//!
//! This module mirrors the v1.0.0 specification where `Bmson` is a single
//! flat object containing both metadata and chart data.
//!
//! # Conversions
//!
//! [`Bmson`] ↔ [`crate::Bmson`] via [`From`].
//!
//! v1 → root (v2) splits [`BmsonInfo`] across [`crate::SongInfo`],
//! [`crate::ChartInfo`] and [`crate::ChartData`].
//! Root → v1 merges them back.
//!
//! # Serde
//!
//! Top-level types use v1-specific field names (`info`, `sound_channels` /
//! `notes` as root key). Leaf types are reused from the root module.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{BGA, BarLine, BpmEvent, KeyChannel, MineChannel, ModeHint, ScrollEvent, StopEvent};

/// Top-level bmson object in the v1.0.0 schema.
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
    /// bmson format version (should be `"1.0.0"`).
    #[serde(borrow)]
    pub version: &'a str,

    /// Metadata object.
    pub info: BmsonInfo<'a>,

    /// Bar-line positions. `None` → 4/4 auto, `Some([])` → no bars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<BarLine>>,

    /// BPM change events (nullable in spec; null treated as empty).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub bpm_events: Vec<BpmEvent>,

    /// Stop (pause) events (nullable in spec; null treated as empty).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub stop_events: Vec<StopEvent>,

    /// Sound channels (v1 uses `notes` inside each channel).
    #[serde(default)]
    pub sound_channels: Vec<SoundChannel<'a>>,

    /// Background animation data.
    pub bga: BGA<'a>,

    /// Scroll-speed events.
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,
    /// Mine channels.
    #[serde(default)]
    pub mine_channels: Vec<MineChannel<'a>>,
    /// Invisible-key channels.
    #[serde(default)]
    pub key_channels: Vec<KeyChannel<'a>>,
}

/// Metadata object in the v1 schema.
///
/// Holds everything about the song and this specific chart.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BmsonInfo<'a> {
    /// Song title.
    #[serde(borrow)]
    pub title: &'a str,

    /// Subtitle (default `""`).
    #[serde(borrow, default)]
    pub subtitle: &'a str,

    /// Primary artist.
    #[serde(borrow)]
    pub artist: &'a str,

    /// Additional contributors (`["key:value", ...]`).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub subartists: Vec<&'a str>,

    /// Genre.
    #[serde(borrow)]
    pub genre: &'a str,

    /// Game-mode hint (default `"beat-7k"`).
    #[serde(default)]
    pub mode_hint: ModeHint,

    /// Chart name / difficulty label (default `""`).
    #[serde(borrow, default)]
    pub chart_name: &'a str,

    /// Numeric difficulty level.
    pub level: u64,

    /// Initial BPM.
    pub init_bpm: f64,

    /// Judgement window (0–100 scale, default 100).
    #[serde(default = "default_100")]
    pub judge_rank: f64,

    /// Life-gauge fill amount (0–100 scale, default 100).
    #[serde(default = "default_100")]
    pub total: f64,

    /// Background image (gameplay).
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_image: Option<&'a Path>,

    /// Eyecatch image (load screen).
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<&'a Path>,

    /// Banner image (select / results).
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<&'a Path>,

    /// Preview music path.
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<&'a Path>,

    /// Title image displayed before gameplay starts.
    /// Equivalent to `#BACKBMP` in the OADX+ skin system.
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub title_image: Option<&'a Path>,

    /// Pulse resolution (default 240).
    #[serde(
        default = "crate::default_resolution",
        deserialize_with = "crate::deserialize_resolution_nonzero"
    )]
    pub resolution: u64,
}

/// Default value 100.0 for `#[serde(default)]` on fields like `resolution`.
const fn default_100() -> f64 {
    100.0
}

/// A sound channel in the v1 schema.
///
/// Identical to [`crate::SoundChannel`] except the notes field is
/// `notes` (v1 convention) instead of `note_events` (v2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct SoundChannel<'a> {
    /// Audio file name.
    #[serde(deserialize_with = "crate::de_path")]
    pub name: &'a Path,
    /// Notes referencing this audio file.
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
