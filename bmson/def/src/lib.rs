//! Type definitions and conversion utilities for the **bmson** chart format.
//!
//! Bmson is a JSON-based serialization format for BMS charts. The format has
//! evolved through three versions:
//!
//! | Version | Status | Distinguishing features |
//! |---|---|---|
//! | v0.2.1 (legacy) | [`v0`] submodule | CamelCase field names; unified `EventNote`; `BarLine` has a `k` field |
//! | v1.0.0 | [`v1`] submodule | `snake_case` field names; flat `Bmson` object |
//! | v2.0.0-rc1 | this module | split into `SongInfo` + `ChartInfo` + `ChartData` |
//!
//! # Version conversion
//!
//! Each version module implements [`From`] / [`TryFrom`] for conversion to the
//! root (v2) `Bmson`:
//!
//! - [`v1::Bmson`] to [`Bmson`]: `From` (infallible; maps with sensible
//!   defaults)
//! - [`v0::Bmson`] to [`Bmson`]: `TryFrom` (some mappings are lossy)
//! - [`Bmson`] to [`v1::Bmson`]: `From`
//! - [`Bmson`] to [`v0::Bmson`]: `TryFrom` (roundtrips may lose extension
//!   fields)
//!
//! # Module layout
//!
//! | Module | Content |
//! |---|---|
//! | [`v0`] | v0.2.1-specific types (`EventNote`, `BarLine` with `k`, and so on) |
//! | [`v1`] | v1.0.0-specific types (flat `Bmson`, `BmsonInfo`, and so on) |
//! | this module | v2.0.0-rc1 types (`Bmson`, `SongInfo`, `ChartInfo`, `ChartData`) |
//!
//! Types shared across versions ([`NoteEvent`], [`BpmEvent`], [`BGA`], and so
//! on) are re-exported from the crate root and defined in the internal `common`
//! module.

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

/// Custom judgment-window offsets introduced by the DJ.NEXT player.
///
/// Each field specifies an **additional** amount (in milliseconds) added to the
/// player's default window for that judgment level.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgementDeltas {
    /// Additional offset for the PERFECT window (milliseconds).
    pub perfect: u64,
    /// Additional offset for the GREAT window (milliseconds).
    pub great: u64,
    /// Additional offset for the GOOD window (milliseconds).
    pub good: u64,
    /// Additional offset for the MISS window (milliseconds).
    pub miss: u64,
}

/// Custom life gauge deltas introduced by the DJ.NEXT player.
///
/// Each field specifies the life change for that judgment level as a percentage
/// of the life gauge. Negative values drain life.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LifeDeltas {
    /// Life change on PERFECT (percentage, may be negative).
    pub perfect: f64,
    /// Life change on GREAT (percentage, may be negative).
    pub great: f64,
    /// Life change on GOOD (percentage, may be negative).
    pub good: f64,
    /// Life change on MISS (percentage, may be negative).
    pub miss: f64,
}

/// Root object of a bmson chart (v2.0.0-rc1 schema).
///
/// In v2, the flat `Bmson` of v1/v0 is split into three sub-objects:
///
/// | Object | Responsibility |
/// |---|---|---|
/// | [`SongInfo`] | song metadata (title, artist, genre) |
/// | [`ChartInfo`] | chart-level metadata (difficulty, images, BGA) |
/// | [`ChartData`] | actual chart data (notes, timing, sound channels) |
///
///
/// # beatoraja extensions
///
/// The optional fields `scroll_events`, `mine_channels`, and `key_channels` are
/// beatoraja-specific extensions and are not part of the core bmson
/// specification.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bmson<'a> {
    /// bmson format version string.
    ///
    /// Must be a valid `SemVer` string. v2 files use `"2.0.0"`. If `version` is
    /// missing (`null`), the player should reject the file or treat it as a
    /// legacy format.
    ///
    #[serde(borrow)]
    pub version: &'a str,

    /// Song-level metadata (title, artist, genre).
    #[serde(rename = "song_info")]
    pub song_info: SongInfo<'a>,

    /// Chart-level metadata (difficulty, images, BGA).
    #[serde(rename = "chart_info")]
    pub chart_info: ChartInfo<'a>,

    /// Chart data (notes, timing, sound channels).
    #[serde(rename = "chart_data")]
    pub chart_data: ChartData<'a>,

    /// Scroll speed change events (beatoraja 0.7.6+).
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,

    /// Mine (landmine) channels (beatoraja extension).
    #[serde(default)]
    pub mine_channels: Vec<MineChannel<'a>>,

    /// Invisible ("key") channels (beatoraja extension).
    #[serde(default)]
    pub key_channels: Vec<KeyChannel<'a>>,
}

/// Song-level metadata (v2.0.0-rc1).
///
/// Extracted from v1's old `BmsonInfo`. Contains only fields that describe the
/// **song itself**, not a specific chart.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongInfo<'a> {
    /// Song title.
    ///
    /// Players must display it as-is and must not split it on separators such as
    /// `()` or `--`.
    ///
    #[serde(borrow)]
    pub title: &'a str,

    /// Main artist.
    ///
    /// Usually the music composer. May contain multiple names separated by
    /// `vs`, `feat.`, and so on.
    ///
    #[serde(borrow)]
    pub artist: &'a str,

    /// Song genre.
    ///
    #[serde(borrow)]
    pub genre: &'a str,
}

/// Chart-level metadata (v2.0.0-rc1).
///
/// Carries difficulty information, display assets, and BGA for one specific
/// chart arrangement of a song.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartInfo<'a> {
    /// Chart subtitle.
    ///
    /// Displayed in a smaller font below [`SongInfo::title`]. May contain `\n`
    /// for a multi-line subtitle.
    ///
    #[serde(borrow, default)]
    pub subtitle: &'a str,

    /// Contributors other than the main artist.
    ///
    /// Each entry has the form `"key:value"`, where `key` is one of `music`,
    /// `vocal`, `chart`, `image`, `movie`, or `other`. If `key` is omitted, it
    /// defaults to `other`.
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub subartists: Vec<&'a str>,

    /// Chart name / difficulty label.
    ///
    /// Examples: `"BEGINNER"`, `"HYPER"`, `"ANOTHER"`.
    ///
    #[serde(borrow, default)]
    pub chart_name: &'a str,

    /// Numeric difficulty level.
    ///
    /// Must be at least 0. In `beat-7k` mode the usual range is 1-12.
    ///
    pub level: u64,

    /// Background image shown **during gameplay**.
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_image: Option<&'a Path>,

    /// Eyecatch image shown **while the song loads**.
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<&'a Path>,

    /// Banner image used on the **song selection and result screens**.
    ///
    /// Recommended aspect ratio: 15:4 (for example, 600x160).
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<&'a Path>,

    /// Path to a short preview audio file.
    ///
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<&'a Path>,

    /// Title image shown **before gameplay starts**.
    ///
    /// Equivalent to `#BACKBMP` in the OADX+ skin system. When missing, the
    /// player displays the title in its default font.
    #[serde(
        default,
        deserialize_with = "de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub title_image: Option<&'a Path>,

    /// Background animation data.
    #[serde(rename = "bga")]
    pub bga: BGA<'a>,
}

/// Actual chart data (v2.0.0-rc1).
///
/// Contains everything needed to play the chart: timing, notes, sound channels,
/// long-note hints, and optional DJ.NEXT extensions.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct ChartData<'a> {
    /// Game mode hint.
    ///
    /// Players should check this field to confirm the chart is compatible with
    /// their input layout.
    ///
    #[serde(default)]
    pub mode_hint: ModeHint,

    /// Chart-level long-note type hint.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `ln` | judged only on press |
    /// | `cn` | judged on both press and release |
    ///
    /// May be overridden per note through [`NoteEvent::ln_type_hint`].
    ///
    #[serde(default)]
    pub ln_type_hint: LnType,

    /// Chart-level long-note judgment hint.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `normal` | judge only the note itself |
    /// | `ticks` | judge additional ticks while held |
    ///
    #[serde(default)]
    pub ln_judge_hint: LnJudge,

    /// Chart-level long-note life hint.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `normal` | only the note itself restores life |
    /// | `ticks` | additional ticks restore life while held |
    ///
    #[serde(default)]
    pub ln_life_hint: LnLife,

    /// Initial BPM at the start of the song (beats per minute).
    ///
    /// Omitting it is treated as a file format error.
    ///
    pub init_bpm: f64,

    /// Judgment window multiplier (v2).
    ///
    /// Default: `1.00`.
    ///
    /// | Value | Window width |
    /// |---|---|
    /// | `1.00` | default (varies by player) |
    /// | `>1.00` | wider (easier) |
    /// | `<1.00` | narrower (harder) |
    ///
    /// In v1 this field is `judge_rank` (0-100 scale); the conversion is
    /// `judge_multiplier = judge_rank / 100.0`.
    ///
    #[serde(default = "default_multiplier")]
    pub judge_multiplier: f64,

    /// Life gauge multiplier (v2).
    ///
    /// Default: `1.00`.
    ///
    /// Values above 1.00 charge the gauge faster; values below 1.00 charge it
    /// more slowly. Must be at least 0. In v1 this field is `total` (0-100
    /// scale); the conversion is `life_multiplier = total / 100.0`.
    ///
    #[serde(default = "default_multiplier")]
    pub life_multiplier: f64,

    /// Ticks per quarter note (beat resolution).
    ///
    /// Default: `240`. Must be greater than 0.
    ///
    /// 240 is the least common multiple of 48 (a common BMS resolution) and 5,
    /// which supports quintuplet rhythms.
    ///
    #[serde(
        default = "default_resolution",
        deserialize_with = "deserialize_resolution_nonzero"
    )]
    pub resolution: u64,

    /// Barline positions (measure boundaries).
    ///
    /// `None` means auto-generate 4/4 barlines. `Some([])` means show no
    /// barlines.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<BarLine>>,

    /// BPM change events.
    ///
    /// The specification marks this field nullable (`BpmEvent[]?`); `null` is
    /// treated as an empty array.
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub bpm_events: Vec<BpmEvent>,

    /// Stop (pause) events.
    ///
    /// The specification marks this field nullable (`StopEvent[]?`); `null` is
    /// treated as an empty array.
    ///
    #[serde(default, deserialize_with = "null_to_default")]
    pub stop_events: Vec<StopEvent>,

    /// Sound channels; each channel binds one audio file to its notes.
    #[serde(default)]
    pub sound_channels: Vec<SoundChannel<'a>>,

    /// Custom judgment-window offsets (DJ.NEXT extension).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_deltas: Option<JudgementDeltas>,

    /// Custom life gauge deltas (DJ.NEXT extension).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_deltas: Option<LifeDeltas>,
}

/// Errors that can occur during bmson version detection or format conversion.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BmsonError {
    /// The version string exists but is not recognized.
    ///
    /// Only `SemVer` versions starting with `"0."` (v0.2.1 legacy), `"1."`
    /// (v1.0.0), or `"2."` (v2.0.0-rc1) are supported.
    #[error("unknown bmson version: {0}")]
    UnknownVersion(String),

    /// Conversion from the legacy v0.2.1 format failed.
    #[error("v0 conversion error: {0}")]
    V0Conversion(String),
}

/// Detected bmson format version, determined by inspecting the `"version"` field
/// of the JSON chart file.
///
/// | Variant | Detection |
/// |---|---|
/// | [`V0`](DetectedVersion::V0) | no `"version"` field (legacy) |
/// | [`V1`](DetectedVersion::V1) | `"version"` starts with `"1."` (SemVer prefix) |
/// | [`V2`](DetectedVersion::V2) | `"version"` starts with `"2."` (SemVer prefix) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedVersion {
    /// v0.2.1 (legacy, no `version` field).
    V0,
    /// v1.0.0 (flat schema).
    V1,
    /// v2.0.0-rc1 (split schema).
    V2,
}

/// Detect the bmson format version by scanning the `"version"` field of the JSON
/// chart file.
///
/// This performs a **lightweight string scan** rather than full JSON parsing, so
/// it is suitable as the first step before dispatching to a version-specific
/// deserializer.
///
/// # Detection logic
///
/// 1. Search the JSON text for the literal `"version"` key.
/// 2. If found, extract the string value after the colon.
/// 3. If the value starts with `"2."`, return [`V2`](DetectedVersion::V2).
/// 4. If the value starts with `"1."`, return [`V1`](DetectedVersion::V1).
/// 5. If the value starts with `"0."`, return [`V0`](DetectedVersion::V0).
/// 6. Otherwise return [`BmsonError::UnknownVersion`].
/// 7. If `"version"` is absent, return [`V0`](DetectedVersion::V0) (legacy).
///
/// # Example
///
/// ```rust
/// let json = r#"{"version":"2.0.0","song_info":{}}"#;
/// assert_eq!(bmsrs_bmson_def::DetectedVersion::detect(json).unwrap(), bmsrs_bmson_def::DetectedVersion::V2);
/// ```
///
/// # Errors
///
/// Returns [`BmsonError::UnknownVersion`] when the `"version"` field is present
/// but its value is not a string, or does not start with `"0."`, `"1."`, or
/// `"2."`.
impl DetectedVersion {
    /// Scan a JSON string and detect the bmson version.
    ///
    /// # Errors
    ///
    /// Returns [`BmsonError::UnknownVersion`] when the `"version"` field exists
    /// but cannot be recognized.
    #[expect(
        clippy::string_slice,
        reason = "JSON bytes for \"version\" key and ASCII version strings; byte indexing is safe"
    )]
    pub fn detect(json: &str) -> Result<Self, BmsonError> {
        // Scan the JSON byte by byte, tracking brace depth, and match only the
        // top-level `"version"` key.
        let mut depth: u32 = 0;
        let mut in_string = false;
        let mut string_start: usize = 0;
        let mut key_pos: Option<usize> = None;

        let mut chars = json.char_indices();
        while let Some((i, ch)) = chars.next() {
            if in_string {
                if ch == '\\' {
                    // Skip the escaped character.
                    chars.next();
                } else if ch == '"' {
                    in_string = false;
                    if depth == 1 && &json[string_start..i] == "version" {
                        key_pos = Some(string_start - 1);
                        break;
                    }
                }
            } else {
                match ch {
                    '{' => depth += 1,
                    '}' => depth = depth.saturating_sub(1),
                    '"' => {
                        in_string = true;
                        string_start = i + 1;
                    }
                    _ => {}
                }
            }
        }

        let Some(pos) = key_pos else {
            return Ok(Self::V0);
        };

        let mut rest = &json[pos + 9..];
        // Skip whitespace; expect `:`.
        rest = rest.trim_start();
        rest = rest.strip_prefix(':').ok_or_else(|| {
            BmsonError::UnknownVersion("malformed version field: expected ':'".into())
        })?;
        // Skip whitespace; expect the opening `"`.
        rest = rest.trim_start();
        rest = rest.strip_prefix('"').ok_or_else(|| {
            BmsonError::UnknownVersion("malformed version field: expected string".into())
        })?;
        // Find the closing `"`.
        let end = rest
            .find('"')
            .ok_or_else(|| BmsonError::UnknownVersion("unterminated version string".into()))?;
        let version = &rest[..end];

        if version.starts_with("2.") {
            Ok(Self::V2)
        } else if version.starts_with("1.") {
            Ok(Self::V1)
        } else if version.starts_with("0.") {
            Ok(Self::V0)
        } else {
            Err(BmsonError::UnknownVersion(version.to_owned()))
        }
    }
}

/// Map [`LnMode`] (numeric 1/2/3) to [`LnType`] (ln/cn/hcn).
#[must_use]
pub(crate) const fn ln_mode_to_type_hint(lt: LnMode) -> LnType {
    match lt {
        LnMode::Ln => LnType::Ln,
        LnMode::Cn => LnType::Cn,
        LnMode::Hcn => LnType::Hcn,
    }
}
