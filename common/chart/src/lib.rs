//! Format-agnostic music game chart data model.
//!
//! [`Chart`] is the core type: the intermediate representation produced by the
//! format processors (`bmsrs-bmson-processor` and `bmsrs-bms-processor`) and
//! consumed by the player (`bmsrs-player`).
//!
//! # Structure
//!
//! A chart is split into three sub-structures corresponding to BMSON v2:
//!
//! | Component | BMSON v2 counterpart | Content |
//! |------|--------------|------|
//! | [`SongInfo`] | `SongInfo` | song-level metadata (title, artist, genre) |
//! | [`ChartInfo`] | `ChartInfo` | chart-level metadata plus BGA resources |
//! | [`ChartData`] | `ChartData` | gameplay data (timing, events, audio) |
//!
//! # Time model
//!
//! All event positions are absolute [`u64`] ticks. The global
//! [`TimingTrack::resolution`](TimingTrack::resolution) defines ticks per quarter
//! note (default 240). Wall-clock time is obtained through
//! [`TimingTrack::tick_to_duration`].
//!
//! # Unified event timeline
//!
//! All timed events live in a single [`events`](ChartData::events) vector sorted
//! by tick. Each variant of the [`Event`] enum represents one event kind (note,
//! BGM, BPM change, stop, scroll, BGA, barline, or a format-specific custom
//! event).
//!
//! # Generic parameters
//!
//! - `T: NoteExt`: per-note extension data (default `()`).
//! - `C: CustomEvent`: format-specific custom event type (default
//!   [`NoCustomEvent`]).
//!
//! # Erasing format extensions
//!
//! Processors may produce a `Chart` with format extensions (such as
//! `BmsonNoteExt` or `BmsCustomEvent`). After consuming those extensions,
//! callers can use [`ChartData::filter_map_events`] to erase the chart to
//! `Chart<(), NoCustomEvent>` for renderers that only depend on the core IR. To
//! replace only the per-note extension while keeping custom events, use
//! [`ChartData::map_events`].
//!
//! # Example
//!
//! ```
//! use std::num::NonZeroU8;
//! use bmsrs_chart::{
//!     Chart, SongInfo, ChartInfo, ChartData, Event, EventKind, Lane, LnJudgeHint, LnLifeHint,
//!     LnTypeHint, NoteKind, NoteSide, TimingTrack,
//! };
//!
//! let chart: Chart = Chart {
//!     song: SongInfo {
//!         title: "Test".into(),
//!         ..Default::default()
//!     },
//!     chart: ChartInfo::default(),
//!     data: ChartData {
//!         timing: TimingTrack::simple(120.0, 240).unwrap(),
//!         judge_multiplier: Some(1.0),
//!         life_multiplier: Some(1.0),
//!         ln_type_hint: LnTypeHint::default(),
//!         ln_judge_hint: LnJudgeHint::default(),
//!         ln_life_hint: LnLifeHint::default(),
//!         judge_deltas: None,
//!         life_deltas: None,
//!         events: vec![Event::new(
//!             0,
//!             EventKind::Note {
//!                 side: NoteSide::P1,
//!                 lane: Lane::Key(NonZeroU8::new(1).unwrap()),
//!                 kind: NoteKind::Normal,
//!                 role: bmsrs_chart::NoteRole::Normal,
//!                 audio: bmsrs_chart::NoteAudio::none(),
//!                 ext: (),
//!             },
//!         )],
//!         audio_assets: vec![],
//!     },
//! };
//! assert_eq!(chart.song.title, "Test");
//! assert_eq!(chart.data.events.len(), 1);
//! ```

use std::fmt;

pub mod audio;
pub mod event;
pub mod mode;
pub mod note;
pub mod timing;
pub mod visual;

pub use audio::{AudioAsset, AudioIndex};
pub use event::{CustomEvent, Event, EventKind, NoCustomEvent, NoteExt};
pub use mode::{Lane, NoteSide};
pub use note::{Damage, LnJudgeHint, LnLifeHint, LnTypeHint, NoteAudio, NoteKind, NoteRole};
pub use timing::{BpmChange, BpmLookup, StopEvent, TimingTrack, TimingTrackError};
pub use visual::{BgaLayer, BgaResource, BgaResourceId, CropRect, VideoAsset};

/// Song-level metadata, corresponding to BMSON v2's `SongInfo`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SongInfo {
    /// Song title.
    pub title: String,
    /// Main artist.
    pub artist: String,
    /// Song genre.
    pub genre: String,
}

/// Chart-level metadata and resources, corresponding to BMSON v2's `ChartInfo`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChartInfo {
    /// Song subtitle (may be empty).
    pub subtitle: String,
    /// Chart name / difficulty label (for example, `"HYPER"`, `"ANOTHER"`).
    pub chart_name: String,
    /// Numeric difficulty level (usually 1-12 in beat modes).
    pub level: u64,
    /// Other contributors (for example, `["music:composer", "chart:charter"]`).
    pub subartists: Vec<String>,
    /// Title image path (shown before the song starts).
    pub title_image: Option<String>,
    /// Background image path (shown during gameplay).
    pub back_image: Option<String>,
    /// Eyecatch image path (shown while loading).
    pub eyecatch_image: Option<String>,
    /// Banner image path (shown during song selection).
    pub banner_image: Option<String>,
    /// Preview audio path (short clip during song selection).
    pub preview_music: Option<String>,
    /// BGA resource declarations (image and video files).
    pub bga_resources: Vec<BgaResource>,
    /// Background video (BMS `#VIDEOFILE` / `#MOVIE`). BMSON has no such
    /// concept, so it is `None`.
    pub video: Option<VideoAsset>,
    /// Total playable keys; `None` means the source format did not provide it or
    /// it cannot be inferred.
    pub key_count: Option<u8>,
}

/// Custom judgment-window offsets (DJ.NEXT extension).
///
/// Each field specifies an additional offset (milliseconds) for that judgment
/// level's window. Sourced from BMSON `judgement_deltas`; BMS charts have no
/// such concept (processors fill `None`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

/// Custom life gauge deltas (DJ.NEXT extension).
///
/// Each field specifies the life change (percentage) for that judgment level;
/// negative values drain life. Sourced from BMSON `life_deltas`; BMS charts have
/// no such concept (processors fill `None`).
///
/// Guaranteed to contain no NaN (BMSON is deserialized through serde, and JSON
/// cannot contain NaN).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LifeDeltas {
    // Manual Eq: guaranteed to contain no NaN.
    /// Life change on PERFECT (percentage, may be negative).
    pub perfect: f64,
    /// Life change on GREAT (percentage, may be negative).
    pub great: f64,
    /// Life change on GOOD (percentage, may be negative).
    pub good: f64,
    /// Life change on MISS (percentage, may be negative).
    pub miss: f64,
}

// LifeDeltas is guaranteed to contain no NaN, so Eq is safe.
impl Eq for LifeDeltas {}

/// `ChartData` validation error type.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ChartDataError {
    /// An event references an out-of-range audio asset index.
    DanglingAudioIndex {
        /// Out-of-range index value.
        index: AudioIndex,
        /// Audio asset table length.
        len: usize,
    },
    /// A BGA event references an out-of-range resource id.
    ///
    /// Both processors renumber to dense indices, so `resource_id` must fall
    /// within [`ChartInfo::bga_resources`](crate::ChartInfo::bga_resources).
    DanglingBgaResource {
        /// Out-of-range resource id.
        id: BgaResourceId,
        /// BGA resource table length.
        len: usize,
    },
    /// Events are not sorted ascending by `(tick, priority)`.
    UnsortedEvents,
}

impl fmt::Display for ChartDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DanglingAudioIndex { index, len } => {
                write!(f, "audio index {index} out of range (0..{len})")
            }
            Self::DanglingBgaResource { id, len } => {
                write!(f, "bga resource id {id} out of range (0..{len})")
            }
            Self::UnsortedEvents => write!(f, "events are not sorted by (tick, priority)"),
        }
    }
}

impl std::error::Error for ChartDataError {}

/// Gameplay data, corresponding to BMSON v2's `ChartData`.
///
/// Does not implement [`Default`]: a valid state requires a [`timing`](TimingTrack)
/// with a valid initial BPM and a positive `resolution`, so there is no
/// meaningful zero default. Callers must provide the timing track explicitly
/// (processors construct it with literals). Beat resolution is owned solely by
/// [`TimingTrack`] and queried through
/// [`TimingTrack::resolution`](TimingTrack::resolution).
#[derive(Clone, Debug, PartialEq)]
pub struct ChartData<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Timing track used for tick to second conversion.
    pub timing: TimingTrack,
    /// Judgment window multiplier (`Some(1.0)` is standard); `None` means the
    /// source did not specify it.
    pub judge_multiplier: Option<f64>,
    /// Life gauge multiplier (`Some(1.0)` is standard); `None` means the source
    /// did not specify it.
    pub life_multiplier: Option<f64>,
    /// Chart-level long-note type hint (BMSON LN/CN).
    pub ln_type_hint: LnTypeHint,
    /// Chart-level long-note judgment hint.
    pub ln_judge_hint: LnJudgeHint,
    /// Chart-level long-note life hint.
    pub ln_life_hint: LnLifeHint,
    /// Custom judgment-window offsets (DJ.NEXT extension; `None` for BMS).
    pub judge_deltas: Option<JudgementDeltas>,
    /// Custom life gauge deltas (DJ.NEXT extension; `None` for BMS).
    pub life_deltas: Option<LifeDeltas>,
    /// All timed events, sorted ascending by tick.
    pub events: Vec<Event<T, C>>,
    /// Audio assets referenced by note and BGM events.
    pub audio_assets: Vec<AudioAsset>,
}

// judge_multiplier and life_multiplier are guaranteed to contain no NaN.
impl<T: NoteExt + Eq, C: CustomEvent + Eq> Eq for ChartData<T, C> {}

impl<T: NoteExt, C: CustomEvent> ChartData<T, C> {
    /// Return the tick of the last event in the chart data.
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        self.events.last().map_or(0, Event::tick)
    }

    /// Return the total duration of the chart data.
    #[must_use]
    #[inline]
    pub fn duration(&self) -> std::time::Duration {
        self.timing.tick_to_duration(self.last_tick())
    }

    /// Validate the key invariants of the chart data.
    ///
    /// Checks:
    /// - events are sorted ascending by [`Event::sort_key`]
    /// - reference integrity: audio indices of `Note`/`Bgm` events fall within
    ///   [`audio_assets`](Self::audio_assets)
    ///
    /// Timing track validity (valid initial BPM and positive resolution) is
    /// guaranteed by [`TimingTrack`] construction, so invalid states cannot exist
    /// after construction.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when any check fails.
    pub fn validate(&self) -> Result<(), ChartDataError> {
        if self.events.windows(2).any(|pair| match pair {
            [left, right] => left.sort_key() > right.sort_key(),
            _ => false,
        }) {
            return Err(ChartDataError::UnsortedEvents);
        }

        let len = self.audio_assets.len();
        for event in &self.events {
            let audio_indices: &[AudioIndex] = match &event.kind {
                EventKind::Note { audio, .. } => audio.as_slice(),
                EventKind::Bgm { audio_index } => std::slice::from_ref(audio_index),
                _ => &[],
            };
            for &audio_index in audio_indices {
                if audio_index.get() as usize >= len {
                    return Err(ChartDataError::DanglingAudioIndex {
                        index: audio_index,
                        len,
                    });
                }
            }
        }
        Ok(())
    }

    /// Ensure events are sorted ascending by tick.
    ///
    /// [`Player`]'s binary search (`partition_point`) depends on sorted events.
    /// [`Player::new`](https://docs.rs/bmsrs-player/latest/bmsrs_player/struct.Player.html#method.new)
    /// calls this method automatically; downstream consumers that read `events`
    /// without going through
    /// [`Player`](https://docs.rs/bmsrs-player/latest/bmsrs_player/struct.Player.html)
    /// must call it once themselves.
    ///
    /// The sort key is [`Event::sort_key`]: within the same tick, events sort by
    /// priority (Bar < Note/BGA/BGM < BPM < Stop < Scroll < Speed < Custom). The
    /// stable sort preserves insertion order for equal priorities.
    ///
    /// [`Player`]: https://docs.rs/bmsrs-player/latest/bmsrs_player/struct.Player.html
    pub fn sort_events(&mut self) {
        self.events.sort_by_key(Event::sort_key);
    }

    /// Transform every event through a mapping function and return a new
    /// `ChartData` with all other fields unchanged.
    ///
    /// The `NoteExt` type may change (for example, erasing `BmsonNoteExt` to
    /// `()`), while the `CustomEvent` type stays the same.
    ///
    /// Typical use: when a renderer consumes only core note fields, replace the
    /// per-note extension of `Chart<BmsonNoteExt, NoCustomEvent>` with `()`;
    /// the custom event type stays unchanged.
    ///
    /// To change the `CustomEvent` type or filter events at the same time, use
    /// [`filter_map_events`](Self::filter_map_events).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bmsrs_chart::{ChartData, Event, NoteExt, NoCustomEvent, EventKind};
    /// # fn example(data: ChartData<(), NoCustomEvent>) {
    /// let mapped: ChartData<(), NoCustomEvent> = data.map_events(|e| {
    ///     Event::new(e.tick(), e.kind.map_ext(|_| ()))
    /// });
    /// # }
    /// ```
    #[must_use]
    pub fn map_events<U: NoteExt>(
        self,
        mut f: impl FnMut(Event<T, C>) -> Event<U, C>,
    ) -> ChartData<U, C> {
        self.rebuild_events(|e| Some(f(e)))
    }

    /// Transform every event through a mapping-and-filter function, changing
    /// both the `T` and `C` generic parameters.
    ///
    /// Events returning [`None`] are dropped. All other fields are unchanged.
    ///
    /// Difference from [`map_events`](Self::map_events):
    /// - `map_events` changes only `T: NoteExt` and keeps `C: CustomEvent`;
    /// - `filter_map_events` can change both and supports dropping events.
    ///
    /// Typical use: convert `Chart<T, BmsCustomEvent>` into
    /// `Chart<U, NoCustomEvent>` while dropping every `Custom` event. After
    /// consuming `BmsCustomEvent`, the upper layer uses this method to erase the
    /// chart to `Chart<(), NoCustomEvent>` for renderers that only depend on the
    /// core IR.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bmsrs_chart::{
    /// #     ChartData, Event, EventKind, NoCustomEvent, NoteExt,
    /// #     CustomEvent,
    /// # };
    /// # fn example(data: ChartData<(), NoCustomEvent>) {
    /// // Drop Custom events and convert the C type of the rest.
    /// let mapped: ChartData<(), NoCustomEvent> = data.filter_map_events(|e| {
    ///     let tick = e.tick();
    ///     match e.kind {
    ///         EventKind::Custom(_) => None,
    ///         kind => Some(Event::new(tick, kind.map_custom(|_| NoCustomEvent))),
    ///     }
    /// });
    /// # }
    /// ```
    #[must_use]
    pub fn filter_map_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> ChartData<U, D> {
        self.rebuild_events(f)
    }

    /// Rebuild the event vector through a mapping function, keeping all other
    /// fields as-is.
    ///
    /// Shared implementation of [`map_events`](Self::map_events) and
    /// [`filter_map_events`](Self::filter_map_events); field destructuring is
    /// centralized here, so adding a field only requires updating this method.
    fn rebuild_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> ChartData<U, D> {
        let Self {
            timing,
            judge_multiplier,
            life_multiplier,
            ln_type_hint,
            ln_judge_hint,
            ln_life_hint,
            judge_deltas,
            life_deltas,
            events,
            audio_assets,
        } = self;
        ChartData {
            timing,
            judge_multiplier,
            life_multiplier,
            ln_type_hint,
            ln_judge_hint,
            ln_life_hint,
            judge_deltas,
            life_deltas,
            events: events.into_iter().filter_map(f).collect(),
            audio_assets,
        }
    }
}

/// Top-level chart, corresponding to BMSON v2's `Bmson` root object.
///
/// Contains song metadata, chart metadata, and gameplay data.
///
/// Does not implement [`Default`] because the embedded [`ChartData`] has no
/// valid default. Song-level and chart-level metadata ([`SongInfo`],
/// [`ChartInfo`]) still implement [`Default`].
#[derive(Clone, Debug, PartialEq)]
pub struct Chart<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Song-level metadata (title, artist, genre).
    pub song: SongInfo,
    /// Chart-level metadata and resources.
    pub chart: ChartInfo,
    /// Gameplay data (timing, events, audio).
    pub data: ChartData<T, C>,
}

impl<T: NoteExt + Eq, C: CustomEvent + Eq> Eq for Chart<T, C> {}

impl<T: NoteExt, C: CustomEvent> Chart<T, C> {
    /// Validate the key invariants of the chart.
    ///
    /// Checks:
    /// - every [`ChartData`] invariant (delegates to [`ChartData::validate`])
    /// - BGA events' `resource_id` falls within
    ///   [`bga_resources`](crate::ChartInfo::bga_resources)
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when any check fails.
    pub fn validate(&self) -> Result<(), ChartDataError> {
        self.data.validate()?;
        let len = self.chart.bga_resources.len();
        for event in &self.data.events {
            if let EventKind::Bga { resource_id, .. } = &event.kind
                && resource_id.get() as usize >= len
            {
                return Err(ChartDataError::DanglingBgaResource {
                    id: *resource_id,
                    len,
                });
            }
        }
        Ok(())
    }

    /// Transform every event through a mapping function and return a new
    /// `Chart` with all other fields unchanged.
    ///
    /// See [`ChartData::map_events`].
    #[must_use]
    pub fn map_events<U: NoteExt>(self, f: impl FnMut(Event<T, C>) -> Event<U, C>) -> Chart<U, C> {
        Chart {
            song: self.song,
            chart: self.chart,
            data: self.data.map_events(f),
        }
    }

    /// Transform every event through a mapping-and-filter function, changing
    /// both the `T` and `C` generic parameters.
    ///
    /// See [`ChartData::filter_map_events`].
    #[must_use]
    pub fn filter_map_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> Chart<U, D> {
        Chart {
            song: self.song,
            chart: self.chart,
            data: self.data.filter_map_events(f),
        }
    }
}
