//! **bmson v0.2.1** (legacy) — camelCase field names, `EventNote` format.
//!
//! Prior to v1.0.0 bmson used a mix of `camelCase` and `snake_case` field
//! names, had a unified `EventNote` type that served both BPM and STOP
//! events, and included a `k` field on `BarLine`.
//!
//! # Conversions
//!
//! | Direction | Trait | Notes |
//! |---|---|---|
//! | `v0::Bmson` → [`crate::Bmson`] | [`TryFrom`] | Discards `BarLine.k`; maps `EventNote` by context; maps `t`→`ln_type_hint`; fills defaults for missing v2 fields |
//! | [`crate::Bmson`] → `v0::Bmson` | [`TryFrom`] | Merges `BpmEvent`+`StopEvent` back into `EventNote`; maps `ln_type_hint`→`t`; defaults for missing v0 fields |
//!
//! # Serde
//!
//! | v0 JSON | Rust field |
//! |---|---|
//! | `info` (key in Bmson) | `info` |
//! | `lines` | `lines` (same key as v1) |
//! | `bpmNotes` | `bpm_notes` |
//! | `stopEvents` / `stopNotes` | `stop_events` (both accepted) |
//! | `soundChannel` | `sound_channels` |
//! | `bgaHeader` / `bgaNotes` / `layerNotes` / `poorNotes` | reused [`crate::BGA`] handles these |
//! | `ID` (inside BGAHeader → handled in [`crate::BGAHeader`]) | `id` |
//! | `lnType` (inside BmsonInfo) | `ln_type` |
//! | `titleImage` (inside BmsonInfo) | `title_image` |
//! | `t` (inside Note, beatoraja extension) | `t` |

use serde::{Deserialize, Serialize};

use crate::{NoteEvent, V0LnType};

// ---------------------------------------------------------------------------
// Bmson (v0.2.1)
// ---------------------------------------------------------------------------

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
pub struct Bmson {
    /// Metadata object.
    pub info: BmsonInfo,

    /// Bar lines (v0 includes the `k` field).
    /// The JSON key is `"lines"` (same as v1/v2).
    pub lines: Option<Vec<BarLine>>,

    /// BPM change events (`bpmNotes` in JSON; nullable per spec).
    #[serde(
        rename = "bpmNotes",
        default,
        deserialize_with = "crate::null_to_default"
    )]
    pub bpm_notes: Vec<EventNote>,

    /// Stop events (`stopEvents` or `stopNotes` in JSON; nullable per spec).
    #[serde(
        rename = "stopEvents",
        alias = "stopNotes",
        default,
        deserialize_with = "crate::null_to_default"
    )]
    pub stop_events: Vec<EventNote>,

    /// Sound channels (`soundChannel` in JSON).
    #[serde(rename = "soundChannel", default)]
    pub sound_channels: Vec<SoundChannel>,

    /// Background animation data.
    #[serde(rename = "bga")]
    pub bga: crate::BGA,

    // ---- beatoraja extensions ----
    /// Scroll-speed events.
    #[serde(default)]
    pub scroll_events: Vec<crate::ScrollEvent>,
    /// Mine channels.
    #[serde(default)]
    pub mine_channels: Vec<crate::MineChannel>,
    /// Invisible-key channels.
    #[serde(default)]
    pub key_channels: Vec<crate::KeyChannel>,
}

// ---------------------------------------------------------------------------
// BmsonInfo (v0.2.1)
// ---------------------------------------------------------------------------

/// Metadata object for v0.2.1.
///
/// Many fields are optional here because early versions did not have them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BmsonInfo {
    /// Song title.
    pub title: String,

    /// Subtitle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,

    /// Primary artist.
    pub artist: String,

    /// Additional contributors (`["key:value", ...]`).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub subartists: Vec<String>,

    /// Genre.
    pub genre: String,

    /// Game-mode hint (`modeHint` in JSON). Not present in original v0.
    #[serde(rename = "modeHint", default, skip_serializing_if = "Option::is_none")]
    pub mode_hint: Option<crate::ModeHint>,

    /// Chart name (`chartName` in JSON).
    #[serde(rename = "chartName", default, skip_serializing_if = "Option::is_none")]
    pub chart_name: Option<String>,

    /// Numeric difficulty level.
    ///
    /// Unlike v1+ fields that were added later, `level` existed in v0.
    pub level: u64,

    /// Initial BPM (`initBPM` in JSON).
    #[serde(rename = "initBPM")]
    pub init_bpm: f64,

    /// Judgement rank (`judgeRank` in JSON, default 100).
    #[serde(rename = "judgeRank", default = "default_100")]
    pub judge_rank: f64,

    /// Total (default 100).
    #[serde(default = "default_100")]
    pub total: f64,

    /// Background image (`backImage` in JSON).
    #[serde(rename = "backImage", default, skip_serializing_if = "Option::is_none")]
    pub back_image: Option<String>,

    /// Eyecatch image (`eyecatchImage` in JSON).
    #[serde(
        rename = "eyecatchImage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<String>,

    /// Banner image (`bannerImage` in JSON).
    #[serde(
        rename = "bannerImage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<String>,

    /// Preview music (`previewMusic` in JSON).
    #[serde(
        rename = "previewMusic",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<String>,

    /// Title image (`titleImage` in JSON).
    #[serde(rename = "titleImage", default, skip_serializing_if = "Option::is_none")]
    pub title_image: Option<String>,

    /// Pulse resolution (default 240).
    #[serde(default = "crate::default_resolution", deserialize_with = "crate::deserialize_resolution_nonzero")]
    pub resolution: u64,

    /// Long-note type — beatoraja extension (`lnType` in JSON).
    #[serde(rename = "lnType", default, skip_serializing_if = "Option::is_none")]
    pub ln_type: Option<V0LnType>,
}

fn default_100() -> f64 {
    100.0
}

// ---------------------------------------------------------------------------
// BarLine (v0.2.1)
// ---------------------------------------------------------------------------

/// A bar line in the v0 schema.
///
/// Has an extra field `k` that was removed in v1.0.0.
/// The `k` field is optional — official samples omit it on some entries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BarLine {
    /// Pulse offset.
    pub y: u64,
    /// Legacy field (removed in v1). Semantics are unspecified;
    /// preserved for round-trip compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k: Option<u64>,
}

// ---------------------------------------------------------------------------
// EventNote (v0.2.1)
// ---------------------------------------------------------------------------

/// A unified timing event used in v0.2.1 for both BPM changes and stops.
///
/// In v1.0.0 this was split into [`crate::BpmEvent`] and [`crate::StopEvent`].
///
/// | Array | `v` meaning |
/// |---|---|
/// | `bpmNotes` | New BPM value |
/// | `stopEvents` | Stop duration (in pulses) |
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventNote {
    /// Pulse offset.
    pub y: u64,
    /// Value: BPM (if in `bpmNotes`) or duration in pulses (if in `stopEvents`).
    pub v: f64,
}

// ---------------------------------------------------------------------------
// SoundChannel (v0.2.1) — uses `notes` field name
// ---------------------------------------------------------------------------

/// A sound channel in the v0 schema.
///
/// Uses the field name `"notes"` (not `"note_events"` as in v2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundChannel {
    /// Audio file name.
    pub name: String,
    /// Notes referencing this audio file.
    pub notes: Vec<NoteEvent>,
}


use crate::{BpmEvent, StopEvent};
use crate::{ChartData, ChartInfo, SongInfo};

/// Error type for v0 ↔ root conversion failures.
#[derive(Clone, Debug)]
pub struct TryFromV0Error;

impl core::fmt::Display for TryFromV0Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("v0 ↔ root conversion error")
    }
}

impl std::error::Error for TryFromV0Error {}

// ---------------------------------------------------------------------------
// v0 → root (v2)
// ---------------------------------------------------------------------------

impl TryFrom<Bmson> for crate::Bmson {
    type Error = TryFromV0Error;

    fn try_from(v0: Bmson) -> Result<Self, Self::Error> {
        let info = v0.info;

        let song_info = SongInfo {
            title: info.title,
            artist: info.artist,
            genre: info.genre,
        };

        let chart_info = ChartInfo {
            subtitle: info.subtitle.unwrap_or_default(),
            subartists: info.subartists,
            chart_name: info.chart_name.unwrap_or_default(),
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
            ln_type_hint: crate::LnType::Ln,
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
                            // Map v0 't' field → v2 'ln_type_hint' if not already set.
                            if note.ln_type_hint.is_none() {
                                note.ln_type_hint = note.t.as_ref().and_then(|t| match t {
                                    V0LnType::Hcn => None, // no LnType equivalent for HCN
                                    V0LnType::Ln => Some(crate::LnType::Ln),
                                    V0LnType::Cn => Some(crate::LnType::Cn),
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
            version: "0.2.1".to_owned(),
            song_info,
            chart_info,
            chart_data,
            scroll_events: v0.scroll_events,
            mine_channels: v0.mine_channels,
            key_channels: v0.key_channels,
        })
    }
}

fn convert_bar_lines(lines: Vec<BarLine>) -> Vec<crate::BarLine> {
    lines
        .into_iter()
        .map(|bl| crate::BarLine { y: bl.y })
        .collect()
}

// ---------------------------------------------------------------------------
// root (v2) → v0
// ---------------------------------------------------------------------------

impl TryFrom<crate::Bmson> for Bmson {
    type Error = TryFromV0Error;

    fn try_from(root: crate::Bmson) -> Result<Self, Self::Error> {
        let info = BmsonInfo {
            title: root.song_info.title,
            subtitle: some_if_nonempty(root.chart_info.subtitle),
            artist: root.song_info.artist,
            subartists: root.chart_info.subartists,
            genre: root.song_info.genre,
            mode_hint: Some(root.chart_data.mode_hint),
            chart_name: some_if_nonempty(root.chart_info.chart_name),
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
                            // Map v2 'ln_type_hint' → v0 't' if not already set.
                            if note.t.is_none() {
                                note.t = note.ln_type_hint.as_ref().map(|h| match h {
                                    crate::LnType::Ln => V0LnType::Ln,
                                    crate::LnType::Cn => V0LnType::Cn,
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

fn some_if_nonempty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}
