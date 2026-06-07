//! Types and conversion utilities for the **bmson** chart format.
//!
//! Bmson is a JSON-based serialization format for BMS charts.
//! The format has evolved through three versions:
//!
//! | Version | Status | Distinguishing features |
//! |---|---|---|
//! | v0.2.1 (legacy) | [`v0`] submodule | CamelCase field names; unified `EventNote`; `BarLine.k` present |
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
//! # Serde strategy
//!
//! All root-level types in this file follow the **v2 schema**.
//! The `v1` and `v0` modules provide their own top-level `Bmson`/`BmsonInfo`
//! structs with appropriate `#[serde(rename = "...")]` attributes, but reuse
//! leaf types from this root module wherever the field layout is compatible.
//!
//! Types that genuinely differ across versions (e.g. `EventNote` in v0,
//! `BarLine.k` in v0) are defined independently in their version module.

pub mod v0;
pub mod v1;

use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// ===========================================================================
// ModeHint
// ===========================================================================

/// Game mode hint specifying the input layout.
///
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ModeHint {
    /// beat-5k (5 keys, 1 scratch).
    Beat5k,
    /// beat-7k (7 keys, 1 scratch).
    #[default]
    Beat7k,
    /// beat-10k (5 keys + scratch per player, 2 players).
    Beat10k,
    /// beat-14k (7 keys + scratch per player, 2 players).
    Beat14k,
    /// popn-5k.
    Popn5k,
    /// popn-9k.
    Popn9k,
    /// dj-5k-only.
    Dj5kOnly,
    /// dj-ruby.
    DjRuby,
    /// dj-5k.
    Dj5k,
    /// dj-7k.
    Dj7k,
    /// dj-10k.
    Dj10k,
    /// dj-14k.
    Dj14k,
    /// dj-andromeda.
    DjAndromeda,
    /// Generic n-keys layout, keys numbered 1..n left-to-right.
    /// Serialized as `"generic-{n}k"`.
    Generic(u64),
    /// Any other (extension) mode hint not in the canonical list.
    Other(String),
}

impl fmt::Display for ModeHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Beat5k => f.write_str("beat-5k"),
            Self::Beat7k => f.write_str("beat-7k"),
            Self::Beat10k => f.write_str("beat-10k"),
            Self::Beat14k => f.write_str("beat-14k"),
            Self::Popn5k => f.write_str("popn-5k"),
            Self::Popn9k => f.write_str("popn-9k"),
            Self::Dj5kOnly => f.write_str("dj-5k-only"),
            Self::DjRuby => f.write_str("dj-ruby"),
            Self::Dj5k => f.write_str("dj-5k"),
            Self::Dj7k => f.write_str("dj-7k"),
            Self::Dj10k => f.write_str("dj-10k"),
            Self::Dj14k => f.write_str("dj-14k"),
            Self::DjAndromeda => f.write_str("dj-andromeda"),
            Self::Generic(n) => write!(f, "generic-{n}k"),
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl core::str::FromStr for ModeHint {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "beat-5k" => Self::Beat5k,
            "beat-7k" => Self::Beat7k,
            "beat-10k" => Self::Beat10k,
            "beat-14k" => Self::Beat14k,
            "popn-5k" => Self::Popn5k,
            "popn-9k" => Self::Popn9k,
            "dj-5k-only" => Self::Dj5kOnly,
            "dj-ruby" => Self::DjRuby,
            "dj-5k" => Self::Dj5k,
            "dj-7k" => Self::Dj7k,
            "dj-10k" => Self::Dj10k,
            "dj-14k" => Self::Dj14k,
            "dj-andromeda" => Self::DjAndromeda,
            _ if s.starts_with("generic-") && s.ends_with('k') => {
                let inner = &s[8..s.len() - 1]; // strip "generic-" prefix and "k" suffix
                inner
                    .parse()
                    .map_or_else(|_| Self::Other(s.to_owned()), Self::Generic)
            }
            _ => Self::Other(s.to_owned()),
        })
    }
}

impl Serialize for ModeHint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ModeHint {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

// ===========================================================================
// LnType / LnJudge / LnLife
// ===========================================================================

/// Long-note type hint (`"ln"` or `"cn"`).
///
/// Can be set at the chart level ([`ChartData::ln_type_hint`]) and overridden
/// per note ([`NoteEvent::ln_type_hint`]).
///
/// | Value | Meaning |
/// |---|---|
/// | `ln` | Judged on initial press only |
/// | `cn` | Judged on both press and release |
///
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LnType {
    /// Long note — judged on initial press only.
    #[default]
    Ln,
    /// Charge note — judged on both press and release.
    Cn,
}

/// Long-note judgement hint (`"normal"` or `"ticks"`).
///
/// | Value | Meaning |
/// |---|---|
/// | `normal` | Only the note itself is judged |
/// | `ticks` | Extra ticks are judged during the hold |
///
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LnJudge {
    /// Only the note itself is judged.
    #[default]
    Normal,
    /// Extra ticks are judged during the hold.
    Ticks,
}

/// Long-note life (gauge) hint (`"normal"` or `"ticks"`).
///
/// | Value | Meaning |
/// |---|---|
/// | `normal` | Only the note itself restores life |
/// | `ticks` | Extra ticks restore life during the hold |
///
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LnLife {
    /// Only the note itself restores life.
    #[default]
    Normal,
    /// Extra ticks restore life during the hold.
    Ticks,
}

// ===========================================================================
// V0LnType (beatoraja extension, numeric 1/2/3)
// ===========================================================================

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

// ===========================================================================
// X – player channel
// ===========================================================================

/// Player channel identifier for a [`NoteEvent`].
///
/// In bmson the `x` field of a [`NoteEvent`] determines which column / key
/// the note belongs to.  The spec declares `x` as `any` (Web IDL) because
/// it can be a positive integer (playable channel), zero (BGM), or `null`
/// (also BGM).
///
/// | `X` variant | JSON value | Meaning |
/// |---|---|---|
/// | `Bgm` | `0` or `null` | Background‑music note, not playable |
/// | `Channel(n)` | positive integer | Playable key / column `n` |
///
/// The exact mapping from channel number to on‑screen column depends on
/// [`ChartData::mode_hint`]:
///
/// | Mode | Channels |
/// |---|---|
/// | `beat-7k` | 1–7 = keys, 8 = scratch |
/// | `beat-5k` | 1–5 = keys, 8 = scratch |
/// | `popn-9k` | 1–9 = keys |
/// | `generic-nkeys` | 1…n left‑to‑right |
///
#[derive(Clone, Debug, PartialEq)]
pub enum X {
    /// BGM note — not playable.
    ///
    /// Serialised as `0` in the JSON output (matching the behaviour of most
    /// existing players).  Deserialises from both `0` and `null`.
    Bgm,
    /// Playable channel number (1‑indexed).
    Channel(u64),
}

impl<'de> Deserialize<'de> for X {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct XVisitor;

        impl Visitor<'_> for XVisitor {
            type Value = X;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("null or non‑negative integer for x")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<X, E> {
                if v == 0 {
                    Ok(X::Bgm)
                } else {
                    Ok(X::Channel(v))
                }
            }

            fn visit_none<E: de::Error>(self) -> Result<X, E> {
                Ok(X::Bgm)
            }

            fn visit_unit<E: de::Error>(self) -> Result<X, E> {
                Ok(X::Bgm)
            }
        }

        deserializer.deserialize_any(XVisitor)
    }
}

impl Serialize for X {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Bgm => serializer.serialize_u64(0),
            Self::Channel(n) => serializer.serialize_u64(*n),
        }
    }
}

// ===========================================================================
// NoteEvent
// ===========================================================================

/// A single note (playable or BGM) in a [`SoundChannel`].
///
/// In v2.0.0-rc1 this was renamed from `Note` to `NoteEvent`.
///
///
/// # Fields shared by all versions
///
/// | Field | Type | Description |
/// |---|---|---|
/// | `x` | [`X`] | Player channel (or BGM) |
/// | `y` | `u64` | Pulse offset |
/// | `l` | `u64` | Length in pulses (`0` = short note, `>0` = long note) |
/// | `c` | `bool` | Continuation flag (audio restart behaviour) |
///
/// # V2‑only optional fields
///
/// The following fields are only present in v2.0.0-rc1+ and will be `None`
/// when the note is deserialised from a v0/v1 file:
///
/// | Field | Type | Description |
/// |---|---|---|
/// | `up` | `bool` | Release‑sound / BSS flag |
/// | `ln_type_hint` | [`LnType`] | Per‑note LN type override |
/// | `ln_judge_hint` | [`LnJudge`] | Per‑note LN judgement override |
/// | `ln_life_hint` | [`LnLife`] | Per‑note LN life override |
/// | `vol` | `i8` | Volume (percent, DJ.NEXT) |
/// | `pan` | `i8` | Pan (DJ.NEXT) |
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoteEvent {
    /// Player channel (or BGM).
    pub x: X,

    /// Pulse offset of this note.
    ///
    /// All timing in bmson is relative to pulses (ticks), not seconds.
    /// The number of pulses per quarter‑note is given by [`ChartData::resolution`] (default 240).
    pub y: u64,

    /// Note length in pulses.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `0` | Short (regular) note — no hold |
    /// | `>0` | Long note lasting from pulse `y` to `y + l` |
    pub l: u64,

    /// Continuation flag — whether to restart the audio slice.
    ///
    /// - `true` — **continue** (do not restart): the audio continues playing
    ///   from the previous slice.
    /// - `false` — **do not continue**: restart the audio at this note's
    ///   slice point.
    pub c: bool,

    // ---- v2.0.0-rc1 optional fields ----
    /// Release‑sound / BSS (Back‑Spin‑Scratch) flag.
    ///
    /// For CN (Charge Note) or BSS, place a `NoteEvent` with `up: true` at
    /// the release position and zero length.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<bool>,

    /// Per‑note override for the chart‑level LN type hint.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_type_hint: Option<LnType>,

    /// Per‑note override for the chart‑level LN judgement hint.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_judge_hint: Option<LnJudge>,

    /// Per‑note override for the chart‑level LN life hint.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_life_hint: Option<LnLife>,

    /// Note volume as a signed percentage (DJ.NEXT extension).
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vol: Option<i8>,

    /// Note pan as a signed percentage (DJ.NEXT extension).
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pan: Option<i8>,
}

impl NoteEvent {
    /// Returns `true` if this note is a BGM note (not playable).
    #[must_use]
    pub fn is_bgm(&self) -> bool {
        self.x == X::Bgm
    }
}

// ===========================================================================
// SoundChannel
// ===========================================================================

/// An **audio channel** — a single audio file with its associated notes.
///
/// Bmson is channel‑based: each [`SoundChannel`] bundles one audio file
/// (`name`) together with all the [`NoteEvent`]s that reference it.
/// The player slices the audio file at the note positions and plays
/// the appropriate segment for each note.
///
///
/// # Field naming
///
/// The root module uses the **v2** field name `note_events`.
/// The [`v1`] module has its own [`SoundChannel`](crate::v1::SoundChannel)
/// with the v1 field `notes` (aliased via `#[serde(rename = "notes")]`).
///
/// # Slicing behaviour
///
/// 1. Collect all unique pulse offsets from `note_events`.
/// 2. Sort them and convert to real time (using BPM and resolution).
/// 3. Slice the audio file at those time points.
/// 4. Each note is assigned the slice that starts at its pulse.
///    Notes with `c: false` cause a restart at that point.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundChannel {
    /// Audio file name (relative path, extension may be omitted).
    ///
    /// The player will search for compatible audio files (`.wav`, `.ogg`,
    /// `.m4a`) when the extension is absent.
    ///
    /// # Security
    ///
    /// Implementations **must** prevent directory traversal and absolute
    /// paths (e.g. `../secret.txt`, `/etc/passwd`).
    pub name: String,

    /// Notes that reference this audio file.
    ///
    /// The field is serialised as `note_events` (v2 convention).
    pub note_events: Vec<NoteEvent>,
}

// ===========================================================================
// BarLine
// ===========================================================================

/// A **bar line** event marking a measure boundary in the chart.
///
/// Bmson has no native concept of measures or time signatures.  Instead,
/// bar lines are explicit markers that a player can render on-screen.
///
///
/// # Field behaviour
///
/// | Value | Meaning |
/// |---|---|
/// | empty array (`[]`) | No bar lines displayed – scroll behaves like [100% minimoo-G](https://www.youtube.com/watch?v=f1VBBNrSdgk) |
/// | `null` | Treated as 4/4 common time (bar line every 4 quarter‑notes = 960 pulses at default resolution) |
/// | `\[{ y }\]` | Bar lines at the given pulse offsets |
///
/// # Errata
///
/// v0.2.1 had an extra `k` field (now removed).  Use [`crate::v0::BarLine`]
/// when round‑tripping through v0.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarLine {
    /// Pulse offset of this bar line.
    ///
    /// The first bar line (`y: 0`) may be omitted; the player decides
    /// whether to render it.
    pub y: u64,
}

// ===========================================================================
// BpmEvent
// ===========================================================================

/// A **BPM change** event that alters the song tempo.
///
/// At pulse `y` the playback BPM is updated to `bpm`.
/// If multiple `BpmEvent` share the same pulse, the **last** one wins
/// (consistent with BMS behaviour).
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BpmEvent {
    /// Pulse offset at which the tempo change takes effect.
    pub y: u64,
    /// New tempo in beats per minute (BPM).
    pub bpm: f64,
}

// ===========================================================================
// StopEvent
// ===========================================================================

/// A **stop** (pause) event that halts the music scroll for a duration.
///
/// When multiple `StopEvent` share the same pulse the durations
/// **accumulate** (e.g. two stops of 240 and 960 pulses = 1200 total).
///
///
/// # Event order at the same pulse
///
/// 1. `Note` / `BGAEvent`
/// 2. `BpmEvent`
/// 3. `StopEvent`
///
/// The BPM value at the stop's pulse is used to compute the real‑time
/// duration of the pause.  Notes at the same pulse as a stop must be
/// pressed **before** the scroll halts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StopEvent {
    /// Pulse offset where the stop begins.
    pub y: u64,
    /// Duration of the stop in pulses (not seconds).
    ///
    /// Duration is "amount of music time that is skipped", converted to
    /// wall‑clock time via the active BPM.
    pub duration: u64,
}

// ===========================================================================
// BGA types
// ===========================================================================

/// Header entry for a BGA image or video resource.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BGAHeader {
    /// Numeric identifier referenced by [`BGAEvent::id`].
    ///
    /// Duplicate `id` values within the same file are a warning;
    /// the last occurrence wins.
    ///
    /// **Note:** v0.2.1 used the key `ID` (upper‑case) for this field.
    /// The `alias` attribute allows deserializing both `id` and `ID`.
    #[serde(alias = "ID")]
    pub id: u64,
    /// File path to the image or video resource.
    ///
    /// Supported formats: `PNG` (images), `WebM` (video, audio track is ignored).
    /// Recommended resolution: 1280×720; 1920×1080 is acceptable.
    pub name: String,
}

/// A BGA display event referencing a resource from [`BGAHeader`].
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BGAEvent {
    /// Pulse offset at which this image/video becomes visible.
    pub y: u64,
    /// Identifier of the resource in [`BGAHeader`] to display.
    pub id: u64,
}

/// Background Animation (BGA) data.
///
/// Holds three independent event tracks that the player can composite:
///
/// | Track | Purpose |
/// |---|---|
/// | `bga_events` | Primary background animation |
/// | `layer_events` | Overlay composited on top of the primary BGA |
/// | `poor_events` | Shown when the player misses notes |
///
///
/// # Transparency note
///
/// Unlike BMS `#LAYER` channels, black pixels in `layer_events` are **not**
/// automatically made transparent.  Use a PNG with actual alpha if you need
/// transparency.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BGA {
    /// Resource declarations (image/video id → filename mapping).
    #[serde(rename = "bga_header", alias = "bgaHeader")]
    pub bga_header: Vec<BGAHeader>,
    /// Primary background animation sequence.
    #[serde(rename = "bga_events", alias = "bgaNotes")]
    pub bga_events: Vec<BGAEvent>,
    /// Overlay sequence composited on top of the primary BGA.
    #[serde(rename = "layer_events", alias = "layerNotes")]
    pub layer_events: Vec<BGAEvent>,
    /// Poor‑performance (miss) animation sequence.
    #[serde(rename = "poor_events", alias = "poorNotes")]
    pub poor_events: Vec<BGAEvent>,
}

// ===========================================================================
// DJ.NEXT extensions (v2.0.0-rc1)
// ===========================================================================

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

// ===========================================================================
// beatoraja extensions
// ===========================================================================

/// A scroll‑speed multiplier event (beatoraja extension).
///
/// Analogous to BMS `#SCROLL` / `#SPEED`.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScrollEvent {
    /// Pulse offset.
    pub y: u64,
    /// Speed multiplier (negative → reverse scroll).
    pub rate: f64,
}

/// A mine (landmine) channel (beatoraja extension).
///
/// Each channel groups mine notes that share the same sound file.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MineChannel {
    /// Audio file name (played when a mine is triggered).
    pub name: String,
    /// Mine notes in this channel.
    pub notes: Vec<MineNote>,
}

/// A single mine note (beatoraja extension).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MineNote {
    /// Player channel (same semantics as [`NoteEvent::x`]).
    pub x: u64,
    /// Pulse offset.
    pub y: u64,
    /// Health damage (supports fractional values).
    pub damage: f64,
}

/// An invisible ("key") channel (beatoraja extension).
///
/// Notes in this channel are neither displayed nor judged, but their
/// audio is triggered when the player presses the corresponding key
/// at the right time.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyChannel {
    /// Audio file name.
    pub name: String,
    /// Invisible notes in this channel.
    pub notes: Vec<KeyNote>,
}

/// A single invisible note (beatoraja extension).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyNote {
    /// Player channel.
    pub x: u64,
    /// Pulse offset.
    pub y: u64,
}

// ===========================================================================
// Root (v2) top-level types
// ===========================================================================

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

// ===========================================================================
// Helper: treat JSON `null` as the default value for `Vec<T>`
// (both v1 and v2 spec mark `bpm_events`/`stop_events` as nullable `?`).
// ===========================================================================

/// Deserialise `null` as [`Default::default()`] for any type `T`.
///
/// Used via `#[serde(deserialize_with = "null_to_default")]` on fields where
/// the bmson spec allows `null` but we prefer the simpler `Vec<T>` type.
pub(crate) fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

// ===========================================================================
// Default helpers (used by serde `default` attributes)
// ===========================================================================

fn default_multiplier() -> f64 {
    1.00
}

/// Resolution default (240 ticks per quarter-note).
#[doc(hidden)]
pub(crate) fn default_resolution() -> u64 {
    240
}
