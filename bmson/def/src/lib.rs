//! Types and conversion utilities for the **bmson** chart format.
//!
//! Bmson is a JSON-based serialization format for BMS charts.
//! The format has evolved through three versions:
//!
//! | Version | Status | Distinguishing features |
//! |---|---|---|
//! | v0.2.1 (legacy) | [`v0`] submodule | CamelCase field names; unified `EventNote`; `BarLine.k` present |
//! | v1.0.0 | [`v1`] submodule | `snake_case` field names; flat `Bmson` object |
//! | v2.0.0-rc1 | this module | Split `SongInfo`+`ChartInfo`+`ChartData` |
//!
//! # Version conversions
//!
//! Each version module implements [`From`] / [`TryFrom`] for conversion to
//! the root (v2) `Bmson`:
//!
//! - [`v1::Bmson`] → [`Bmson`] — `From` (infallible, values are mapped with reasonable defaults)
//! - [`v0::Bmson`] → [`Bmson`] — `TryFrom` (some lossy mappings)
//! - [`Bmson`] → [`v1::Bmson`] — `From`
//! - [`Bmson`] → [`v0::Bmson`] — `TryFrom` (round-trip may lose extensions)
//!
//! # Module layout
//!
//! | Module | Contents |
//! |---|---|
//! | [`common`] | Types shared across all versions ([`NoteEvent`], [`BpmEvent`], [`BGA`], …) |
//! | [`v0`] | v0.2.1 specific types (`EventNote`, `BarLine` with `k`, …) |
//! | [`v1`] | v1.0.0 specific types (flat `Bmson`, `BmsonInfo`, …) |
//! | this module | v2.0.0-rc1 types (`Bmson`, `SongInfo`, `ChartInfo`, `ChartData`) |

pub mod v0;
pub mod v1;

mod common;

pub use common::{
    BarLine, BGA, BGAEvent, BGAHeader, BpmEvent, KeyChannel, KeyNote, LnJudge, LnLife, LnType,
    MineChannel, MineNote, ModeHint, NoteEvent, ScrollEvent, SoundChannel, StopEvent,
};

pub(crate) use common::{default_multiplier, default_resolution, null_to_default};

use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};


/// beatoraja long-note type (v0 extension, numeric).
///
/// | Value | Variant | Meaning |
/// |---|---|---|
/// | `1` | `Ln` | LN — press only |
/// | `2` | `Cn` | CN — press + release |
/// | `3` | `Hcn` | HCN — hell charge note |
#[derive(Clone, Debug, PartialEq)]
pub enum V0LnType {
    /// LN (1) — press only.
    Ln,
    /// CN (2) — press + release.
    Cn,
    /// HCN (3) — hell charge note.
    Hcn,
}

impl Serialize for V0LnType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Ln => serializer.serialize_u64(1),
            Self::Cn => serializer.serialize_u64(2),
            Self::Hcn => serializer.serialize_u64(3),
        }
    }
}

impl<'de> Deserialize<'de> for V0LnType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let n = u64::deserialize(deserializer)?;
        match n {
            1 => Ok(Self::Ln),
            2 => Ok(Self::Cn),
            3 => Ok(Self::Hcn),
            other => Err(de::Error::invalid_value(
                Unexpected::Unsigned(other),
                &"1 (LN), 2 (CN), or 3 (HCN)",
            )),
        }
    }
}


/// Custom judgement window offsets introduced by the DJ.NEXT player.
///
/// Each field specifies an **additional** amount (in milliseconds) added
/// to the player's default window for that judgement tier.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JudgementDeltas {
    /// Additional offset for the PERFECT window (ms).
    pub perfect: u64,
    /// Additional offset for the GREAT window (ms).
    pub great: u64,
    /// Additional offset for the GOOD window (ms).
    pub good: u64,
    /// Additional offset for the MISS window (ms).
    pub miss: u64,
}

/// Custom life‑gauge deltas introduced by the DJ.NEXT player.
///
/// Each field specifies the life change (as a percentage of the gauge)
/// for that judgement tier.  Negative values drain life.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifeDeltas {
    /// Life change on PERFECT (percent, may be negative).
    pub perfect: f64,
    /// Life change on GREAT (percent, may be negative).
    pub great: f64,
    /// Life change on GOOD (percent, may be negative).
    pub good: f64,
    /// Life change on MISS (percent, may be negative).
    pub miss: f64,
}


/// The root object of a bmson chart (v2.0.0-rc1 schema).
///
/// In v2 the flat `Bmson` from v1/v0 has been split into three sub‑objects:
///
/// | Object | Role |
/// |---|---|---|
/// | [`SongInfo`] | Musical metadata (title, artist, genre) |
/// | [`ChartInfo`] | Per‑chart metadata (difficulty, images, BGA) |
/// | [`ChartData`] | Actual chart data (notes, timing, sound channels) |
///
///
/// # beatoraja extensions
///
/// The optional fields `scroll_events`, `mine_channels` and `key_channels`
/// are beatoraja-specific extensions and are not part of the core bmson
/// specification.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bmson {
    /// bmson format version string.
    ///
    /// Must be a valid `SemVer` string.  For v2 files the value is `"2.0.0"`.
    /// If `version` is missing (`null`), the player should reject the file
    /// or treat it as an older format.
    ///
    pub version: String,

    /// Song‑level metadata (title, artist, genre).
    #[serde(rename = "song_info")]
    pub song_info: SongInfo,

    /// Per‑chart metadata (difficulty, images, BGA).
    #[serde(rename = "chart_info")]
    pub chart_info: ChartInfo,

    /// Chart data (notes, timing, sound channels).
    #[serde(rename = "chart_data")]
    pub chart_data: ChartData,

    // ---- beatoraja extensions ----
    /// Scroll‑speed change events (beatoraja 0.7.6+).
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,

    /// Mine (landmine) channels (beatoraja extension).
    #[serde(default)]
    pub mine_channels: Vec<MineChannel>,

    /// Invisible ("key") channels (beatoraja extension).
    #[serde(default)]
    pub key_channels: Vec<KeyChannel>,
}

// ---------------------------------------------------------------------------
// SongInfo
// ---------------------------------------------------------------------------

/// Song‑level metadata (v2.0.0-rc1).
///
/// Extracted from the old `BmsonInfo` in v1.  Contains only the fields that
/// describe the **composition itself** (not a specific chart).
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SongInfo {
    /// Song title.
    ///
    /// Players must display this as-is without splitting on delimiters
    /// like `()` or `--`.
    ///
    pub title: String,

    /// Primary artist.
    ///
    /// Usually the music composer.  May contain multiple names separated
    /// by `vs`, `feat.`, etc.
    ///
    pub artist: String,

    /// Song genre.
    ///
    pub genre: String,
}

// ---------------------------------------------------------------------------
// ChartInfo
// ---------------------------------------------------------------------------

/// Per‑chart metadata (v2.0.0-rc1).
///
/// Carries difficulty information, display assets and BGA for one specific
/// chart arrangement of a song.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChartInfo {
    /// Chart subtitle.
    ///
    /// Displayed in a smaller font below [`SongInfo::title`].
    /// May contain `\n` for multi‑line subtitles.
    ///
    #[serde(default)]
    pub subtitle: String,

    /// Contributors other than the primary artist.
    ///
    /// Each entry has the form `"key:value"` where `key` is one of:
    /// `music`, `vocal`, `chart`, `image`, `movie`, `other`.
    /// If `key` is omitted it defaults to `other`.
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub subartists: Vec<String>,

    /// Chart name / difficulty label.
    ///
    /// Examples: `"BEGINNER"`, `"HYPER"`, `"ANOTHER"`.
    ///
    #[serde(default)]
    pub chart_name: String,

    /// Numeric difficulty level.
    ///
    /// Must be ≥ 0.  For `beat-7k` mode the value is typically in the
    /// range 1–12.
    ///
    pub level: u64,

    /// Background image displayed **during gameplay**.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_image: Option<String>,

    /// Eyecatch image displayed **during song load**.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eyecatch_image: Option<String>,

    /// Banner image used in **song‑select and result screens**.
    ///
    /// Recommended aspect ratio: 15 : 4 (e.g. 600×160).
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_image: Option<String>,

    /// Short preview audio file path.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_music: Option<String>,

    /// Background animation data.
    #[serde(rename = "bga")]
    pub bga: BGA,
}

// ---------------------------------------------------------------------------
// ChartData
// ---------------------------------------------------------------------------

/// The actual chart data (v2.0.0-rc1).
///
/// Contains everything needed to play the chart: timing, notes, sound
/// channels, LN hints, and optional DJ.NEXT extensions.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChartData {
    /// Game‑mode hint.
    ///
    /// Players should check this field to verify that the chart is
    /// compatible with their input layout.
    ///
    #[serde(default)]
    pub mode_hint: ModeHint,

    /// Chart‑level LN (long‑note) type hint.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `ln` | Judged on initial press only |
    /// | `cn` | Judged on both press and release |
    ///
    /// This can be overridden per‑note via [`NoteEvent::ln_type_hint`].
    ///
    #[serde(default)]
    pub ln_type_hint: LnType,

    /// Chart‑level LN judgement hint.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `normal` | Only the note itself is judged |
    /// | `ticks` | Extra ticks are judged during the hold |
    ///
    #[serde(default)]
    pub ln_judge_hint: LnJudge,

    /// Chart‑level LN life (gauge) hint.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `normal` | Only the note itself restores life |
    /// | `ticks` | Extra ticks restore life during the hold |
    ///
    #[serde(default)]
    pub ln_life_hint: LnLife,

    /// Initial BPM (beats per minute) at the start of the song.
    ///
    /// If omitted the file is considered malformed.
    ///
    pub init_bpm: f64,

    /// Judgement window multiplier (v2).
    ///
    /// Default: `1.00`.
    ///
    /// | Value | Window width |
    /// |---|---|
    /// | `1.00` | Default (player‑specific) |
    /// | `>1.00` | Wider (easier) |
    /// | `<1.00` | Narrower (harder) |
    ///
    /// In v1 this was `judge_rank` (0–100 scale); the conversion is
    /// `judge_multiplier = judge_rank / 100.0`.
    ///
    #[serde(default = "default_multiplier")]
    pub judge_multiplier: f64,

    /// Life‑gauge multiplier (v2).
    ///
    /// Default: `1.00`.
    ///
    /// Values >1.00 make the gauge fill faster; values <1.00 make it
    /// fill slower.  Must be ≥ 0.  In v1 this was `total` (0–100 scale);
    /// the conversion is `life_multiplier = total / 100.0`.
    ///
    #[serde(default = "default_multiplier")]
    pub life_multiplier: f64,

    /// Ticks per quarter‑note (pulse resolution).
    ///
    /// Default: `240`.  Must be > 0.
    ///
    /// 240 is the LCM of 48 (common BMS resolution) and 5, allowing
    /// quintuplet rhythms.
    ///
    #[serde(default = "default_resolution")]
    pub resolution: u64,

    /// Bar‑line positions (measure boundaries).
    ///
    /// `None` → auto‑generate 4/4 bar lines.
    /// `Some([])` → no bar lines displayed.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<BarLine>>,

    /// BPM change events.
    ///
    /// The spec marks this field as nullable (`BpmEvent[]?`);
    /// `null` is treated as an empty array.
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub bpm_events: Vec<BpmEvent>,

    /// Stop (pause) events.
    ///
    /// The spec marks this field as nullable (`StopEvent[]?`);
    /// `null` is treated as an empty array.
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub stop_events: Vec<StopEvent>,

    /// Sound channels — each bundles an audio file with its notes.
    #[serde(default)]
    pub sound_channels: Vec<SoundChannel>,

    // ---- DJ.NEXT extensions ----
    /// Custom judgement window offsets (DJ.NEXT extension).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_deltas: Option<JudgementDeltas>,

    /// Custom life‑gauge deltas (DJ.NEXT extension).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_deltas: Option<LifeDeltas>,
}
