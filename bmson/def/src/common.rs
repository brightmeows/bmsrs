//! Types shared across bmson versions (v0, v1, v2).

use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::Path;

/// Game mode hint specifying the input layout.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
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
            Self::Generic(n) => write!(f, "generic-{n}keys"),
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
            _ if s.starts_with("generic-") && s.ends_with("keys") => {
                let inner = &s[8..s.len() - 4]; // strip "generic-" prefix and "keys" suffix
                inner
                    .parse()
                    .map_or_else(|_| Self::Other(s.to_owned()), Self::Generic)
            }
            _ if s.starts_with("generic-") && s.ends_with('k') => {
                let inner = &s[8..s.len() - 1]; // strip "generic-" prefix and "k" suffix (legacy compat)
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

/// Long-note type hint (`"ln"` or `"cn"`).
///
/// Can be set at the chart level ([`crate::ChartData::ln_type_hint`]) and overridden
/// per note ([`NoteEvent::ln_type_hint`]).
///
/// | Value | Meaning |
/// |---|---|
/// | `ln` | Judged on initial press only |
/// | `cn` | Judged on both press and release |
///
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LnLife {
    /// Only the note itself restores life.
    #[default]
    Normal,
    /// Extra ticks restore life during the hold.
    Ticks,
}

/// beatoraja long-note mode (numeric, v0 extension).
///
/// | Value | Variant | Meaning |
/// |---|---|---|
/// | `1` | `Ln` | LN — press only |
/// | `2` | `Cn` | CN — press + release |
/// | `3` | `Hcn` | HCN — hell charge note |
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
#[non_exhaustive]
pub enum LnMode {
    /// LN (1) — press only.
    Ln,
    /// CN (2) — press + release.
    Cn,
    /// HCN (3) — hell charge note.
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

/// A single note (playable or BGM) in a [`SoundChannel`].
///
/// In v2.0.0-rc1 this was renamed from `Note` to `NoteEvent`.
///
///
/// # Fields shared by all versions
///
/// | Field | Type | Description |
/// |---|---|---|
/// | `x` | `u64` | Player channel (`0` = BGM, `>0` = playable channel) |
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteEvent {
    /// Player channel (`0` = BGM, `>0` = playable key/column).
    /// Defaults to `0` when the field is absent. `null` is also accepted
    /// and treated as `0` (BGM).
    ///
    /// The exact mapping from channel number to on‑screen column depends
    /// on [`crate::ChartData::mode_hint`]:
    ///
    /// | Mode | Channels |
    /// |---|---|
    /// | `beat-7k` | 1–7 = keys, 8 = scratch |
    /// | `beat-5k` | 1–5 = keys, 8 = scratch |
    /// | `popn-9k` | 1–9 = keys |
    /// | `generic-nkeys` | 1…n left‑to‑right |
    #[serde(default, deserialize_with = "null_to_u64")]
    pub x: u64,

    /// Pulse offset of this note.
    ///
    /// All timing in bmson is relative to pulses (ticks), not seconds.
    /// The number of pulses per quarter‑note is given by [`crate::ChartData::resolution`] (default 240).
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

    /// Per-note long-note type override (beatoraja extension, numeric).
    ///
    /// | Value | Meaning |
    /// |---|---|---|
    /// | `1` | LN — press only |
    /// | `2` | CN — press + release |
    /// | `3` | HCN — hell charge note |
    ///
    /// See also [`NoteEvent::ln_type_hint`] for the v2 equivalent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<LnMode>,

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
        self.x == 0
    }
}

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
/// The [`crate::v1`] module has its own [`SoundChannel`](crate::v1::SoundChannel)
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct SoundChannel<'a> {
    /// Audio file name (relative path, extension may be omitted).
    ///
    /// The player will search for compatible audio files (`.wav`, `.ogg`,
    /// `.m4a`) when the extension is absent.
    ///
    /// # Security
    ///
    /// Implementations **must** prevent directory traversal and absolute
    /// paths (e.g. `../secret.txt`, `/etc/passwd`).
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,

    /// Notes that reference this audio file.
    ///
    /// The field is serialised as `note_events` (v2 convention).
    pub note_events: Vec<NoteEvent>,
}

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
#[derive(Clone, Debug, Default, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct BarLine {
    /// Pulse offset of this bar line.
    ///
    /// The first bar line (`y: 0`) may be omitted; the player decides
    /// whether to render it.
    pub y: u64,
}

/// A **BPM change** event that alters the song tempo.
///
/// At pulse `y` the playback BPM is updated to `bpm`.
/// If multiple `BpmEvent` share the same pulse, the **last** one wins
/// (consistent with BMS behaviour).
///
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
pub struct BpmEvent {
    /// Pulse offset at which the tempo change takes effect.
    pub y: u64,
    /// New tempo in beats per minute (BPM).
    pub bpm: f64,
}

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
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub struct StopEvent {
    /// Pulse offset where the stop begins.
    pub y: u64,
    /// Duration of the stop in pulses (not seconds).
    ///
    /// Duration is "amount of music time that is skipped", converted to
    /// wall‑clock time via the active BPM.
    pub duration: u64,
}

/// Header entry for a BGA image or video resource.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BGAHeader<'a> {
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
    #[serde(deserialize_with = "de_path")]
    pub name: &'a Path,
}

/// A BGA display event referencing a resource from [`BGAHeader`].
///
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BGA<'a> {
    /// Resource declarations (image/video id → filename mapping).
    #[serde(rename = "bga_header", alias = "bgaHeader")]
    pub bga_header: Vec<BGAHeader<'a>>,
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

/// A scroll‑speed multiplier event (beatoraja extension).
///
/// Analogous to BMS `#SCROLL` / `#SPEED`.
///
#[derive(Clone, Debug, PartialEq, Copy, Serialize, Deserialize)]
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
    /// Pulse offset.
    pub y: u64,
}

/// Deserialise `null` as [`Default::default()`] for any type `T`.
///
/// Used via `#[serde(deserialize_with = "null_to_default")]` on fields where
/// the bmson spec allows `null` but we prefer the simpler `Vec<T>` type.
///
/// # Errors
///
/// Delegates to `T`'s [`Deserialize`] implementation; returns an error if the
/// JSON value is neither `null` nor a valid `T`.
pub fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// Default value for `judge_multiplier` / `life_multiplier` (`1.00`).
#[must_use]
pub fn default_multiplier() -> f64 {
    1.00
}

/// Default pulse resolution (240 ticks per quarter-note).
#[must_use]
pub fn default_resolution() -> u64 {
    240
}

/// Deserialise a `u64` field, accepting `null` as `0`.
///
/// Used for [`NoteEvent::x`] where the spec allows `null` (→ BGM).
///
/// # Errors
///
/// Returns an error if the JSON value is neither `null` nor a valid unsigned integer.
pub fn null_to_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum X {
        Num(u64),
        Null,
    }
    match X::deserialize(deserializer)? {
        X::Num(n) => Ok(n),
        X::Null => Ok(0),
    }
}

/// Deserialise a `u64` resolution field, replacing `0` with the default `240`.
///
/// Per the bmson spec, a resolution of `0`, `null` or `undefined` must be
/// treated as `240`.  This helper handles the `0` case; `null`/`undefined`
/// are handled by `#[serde(default)]`.
///
/// # Errors
///
/// Returns an error if the JSON value is not a valid unsigned integer.
pub fn deserialize_resolution_nonzero<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<u64, D::Error> {
    let v = u64::deserialize(deserializer)?;
    if v == 0 { Ok(240) } else { Ok(v) }
}

/// Deserialise a JSON string as a borrowed `&Path`.
///
/// The JSON input must be a valid UTF‑8 string; the resulting `&Path`
/// reinterprets the same bytes as a path (zero‑copy).
///
/// # Errors
///
/// Returns an error if the JSON value is not a string.
pub fn de_path<'de, D: Deserializer<'de>>(deserializer: D) -> Result<&'de Path, D::Error> {
    let s: &'de str = Deserialize::deserialize(deserializer)?;
    Ok(Path::new(s))
}

/// Deserialise a JSON string or `null` as an `Option<&Path>`.
///
/// JSON `null` maps to `None`; a string maps to `Some(&Path)` reinterpreting
/// the same UTF‑8 bytes (zero‑copy).
///
/// # Errors
///
/// Returns an error if the JSON value is neither a string nor `null`.
pub fn de_opt_path<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<&'de Path>, D::Error> {
    let s: Option<&'de str> = Deserialize::deserialize(deserializer)?;
    Ok(s.map(Path::new))
}
