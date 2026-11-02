//! Types shared across bmson versions (v0, v1, and v2).

use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DefaultOnNull, serde_as};
use std::fmt;
use std::path::Path;

/// Game mode hint specifying the input layout.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModeHint {
    /// beat-5k (5 keys plus 1 scratch).
    Beat5k,
    /// beat-7k (7 keys plus 1 scratch).
    #[default]
    Beat7k,
    /// beat-10k (5 keys plus scratch per player, two players).
    Beat10k,
    /// beat-14k (7 keys plus scratch per player, two players).
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
    /// Generic n-key layout, with keys numbered 1..n from left to right.
    /// Serialized as `"generic-{n}k"`.
    Generic(u64),
    /// Any other (extension) mode hint outside the specification list.
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
            Self::Generic(n) => write!(f, "generic-{n}keys"),
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl core::str::FromStr for ModeHint {
    type Err = core::convert::Infallible;

    #[expect(
        clippy::string_slice,
        reason = "\"generic-\" and \"keys\"/\"k\" are ASCII; byte indexing is safe"
    )]
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
            _ if s.starts_with("generic-") && s.ends_with("keys") => {
                let inner = &s[8..s.len() - 4]; // Strip "generic-" and "keys".
                inner
                    .parse()
                    .map_or_else(|_| Self::Other(s.to_owned()), Self::Generic)
            }
            _ if s.starts_with("generic-") && s.ends_with('k') => {
                let inner = &s[8..s.len() - 1]; // Strip "generic-" and "k" (legacy).
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

/// Long-note type hint.
///
/// Can be set at the chart level ([`crate::ChartData::ln_type_hint`]) and
/// overridden per note ([`NoteEvent::ln_type_hint`]).
///
/// | Value | Meaning | Source |
/// |---|---|---|
/// | `ln` | judged only on press | v2 standard |
/// | `cn` | judged on both press and release | v2 standard |
/// | `hcn` | hell charge note, stricter judgment while held | beatoraja extension |
///
/// `hcn` is a beatoraja extension and is not part of the core bmson v2
/// specification. Players that do not support HCN should downgrade it to `cn`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LnType {
    /// Long note (LN): judged only on press.
    #[default]
    Ln,
    /// Charge note (CN): judged on both press and release.
    Cn,
    /// Hell charge note (HCN): beatoraja extension, not v2 standard.
    Hcn,
}

/// Long-note judgment hint (`"normal"` or `"ticks"`).
///
/// | Value | Meaning |
/// |---|---|
/// | `normal` | judge only the note itself |
/// | `ticks` | judge additional ticks while held |
///
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LnJudge {
    /// Judge only the note itself.
    #[default]
    Normal,
    /// Judge additional ticks while held.
    Ticks,
}

/// Long-note life hint (`"normal"` or `"ticks"`).
///
/// | Value | Meaning |
/// |---|---|
/// | `normal` | only the note itself restores life |
/// | `ticks` | additional ticks restore life while held |
///
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LnLife {
    /// Only the note itself restores life.
    #[default]
    Normal,
    /// Additional ticks restore life while held.
    Ticks,
}

/// beatoraja long-note mode (numeric, v0 extension).
///
/// | Value | Variant | Meaning |
/// |---|---|---|
/// | `1` | `Ln` | LN: press only |
/// | `2` | `Cn` | CN: press and release |
/// | `3` | `Hcn` | HCN: hell charge note |
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
#[non_exhaustive]
pub enum LnMode {
    /// LN (1): press only.
    Ln,
    /// CN (2): press and release.
    Cn,
    /// HCN (3): hell charge note.
    Hcn,
}

impl Serialize for LnMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Ln => serializer.serialize_u64(1),
            Self::Cn => serializer.serialize_u64(2),
            Self::Hcn => serializer.serialize_u64(3),
        }
    }
}

impl<'de> Deserialize<'de> for LnMode {
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

/// A single note (playable or BGM) inside a [`SoundChannel`].
///
/// Renamed from `Note` to `NoteEvent` in v2.0.0-rc1.
///
///
/// # Fields shared across versions
///
/// | Field | Type | Meaning |
/// |---|---|---|
/// | `x` | `u64` | player channel (`0` = BGM, `>0` = playable channel) |
/// | `y` | `u64` | tick offset |
/// | `l` | `u64` | tick length (`0` = short note, `>0` = long note) |
/// | `c` | `bool` | continuation flag (audio replay behavior) |
///
/// # v2-only optional fields
///
/// These fields exist only in v2.0.0-rc1 and later; they are `None` when
/// deserializing v0/v1 files:
///
/// | Field | Type | Meaning |
/// |---|---|---|
/// | `up` | `bool` | release note / BSS flag |
/// | `ln_type_hint` | [`LnType`] | per-note long-note type override |
/// | `ln_judge_hint` | [`LnJudge`] | per-note long-note judgment override |
/// | `ln_life_hint` | [`LnLife`] | per-note long-note life override |
/// | `vol` | `i8` | volume (percentage, DJ.NEXT) |
/// | `pan` | `i8` | pan (DJ.NEXT) |
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteEvent {
    /// Player channel (`0` = BGM, `>0` = playable key/lane). Defaults to `0` when
    /// the field is missing. Also accepts `null`, treated as `0` (BGM).
    ///
    /// The exact mapping from channel number to screen lane depends on
    /// [`crate::ChartData::mode_hint`]:
    ///
    /// | Mode | Channels |
    /// |---|---|
    /// | `beat-7k` | 1-7 = keys, 8 = scratch |
    /// | `beat-5k` | 1-5 = keys, 8 = scratch |
    /// | `popn-9k` | 1-9 = keys |
    /// | `generic-nkeys` | 1..n from left to right |
    #[serde(default)]
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub x: u64,

    /// Tick offset of this note.
    ///
    /// All timing in bmson is based on ticks, not seconds. Ticks per quarter note
    /// are given by [`crate::ChartData::resolution`] (default 240).
    pub y: u64,

    /// Note length in ticks.
    ///
    /// | Value | Meaning |
    /// |---|---|
    /// | `0` | short (normal) note, no hold |
    /// | `>0` | long note from tick `y` through `y + l` |
    pub l: u64,

    /// Continuation flag: whether the audio slice is replayed.
    ///
    /// - `true`: **continue** (do not replay); audio continues from the previous
    ///   slice.
    /// - `false`: **do not continue**; replay the audio at this note's slice
    ///   point.
    pub c: bool,

    /// Per-note long-note type override (beatoraja extension, numeric).
    ///
    /// | Value | Meaning |
    /// |---|---|---|
    /// | `1` | LN: press only |
    /// | `2` | CN: press and release |
    /// | `3` | HCN: hell charge note |
    ///
    /// See also the v2 equivalent, [`NoteEvent::ln_type_hint`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<LnMode>,

    /// Release note / BSS (Back-Spin-Scratch) flag.
    ///
    /// For CN (charge notes) or BSS, place a zero-length `NoteEvent` with
    /// `up: true` at the release position.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<bool>,

    /// Per-note override of the chart-level long-note type hint.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_type_hint: Option<LnType>,

    /// Per-note override of the chart-level long-note judgment hint.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ln_judge_hint: Option<LnJudge>,

    /// Per-note override of the chart-level long-note life hint.
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

/// Backfill per-note `t` (`LnMode`) into `ln_type_hint` (`LnType`).
///
/// Backfills only when `ln_type_hint` is unset, preserving explicit values. Shared
/// by the v0 and v1 upgrade paths.
pub fn backfill_ln_type_hint(note: &mut NoteEvent) {
    if note.ln_type_hint.is_none() {
        note.ln_type_hint = note.t.map(crate::ln_mode_to_type_hint);
    }
}

impl NoteEvent {
    /// Return `true` if this note is a BGM note (not playable).
    #[must_use]
    #[inline]
    pub const fn is_bgm(&self) -> bool {
        self.x == 0
    }
}

/// **Sound channel**: one audio file and its associated notes.
///
/// Bmson is channel-based: each [`SoundChannel`] binds one audio file (`name`)
/// to every [`NoteEvent`] that references it. The player slices the audio file
/// at note positions and plays the corresponding slice for each note.
///
///
/// # Field naming
///
/// The root module uses the **v2** field name `note_events`. The [`crate::v1`]
/// module has its own [`SoundChannel`](crate::v1::SoundChannel) using the v1 field
/// name `notes` (through a `#[serde(rename = "notes")]` alias).
///
/// # Slicing behavior
///
/// 1. Collect every unique tick offset from `note_events`.
/// 2. Sort them and convert to actual time (using BPM and resolution).
/// 3. Slice the audio file at those times.
/// 4. Assign each note to the slice starting at its tick. Notes with
///    `c: false` trigger a replay at that point.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct SoundChannel<'a> {
    /// Audio file name (relative path; the extension may be omitted).
    ///
    /// When the extension is missing, the player searches for compatible audio
    /// files (`.wav`, `.ogg`, `.m4a`).
    ///
    /// # Security
    ///
    /// Implementations **must** prevent directory traversal and absolute paths
    /// (for example, `../secret.txt` or `/etc/passwd`).
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,

    /// Notes referencing this audio file.
    ///
    /// This field serializes as `note_events` (v2 convention).
    pub note_events: Vec<NoteEvent>,
}

/// **Barline** event marking measure boundaries in a chart.
///
/// Bmson natively has no concept of measures or time signatures. Barlines are
/// explicit markers that players may render on screen.
///
///
/// # Field behavior
///
/// | Value | Meaning |
/// |---|---|
/// | Empty array (`[]`) | show no barlines; the scroll effect resembles [100% minimoo-G](https://www.youtube.com/watch?v=f1VBBNrSdgk) |
/// | `null` | treat as regular 4/4 (one barline every 4 quarter notes, or 960 ticks at the default resolution) |
/// | `\[{ y }\]` | place a barline at the given tick offset |
///
/// # Errata
///
/// v0.2.1 has an extra `k` field (now removed). Use [`crate::v0::BarLine`] when
/// roundtripping through v0.
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct BarLine {
    /// Tick offset of this barline.
    ///
    /// The first barline (`y: 0`) may be omitted; rendering it is up to the
    /// player.
    pub y: u64,
}

/// **BPM change** event that changes the song tempo.
///
/// At tick `y`, the playback BPM updates to `bpm`. If several `BpmEvent`s share
/// the same tick, the **last** one wins (matching BMS behavior).
///
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct BpmEvent {
    /// Tick offset where the tempo change takes effect.
    pub y: u64,
    /// New tempo in beats per minute (BPM).
    pub bpm: f64,
}

/// **Stop** event that pauses music scrolling for a duration.
///
/// When several `StopEvent`s share the same tick, their durations **accumulate**
/// (for example, stops of 240 and 960 ticks total 1200).
///
///
/// # Event order at the same tick
///
/// 1. `Note` / `BGAEvent`
/// 2. `BpmEvent`
/// 3. `StopEvent`
///
/// The BPM at the stop's tick is used to compute the actual pause duration.
/// Notes at the same tick as a stop must be pressed **before** scrolling stops.
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct StopEvent {
    /// Tick offset where the stop begins.
    pub y: u64,
    /// Stop duration in ticks, not seconds.
    ///
    /// The duration is the amount of musical time skipped, converted to actual
    /// time through the current BPM.
    pub duration: u64,
}

/// Header entry for a BGA image or video resource.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BGAHeader<'a> {
    /// Numeric identifier referenced by [`BGAEvent::id`].
    ///
    /// Duplicate `id` values in the same file produce a warning; the last
    /// occurrence wins.
    ///
    /// **Note:** v0.2.1 uses the uppercase key `ID` for this field. The `alias`
    /// attribute allows deserializing both `id` and `ID`.
    #[serde(alias = "ID")]
    pub id: u64,
    /// File path of the image or video resource.
    ///
    /// Supported formats: `PNG` (image) and `WebM` (video; the audio track is
    /// ignored). Recommended resolution: 1280x720; 1920x1080 is also accepted.
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,
}

/// BGA display event referencing a resource in [`BGAHeader`].
///
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct BGAEvent {
    /// Tick offset where this image/video becomes visible.
    pub y: u64,
    /// Identifier of the [`BGAHeader`] resource to display.
    pub id: u64,
}

/// Background animation (BGA) data.
///
/// Contains three independent event tracks that the player can composite:
///
/// | Track | Purpose |
/// |---|---|
/// | `bga_events` | main background animation |
/// | `layer_events` | overlay layers on top of the main BGA |
/// | `poor_events` | POOR animation shown when the player misses a note |
///
///
/// # Transparency note
///
/// Unlike BMS `#LAYER` channels, black pixels in `layer_events` do **not**
/// automatically become transparent. For transparency, use a PNG with a real
/// alpha channel.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BGA<'a> {
    /// Resource declarations (image/video id to file-name mapping).
    #[serde(rename = "bga_header", alias = "bgaHeader")]
    pub bga_header: Vec<BGAHeader<'a>>,
    /// Main background animation sequence.
    #[serde(rename = "bga_events", alias = "bgaNotes")]
    pub bga_events: Vec<BGAEvent>,
    /// Overlay layer sequence on top of the main BGA.
    #[serde(rename = "layer_events", alias = "layerNotes")]
    pub layer_events: Vec<BGAEvent>,
    /// POOR (miss) animation sequence.
    #[serde(rename = "poor_events", alias = "poorNotes")]
    pub poor_events: Vec<BGAEvent>,
}

/// Scroll speed multiplier event (beatoraja extension).
///
/// Similar to BMS `#SCROLL` / `#SPEED`.
///
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct ScrollEvent {
    /// Tick offset.
    pub y: u64,
    /// Speed multiplier (negative values scroll in reverse).
    pub rate: f64,
}

/// Mine (landmine) channel (beatoraja extension).
///
/// Each channel groups mine notes that share the same audio file.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct MineChannel<'a> {
    /// Audio file name (played when a mine is triggered).
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,
    /// Mine notes in this channel.
    pub notes: Vec<MineNote>,
}

/// A single mine note (beatoraja extension).
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct MineNote {
    /// Player channel (same semantics as [`NoteEvent::x`]).
    pub x: u64,
    /// Tick offset.
    pub y: u64,
    /// Life damage (fractional values supported).
    pub damage: f64,
}

/// Invisible ("key") channel (beatoraja extension).
///
/// Notes in this channel are neither displayed nor judged, but their audio is
/// triggered when the player presses the corresponding key at the right time.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct KeyChannel<'a> {
    /// Audio file name.
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,
    /// Invisible notes in this channel.
    pub notes: Vec<KeyNote>,
}

/// A single invisible note (beatoraja extension).
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct KeyNote {
    /// Player channel.
    pub x: u64,
    /// Tick offset.
    pub y: u64,
}

/// Deserialize `null` into [`Default::default()`] for any type `T`.
///
/// Used through `#[serde(deserialize_with = "null_to_default")]` for fields
/// where the bmson specification allows `null` but a simpler `Vec<T>` type is
/// preferred.
///
/// # Errors
///
/// Delegates to `T`'s [`Deserialize`] implementation; returns an error when the
/// JSON value is neither `null` nor a valid `T`.
#[inline]
pub fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// Default value for `judge_multiplier` / `life_multiplier` (`1.00`).
#[must_use]
#[inline]
pub const fn default_multiplier() -> f64 {
    1.00
}

/// Default beat resolution (240 ticks per quarter note).
#[must_use]
#[inline]
pub const fn default_resolution() -> u64 {
    240
}

/// Deserialize a `u64` resolution field, rejecting `0`.
///
/// The bmson specification requires falling back to the default `240` for `0`,
/// `null`, or `undefined`; this implementation is **stricter than the
/// specification**: `0` is treated as an authoring-tool defect and rejects the
/// whole file instead of silently rewriting it. `null`/`undefined` still go
/// through [`default_resolution`](crate::default_resolution) via
/// `#[serde(default)]`.
///
/// # Errors
///
/// Returns an error when the JSON value is not a valid unsigned integer or is
/// `0`.
#[inline]
pub fn deserialize_resolution_nonzero<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<u64, D::Error> {
    let v = u64::deserialize(deserializer)?;
    if v == 0 {
        Err(de::Error::custom(
            "resolution must be > 0 (omit the field for the default 240)",
        ))
    } else {
        Ok(v)
    }
}

/// Deserialize a JSON string into a borrowed `&Path`.
///
/// The JSON input must be a valid UTF-8 string; the resulting `&Path`
/// reinterprets the same bytes as a path (zero-copy).
///
/// # Errors
///
/// Returns an error when the JSON value is not a string.
#[inline]
pub fn de_path<'de, D: Deserializer<'de>>(deserializer: D) -> Result<&'de Path, D::Error> {
    let s: &'de str = Deserialize::deserialize(deserializer)?;
    Ok(Path::new(s))
}

/// Deserialize a JSON string or `null` into `Option<&Path>`.
///
/// JSON `null` maps to `None`; a string maps to `Some(&Path)`, reinterpreting
/// the same UTF-8 bytes as a path (zero-copy).
///
/// # Errors
///
/// Returns an error when the JSON value is neither a string nor `null`.
#[inline]
pub fn de_opt_path<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<&'de Path>, D::Error> {
    let s: Option<&'de str> = Deserialize::deserialize(deserializer)?;
    Ok(s.map(Path::new))
}
