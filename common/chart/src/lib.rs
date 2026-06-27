//! Format-agnostic rhythm game chart data model.
//!
//! [`Chart`] is the central type: an intermediate representation produced
//! by format processors (`bmson-processor`, `bms-processor`) and consumed
//! by the player (`bmsrs-player`).
//!
//! # Structure
//!
//! The chart is split into three sub-structs mirroring the BMSON v2 format:
//!
//! | Component | bmson v2 equivalent | Contents |
//! |-----------|-------------------|----------|
//! | [`SongInfo`] | `SongInfo` | Song-level metadata (title, artist, genre) |
//! | [`ChartInfo`] | `ChartInfo` | Chart-level metadata + BGA resources |
//! | [`ChartData`] | `ChartData` | Gameplay data (timing, events, audio) |
//!
//! # Time model
//!
//! All event positions are absolute [`u64`] ticks. The global
//! [`resolution`](ChartData::resolution) defines ticks per quarter note
//! (default 240). Wall-clock time is derived via
//! [`TimingTrack::tick_to_duration`].
//!
//! # Unified event timeline
//!
//! All timed events live in a single [`events`](ChartData::events) vector,
//! sorted by tick.  Each variant of the [`Event`] enum represents a
//! different kind of event (note, BGM, BPM change, stop, scroll, BGA,
//! bar line, or format-specific custom event).
//!
//! # Generic parameters
//!
//! - `T: NoteExt` — per-note extension data (default `()`).
//! - `C: CustomEvent` — format-specific custom event type
//!   (default [`NoCustomEvent`]).
//!
//! # Example
//!
//! ```
//! use std::num::NonZeroU8;
//! use bmsrs_chart::{
//!     Chart, SongInfo, ChartInfo, ChartData, Event, Lane, NoteKind, NoteSide,
//!     TimingTrack,
//! };
//!
//! let chart: Chart = Chart {
//!     song: SongInfo {
//!         title: "Test".into(),
//!         ..Default::default()
//!     },
//!     chart: ChartInfo::default(),
//!     data: ChartData {
//!         resolution: 240,
//!         timing: TimingTrack {
//!             init_bpm: 120.0,
//!             bpm_changes: vec![],
//!             stops: vec![],
//!         },
//!         events: vec![Event::Note {
//!             tick: 0,
//!             side: NoteSide::P1,
//!             lane: Lane::Key(NonZeroU8::new(1).unwrap()),
//!             kind: NoteKind::Normal,
//!             audio_index: None,
//!             ext: (),
//!         }],
//!         audio_assets: vec![],
//!         ..Default::default()
//!     },
//! };
//! assert_eq!(chart.song.title, "Test");
//! assert_eq!(chart.data.events.len(), 1);
//! ```

pub mod audio;
pub mod event;
pub mod mode;
pub mod note;
pub mod timing;
pub mod visual;

pub use audio::AudioAsset;
pub use event::{CustomEvent, Event, NoCustomEvent, NoteExt};
pub use mode::{Lane, NoteSide};
pub use note::NoteKind;
pub use timing::{BpmChange, StopEvent, TimingTrack};
pub use visual::{BgaLayer, BgaResource};

/// Song-level metadata — mirrors bmson v2's `SongInfo`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SongInfo {
    /// Song title.
    pub title: String,
    /// Primary artist.
    pub artist: String,
    /// Song genre.
    pub genre: String,
    /// Additional contributors (e.g. `["music:composer", "chart:charter"]`).
    pub subartists: Vec<String>,
}

/// Chart-level metadata and resources — mirrors bmson v2's `ChartInfo`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChartInfo {
    /// Song subtitle (may be empty).
    pub subtitle: String,
    /// Chart name / difficulty label (e.g. `"HYPER"`, `"ANOTHER"`).
    pub chart_name: String,
    /// Numeric difficulty level (typically 1–12 for beat modes).
    pub level: u64,
    /// Background image path (displayed during gameplay).
    pub back_image: Option<String>,
    /// Eyecatch image path (displayed during loading).
    pub eyecatch_image: Option<String>,
    /// Banner image path (displayed in song selection).
    pub banner_image: Option<String>,
    /// Preview music path (short audio clip for song selection).
    pub preview_music: Option<String>,
    /// BGA resource declarations (image/video files).
    pub bga_resources: Vec<BgaResource>,
}

/// Gameplay data — mirrors bmson v2's `ChartData`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartData<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Ticks per quarter note (pulse resolution).
    pub resolution: u64,
    /// Timing track for tick ↔ seconds conversion.
    pub timing: TimingTrack,
    /// Judgement window multiplier (`1.0` = normal).
    pub judge_multiplier: f64,
    /// Life gauge multiplier (`1.0` = normal).
    pub life_multiplier: f64,
    /// All timed events, sorted by tick ascending.
    pub events: Vec<Event<T, C>>,
    /// Audio assets referenced by note and BGM events.
    pub audio_assets: Vec<AudioAsset>,
}

impl<T: NoteExt, C: CustomEvent> ChartData<T, C> {
    /// Returns the last tick position of any event in the chart data.
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        self.events.last().map_or(0, Event::tick)
    }

    /// Returns the total duration of the chart data.
    #[must_use]
    #[inline]
    pub fn duration(&self) -> std::time::Duration {
        self.timing
            .tick_to_duration(self.last_tick(), self.resolution)
    }
}

/// Top-level chart — mirrors bmson v2's `Bmson` root object.
///
/// Contains song metadata, chart metadata, and gameplay data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chart<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Song-level metadata (title, artist, genre).
    pub song: SongInfo,
    /// Chart-level metadata and resources.
    pub chart: ChartInfo,
    /// Gameplay data (timing, events, audio).
    pub data: ChartData<T, C>,
}
