//! **bmson v0.2.1** (legacy): camelCase field names and the `EventNote` format.
//!
//! Before v1.0.0, bmson mixed `camelCase` and `snake_case` field names, used a
//! unified `EventNote` type for both BPM and STOP events, and included a `k`
//! field on `BarLine`.
//!
//! # Conversion
//!
//! | Direction | Trait | Notes |
//! |---|---|---|
//! | `v0::Bmson` to [`crate::Bmson`] | [`TryFrom`] | drops `BarLine.k`; maps `EventNote` by context; maps `t` to `ln_type_hint`; fills missing v2 fields with defaults |
//! | [`crate::Bmson`] to `v0::Bmson` | [`TryFrom`] | merges `BpmEvent` and `StopEvent` back into `EventNote`; maps `ln_type_hint` to `t`; fills missing v0 fields with defaults |
//!
//! # Serde
//!
//! | v0 JSON | Rust field |
//! |---|---|
//! | `info` (key inside Bmson) | `info` |
//! | `lines` | `lines` (same key as v1) |
//! | `bpmNotes` (alias `bpm_events`) | `bpm_notes` |
//! | `stopEvents` / `stopNotes` | `stop_events` (both accepted) |
//! | `soundChannel` (alias `sound_channels`) | `sound_channels` |
//! | `bgaHeader` / `bgaNotes` / `layerNotes` / `poorNotes` | handled by the shared [`crate::BGA`] |
//! | `ID` (inside BGAHeader, handled by [`crate::BGAHeader`]) | `id` |
//! | `lnType` (inside BmsonInfo) | `ln_type` |
//! | `titleImage` (inside BmsonInfo) | `title_image` |
//! | `t` (inside Note, beatoraja extension) | `t` |

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{LnMode, ModeHint};

/// Top-level bmson object in the legacy v0.2.1 schema.
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
    /// Metadata object.
    pub info: BmsonInfo<'a>,

    /// Barlines (v0 includes the `k` field). The JSON key is `"lines"` (same as
    /// v1/v2).
    pub lines: Option<Vec<BarLine>>,

    /// BPM change events (JSON key `bpmNotes`, alias `bpm_events`; editors
    /// mix `camelCase` and `snake_case`, and the specification allows null).
    #[serde(
        rename = "bpmNotes",
        alias = "bpm_events",
        default,
        deserialize_with = "crate::null_to_default"
    )]
    pub bpm_notes: Vec<EventNote>,

    /// Stop events (JSON keys `stopEvents` or `stopNotes`; the specification
    /// allows null).
    #[serde(
        rename = "stopEvents",
        alias = "stopNotes",
        default,
        deserialize_with = "crate::null_to_default"
    )]
    pub stop_events: Vec<EventNote>,

    /// Sound channels (JSON key `soundChannel`, alias `sound_channels`).
    #[serde(rename = "soundChannel", alias = "sound_channels", default)]
    pub sound_channels: Vec<SoundChannel<'a>>,

    /// Background animation data.
    #[serde(rename = "bga")]
    pub bga: crate::BGA<'a>,

    /// Scroll speed events.
    #[serde(default)]
    pub scroll_events: Vec<crate::ScrollEvent>,
    /// Mine channels.
    #[serde(default)]
    pub mine_channels: Vec<crate::MineChannel<'a>>,
    /// Invisible key channels.
    #[serde(default)]
    pub key_channels: Vec<crate::KeyChannel<'a>>,
}

/// Metadata object for v0.2.1.
///
/// Many fields are optional here because earlier versions did not have them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BmsonInfo<'a> {
    /// Song title.
    #[serde(borrow)]
    pub title: &'a str,

    /// Subtitle.
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<&'a str>,

    /// Main artist.
    #[serde(borrow)]
    pub artist: &'a str,

    /// Other contributors (`["key:value", ...]`).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub subartists: Vec<&'a str>,

    /// Genre.
    #[serde(borrow)]
    pub genre: &'a str,

    /// Game mode hint (JSON key `modeHint`). Not present in original v0.
    #[serde(rename = "modeHint", default, skip_serializing_if = "Option::is_none")]
    pub mode_hint: Option<ModeHint>,

    /// Chart name (JSON key `chartName`).
    #[serde(
        rename = "chartName",
        borrow,
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub chart_name: Option<&'a str>,

    /// Numeric difficulty level.
    ///
    /// Unlike fields added later in v1+, `level` already exists in v0.
    pub level: u64,

    /// Initial BPM (JSON key `initBPM`).
    #[serde(rename = "initBPM")]
    pub init_bpm: f64,

    /// Judgment rank (JSON key `judgeRank`, default 100).
    #[serde(rename = "judgeRank", default = "default_100")]
    pub judge_rank: f64,

    /// Total life gauge (default 100).
    #[serde(default = "default_100")]
    pub total: f64,

    /// Background image (JSON key `backImage`).
    #[serde(
        rename = "backImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_image: Option<&'a Path>,

    /// Eyecatch image (JSON key `eyecatchImage`).
    #[serde(
        rename = "eyecatchImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<&'a Path>,

    /// Banner image (JSON key `bannerImage`).
    #[serde(
        rename = "bannerImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<&'a Path>,

    /// Preview audio (JSON key `previewMusic`).
    #[serde(
        rename = "previewMusic",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<&'a Path>,

    /// Title image (JSON key `titleImage`).
    #[serde(
        rename = "titleImage",
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub title_image: Option<&'a Path>,

    /// Beat resolution (default 240).
    #[serde(
        default = "crate::default_resolution",
        deserialize_with = "crate::deserialize_resolution_nonzero"
    )]
    pub resolution: u64,

    /// Long-note type, a beatoraja extension (JSON key `lnType`).
    #[serde(rename = "lnType", default, skip_serializing_if = "Option::is_none")]
    pub ln_type: Option<LnMode>,
}

/// Default value 100.0 used by `#[serde(default)]` on fields such as `judge_rank`.
const fn default_100() -> f64 {
    100.0
}

/// Barline in the v0 schema.
///
/// Contains an extra `k` field that was removed in v1.0.0. The `k` field is
/// optional; official examples omit it in some entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarLine {
    /// Tick offset.
    pub y: u64,
    /// Legacy field (removed in v1). Its semantics are unspecified; kept for
    /// roundtrip compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k: Option<u64>,
}

/// Unified timing event used for BPM changes and stops in v0.2.1.
///
/// Split into [`crate::BpmEvent`] and [`crate::StopEvent`] in v1.0.0.
///
/// | Containing array | Meaning of `v` |
/// |---|---|
/// | `bpmNotes` | new BPM value |
/// | `stopEvents` | stop duration in ticks |
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventNote {
    /// Tick offset.
    pub y: u64,
    /// Value: BPM (in `bpmNotes`) or tick duration (in `stopEvents`).
    pub v: f64,
}

/// Sound channel in the v0 schema.
///
/// Uses the field name `"notes"` (rather than v2's `"note_events"`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct SoundChannel<'a> {
    /// Audio file name.
    #[serde(deserialize_with = "crate::de_path")]
    pub name: &'a Path,
    /// Notes referencing this audio file.
    pub notes: Vec<crate::NoteEvent>,
}

use crate::{BpmEvent, StopEvent};
use crate::{ChartData, ChartInfo, SongInfo};

/// Error type for failed v0 to root conversion.
#[derive(Clone, Debug, derive_more::Display)]
#[display("v0 conversion error: {message}")]
pub struct TryFromV0Error {
    /// Human-readable description of the failure.
    pub message: String,
}

impl TryFromV0Error {
    /// Create a new conversion error with a descriptive message.
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
                    // Negative values saturate to 0 through `round() as u64` in
                    // Rust 1.45+ (invalid input is clamped).
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
                            crate::common::backfill_ln_type_hint(&mut note);
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

/// Convert v0 `BarLine` (with the extra `k` field) into the common `BarLine`.
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
                            // Map v2 `ln_type_hint` to v0 `t` when not set.
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

/// Return `None` for an empty string, otherwise `Some(s)`.
///
/// Used when converting the root [`Bmson`] to [`v0::Bmson`](Bmson), mapping
/// empty `&str` fields to `None` so they serialize as absent.
#[must_use]
pub(crate) const fn some_if_nonempty(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}
