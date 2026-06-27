//! Format-agnostic rhythm game chart data model.
//!
//! [`Chart`] is the central type: an intermediate representation produced
//! by format processors (`bmson-processor`, `bms-processor`) and consumed
//! by the player (`bmsrs-player`).
//!
//! # Time model
//!
//! All event positions are absolute [`u64`] ticks. The global
//! [`resolution`][Chart::resolution] defines ticks per quarter note
//! (default 240). Wall-clock time is derived via
//! [`TimingTrack::tick_to_duration`][timing::TimingTrack::tick_to_duration].
//!
//! # Unified event timeline
//!
//! All timed events live in a single [`events`][Chart::events] vector,
//! sorted by tick.  Each variant of the [`Event`] enum represents a
//! different kind of event (note, BGM, BPM change, stop, scroll, BGA,
//! bar line, or format-specific custom event).
//!
//! # Generic parameters
//!
//! - `T: NoteExt` — per-note extension data (default `()`).
//! - `C: CustomEvent` — format-specific custom event type (default
//!   [`NoCustomEvent`]).
//!
//! # Example
//!
//! ```
//! use std::num::NonZeroU8;
//! use bmsrs_chart::{
//!     Chart, ChartMetadata, Event, Lane, NoteKind, NoteSide, TimingTrack,
//! };
//!
//! let chart: Chart = Chart {
//!     metadata: ChartMetadata {
//!         title: "Test".into(),
//!         ..Default::default()
//!     },
//!     resolution: 240,
//!     timing: TimingTrack {
//!         init_bpm: 120.0,
//!         bpm_changes: vec![],
//!         stops: vec![],
//!     },
//!     judge_multiplier: 1.0,
//!     life_multiplier: 1.0,
//!     events: vec![Event::Note {
//!         tick: 0,
//!         side: NoteSide::P1,
//!         lane: Lane::Key(NonZeroU8::new(1).unwrap()),
//!         kind: NoteKind::Normal,
//!         audio_index: None,
//!         ext: (),
//!     }],
//!     audio_assets: vec![],
//!     bga_resources: vec![],
//! };
//! assert_eq!(chart.resolution, 240);
//! assert_eq!(chart.events.len(), 1);
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

/// Chart metadata (song and difficulty information).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChartMetadata {
    /// Song title.
    pub title: String,
    /// Song subtitle (may be empty).
    pub subtitle: String,
    /// Primary artist.
    pub artist: String,
    /// Additional contributors (e.g. `["music:composer", "chart:charter"]`).
    pub subartists: Vec<String>,
    /// Song genre.
    pub genre: String,
    /// Chart name / difficulty label (e.g. `"HYPER"`, `"ANOTHER"`).
    pub chart_name: String,
    /// Numeric difficulty level (typically 1–12 for beat modes).
    pub level: u64,
}

/// Format-agnostic rhythm game chart.
///
/// Produced by format processors and consumed by the player.
/// All events are in a single vector sorted by tick ascending —
/// processors guarantee this, and the player relies on it for
/// binary-search queries.
///
/// Each note carries its position as `(NoteSide, Lane)` directly, so the
/// chart needs no separate mode field.
///
/// # Fields
///
/// | Field | Source |
/// |-------|--------|
/// | `metadata` | BMSON `SongInfo`/`ChartInfo` or BMS `Metadata` |
/// | `resolution` | BMSON `resolution` or processor-chosen (240 for BMS) |
/// | `timing` | BMSON `bpm_events`/`stop_events` or BMS `timing`/`messages` |
/// | `events` | unified timeline from all source events |
/// | `audio_assets` | BMSON sliced sound channels or BMS WAV table |
/// | `bga_resources` | BGA resource declarations |
#[derive(Clone, Debug, PartialEq)]
pub struct Chart<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Song and chart metadata.
    pub metadata: ChartMetadata,

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

    /// BGA resource declarations (image/video files).
    pub bga_resources: Vec<BgaResource>,
}

impl<T: NoteExt, C: CustomEvent> Chart<T, C> {
    /// Returns the last tick position of any event in the chart.
    ///
    /// Useful for computing total chart duration.
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        self.events.last().map_or(0, Event::tick)
    }

    /// Returns the total duration of the chart.
    #[must_use]
    #[inline]
    pub fn duration(&self) -> std::time::Duration {
        self.timing
            .tick_to_duration(self.last_tick(), self.resolution)
    }
}
