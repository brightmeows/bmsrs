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
//! All event positions are absolute [`u64`] ticks.
//! [`ChartData::resolution`] defines ticks per quarter note (default 240).
//! BPM changes and stops live in the event stream; `bmsrs-player` derives a
//! [`TimingTrack`] from them. Wall-clock time is obtained through
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
//! callers can use [`ChartData::try_filter_map_events`] to erase the chart to
//! `Chart<(), NoCustomEvent>` for renderers that only depend on the core IR. To
//! replace only the per-note extension while keeping custom events, use
//! [`ChartData::try_map_events`].
//!
//! # Example
//!
//! ```
//! use std::num::NonZeroU8;
//! use bmsrs_chart::{
//!     Chart, ChartData, ChartInfo, Event, EventKind, Lane, NoteKind, NoteSide, SongInfo,
//! };
//!
//! let data: ChartData = ChartData::new(
//!     120.0,
//!     240,
//!     vec![Event::new(
//!         0,
//!         EventKind::Note {
//!             side: NoteSide::P1,
//!             lane: Lane::Key(NonZeroU8::new(1).unwrap()),
//!             kind: NoteKind::Normal,
//!             role: bmsrs_chart::NoteRole::Normal,
//!             audio: bmsrs_chart::NoteAudio::none(),
//!             ext: (),
//!         },
//!     )],
//!     vec![],
//! )
//! .unwrap();
//!
//! let chart: Chart = Chart::new(
//!     SongInfo {
//!         title: "Test".into(),
//!         ..Default::default()
//!     },
//!     ChartInfo::default(),
//!     data,
//! )
//! .unwrap();
//! assert_eq!(chart.song().title, "Test");
//! assert_eq!(chart.data().events().len(), 1);
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
#[derive(Clone, Debug, Default, PartialEq)]
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

/// `ChartData` validation error type.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ChartDataError {
    /// The initial BPM or beat resolution is invalid.
    Timing(TimingTrackError),
    /// A BPM event carries a non-positive or non-finite value.
    InvalidBpmEvent {
        /// Tick where the invalid BPM event occurs.
        tick: u64,
        /// Invalid BPM value.
        bpm: f64,
    },
    /// A SCROLL or SPEED event carries a non-finite rate.
    InvalidVisualRate {
        /// Tick where the invalid event occurs.
        tick: u64,
        /// Invalid rate value.
        rate: f64,
    },
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
            Self::Timing(error) => error.fmt(f),
            Self::InvalidBpmEvent { tick, bpm } => {
                write!(
                    f,
                    "invalid BPM event at tick {tick}: {bpm} (must be positive finite)"
                )
            }
            Self::InvalidVisualRate { tick, rate } => {
                write!(
                    f,
                    "invalid visual rate at tick {tick}: {rate} (must be finite)"
                )
            }
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

impl From<TimingTrackError> for ChartDataError {
    fn from(error: TimingTrackError) -> Self {
        Self::Timing(error)
    }
}

/// Gameplay data, corresponding to BMSON v2's `ChartData`.
///
/// Does not implement [`Default`]: a valid state requires a positive initial BPM
/// and a positive `resolution`, so there is no meaningful zero default. BPM
/// changes and stops live in `events`; the player derives a
/// [`TimingTrack`] from them.
///
/// Fields are private. A value is always produced by [`ChartData::new`] or a
/// fallible event transformation, so the event stream is sorted and every audio
/// reference resolves.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartData<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// BPM in effect at tick 0.
    init_bpm: f64,
    /// Ticks per quarter note (beat resolution).
    resolution: u64,
    /// Judgment window multiplier (`Some(1.0)` is standard); `None` means the
    /// source did not specify it.
    judge_multiplier: Option<f64>,
    /// Life gauge multiplier (`Some(1.0)` is standard); `None` means the source
    /// did not specify it.
    life_multiplier: Option<f64>,
    /// Chart-level long-note type hint (BMSON LN/CN).
    ln_type_hint: LnTypeHint,
    /// Chart-level long-note judgment hint.
    ln_judge_hint: LnJudgeHint,
    /// Chart-level long-note life hint.
    ln_life_hint: LnLifeHint,
    /// Custom judgment-window offsets (DJ.NEXT extension; `None` for BMS).
    judge_deltas: Option<JudgementDeltas>,
    /// Custom life gauge deltas (DJ.NEXT extension; `None` for BMS).
    life_deltas: Option<LifeDeltas>,
    /// All timed events, sorted ascending by tick.
    events: Vec<Event<T, C>>,
    /// Audio assets referenced by note and BGM events.
    audio_assets: Vec<AudioAsset>,
}

impl<T: NoteExt, C: CustomEvent> ChartData<T, C> {
    /// Construct chart data, sort events, and validate the result.
    ///
    /// `init_bpm` and `resolution` are the scalar timing configuration; BPM
    /// changes and stops live only in `events`. The player derives its
    /// [`TimingTrack`] from these events, so the event stream is the single
    /// source of truth for timing changes.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when the initial BPM or resolution is invalid,
    /// an event value is invalid, or an audio reference is out of range.
    pub fn new(
        init_bpm: f64,
        resolution: u64,
        mut events: Vec<Event<T, C>>,
        audio_assets: Vec<AudioAsset>,
    ) -> Result<Self, ChartDataError> {
        events.sort_by_key(Event::sort_key);
        let data = Self {
            init_bpm,
            resolution,
            judge_multiplier: None,
            life_multiplier: None,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events,
            audio_assets,
        };
        data.validate()?;
        Ok(data)
    }

    /// Return the initial BPM.
    #[must_use]
    pub const fn init_bpm(&self) -> f64 {
        self.init_bpm
    }

    /// Return the ticks-per-quarter-note resolution.
    #[must_use]
    pub const fn resolution(&self) -> u64 {
        self.resolution
    }

    /// Return the judgment window multiplier, if the source specified one.
    #[must_use]
    pub const fn judge_multiplier(&self) -> Option<f64> {
        self.judge_multiplier
    }

    /// Return the life gauge multiplier, if the source specified one.
    #[must_use]
    pub const fn life_multiplier(&self) -> Option<f64> {
        self.life_multiplier
    }

    /// Return the chart-level long-note type hint.
    #[must_use]
    pub const fn ln_type_hint(&self) -> LnTypeHint {
        self.ln_type_hint
    }

    /// Return the chart-level long-note judgment hint.
    #[must_use]
    pub const fn ln_judge_hint(&self) -> LnJudgeHint {
        self.ln_judge_hint
    }

    /// Return the chart-level long-note life hint.
    #[must_use]
    pub const fn ln_life_hint(&self) -> LnLifeHint {
        self.ln_life_hint
    }

    /// Return the custom judgment-window offsets, if present.
    #[must_use]
    pub const fn judge_deltas(&self) -> Option<JudgementDeltas> {
        self.judge_deltas
    }

    /// Return the custom life gauge deltas, if present.
    #[must_use]
    pub const fn life_deltas(&self) -> Option<LifeDeltas> {
        self.life_deltas
    }

    /// Return all timed events, sorted ascending by tick.
    #[must_use]
    pub fn events(&self) -> &[Event<T, C>] {
        &self.events
    }

    /// Return the audio asset table.
    #[must_use]
    pub fn audio_assets(&self) -> &[AudioAsset] {
        &self.audio_assets
    }

    /// Set the judgment window multiplier.
    #[must_use]
    pub const fn with_judge_multiplier(mut self, value: Option<f64>) -> Self {
        self.judge_multiplier = value;
        self
    }

    /// Set the life gauge multiplier.
    #[must_use]
    pub const fn with_life_multiplier(mut self, value: Option<f64>) -> Self {
        self.life_multiplier = value;
        self
    }

    /// Set the chart-level long-note type hint.
    #[must_use]
    pub const fn with_ln_type_hint(mut self, value: LnTypeHint) -> Self {
        self.ln_type_hint = value;
        self
    }

    /// Set the chart-level long-note judgment hint.
    #[must_use]
    pub const fn with_ln_judge_hint(mut self, value: LnJudgeHint) -> Self {
        self.ln_judge_hint = value;
        self
    }

    /// Set the chart-level long-note life hint.
    #[must_use]
    pub const fn with_ln_life_hint(mut self, value: LnLifeHint) -> Self {
        self.ln_life_hint = value;
        self
    }

    /// Set the custom judgment-window offsets.
    #[must_use]
    pub const fn with_judge_deltas(mut self, value: Option<JudgementDeltas>) -> Self {
        self.judge_deltas = value;
        self
    }

    /// Set the custom life gauge deltas.
    #[must_use]
    pub const fn with_life_deltas(mut self, value: Option<LifeDeltas>) -> Self {
        self.life_deltas = value;
        self
    }

    /// Return the tick of the last event in the chart data.
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        self.events.last().map_or(0, Event::tick)
    }

    /// Validate the key invariants of the chart data.
    ///
    /// Checks:
    /// - `init_bpm` is positive and finite, and `resolution` is positive
    /// - events are sorted ascending by [`Event::sort_key`]
    /// - BPM events are positive and finite; SCROLL/SPEED rates are finite
    /// - reference integrity: audio indices of `Note`/`Bgm` events fall within
    ///   the audio asset table
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when any check fails.
    fn validate(&self) -> Result<(), ChartDataError> {
        if !BpmChange::is_valid(self.init_bpm) {
            return Err(TimingTrackError::InvalidBpm { bpm: self.init_bpm }.into());
        }
        if self.resolution == 0 {
            return Err(TimingTrackError::ZeroResolution.into());
        }

        if !self.events.is_sorted_by_key(Event::sort_key) {
            return Err(ChartDataError::UnsortedEvents);
        }

        let len = self.audio_assets.len();
        for event in &self.events {
            match &event.kind {
                EventKind::Bpm { bpm } if !BpmChange::is_valid(*bpm) => {
                    return Err(ChartDataError::InvalidBpmEvent {
                        tick: event.tick(),
                        bpm: *bpm,
                    });
                }
                EventKind::Scroll { rate } | EventKind::Speed { rate } if !rate.is_finite() => {
                    return Err(ChartDataError::InvalidVisualRate {
                        tick: event.tick(),
                        rate: *rate,
                    });
                }
                EventKind::Note { audio, .. } => {
                    for &audio_index in audio.as_slice() {
                        if audio_index.get() as usize >= len {
                            return Err(ChartDataError::DanglingAudioIndex {
                                index: audio_index,
                                len,
                            });
                        }
                    }
                }
                EventKind::Bgm { audio_index } if audio_index.get() as usize >= len => {
                    return Err(ChartDataError::DanglingAudioIndex {
                        index: *audio_index,
                        len,
                    });
                }
                _ => {}
            }
        }
        Ok(())
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
    /// [`try_filter_map_events`](Self::try_filter_map_events).
    ///
    /// The result is re-sorted and re-validated. Returns [`ChartDataError`] when
    /// the mapped events violate a chart invariant.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when the mapped events violate a chart
    /// invariant.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bmsrs_chart::{ChartData, Event, NoteExt, NoCustomEvent, EventKind};
    /// # fn example(data: ChartData<(), NoCustomEvent>) {
    /// let mapped: ChartData<(), NoCustomEvent> = data
    ///     .try_map_events(|e| Event::new(e.tick(), e.kind.map_ext(|_| ())))
    ///     .unwrap();
    /// # }
    /// ```
    pub fn try_map_events<U: NoteExt>(
        self,
        mut f: impl FnMut(Event<T, C>) -> Event<U, C>,
    ) -> Result<ChartData<U, C>, ChartDataError> {
        self.rebuild_events(|e| Some(f(e)))
    }

    /// Transform every event through a mapping-and-filter function, changing
    /// both the `T` and `C` generic parameters.
    ///
    /// Events returning [`None`] are dropped. All other fields are unchanged.
    ///
    /// Difference from [`try_map_events`](Self::try_map_events):
    /// - `try_map_events` changes only `T: NoteExt` and keeps `C: CustomEvent`;
    /// - `try_filter_map_events` can change both and supports dropping events.
    ///
    /// Typical use: convert `Chart<T, BmsCustomEvent>` into
    /// `Chart<U, NoCustomEvent>` while dropping every `Custom` event. After
    /// consuming `BmsCustomEvent`, the upper layer uses this method to erase the
    /// chart to `Chart<(), NoCustomEvent>` for renderers that only depend on the
    /// core IR.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when the mapped events violate a chart
    /// invariant.
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
    /// let mapped: ChartData<(), NoCustomEvent> = data
    ///     .try_filter_map_events(|e| {
    ///         let tick = e.tick();
    ///         match e.kind {
    ///             EventKind::Custom(_) => None,
    ///             kind => Some(Event::new(tick, kind.map_custom(|_| NoCustomEvent))),
    ///         }
    ///     })
    ///     .unwrap();
    /// # }
    /// ```
    pub fn try_filter_map_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> Result<ChartData<U, D>, ChartDataError> {
        self.rebuild_events(f)
    }

    /// Rebuild the event vector through a mapping function, sort it, and
    /// validate the result.
    ///
    /// Shared implementation of [`try_map_events`](Self::try_map_events) and
    /// [`try_filter_map_events`](Self::try_filter_map_events); field
    /// destructuring is centralized here, so adding a field only requires
    /// updating this method.
    fn rebuild_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> Result<ChartData<U, D>, ChartDataError> {
        let Self {
            init_bpm,
            resolution,
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
        let mut mapped_events: Vec<_> = events.into_iter().filter_map(f).collect();
        mapped_events.sort_by_key(Event::sort_key);
        let data = ChartData {
            init_bpm,
            resolution,
            judge_multiplier,
            life_multiplier,
            ln_type_hint,
            ln_judge_hint,
            ln_life_hint,
            judge_deltas,
            life_deltas,
            events: mapped_events,
            audio_assets,
        };
        data.validate()?;
        Ok(data)
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
#[expect(
    clippy::struct_field_names,
    reason = "the chart-level metadata field mirrors BMSON's chart_info naming"
)]
pub struct Chart<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Song-level metadata (title, artist, genre).
    song: SongInfo,
    /// Chart-level metadata and resources.
    chart: ChartInfo,
    /// Gameplay data (timing configuration, events, audio).
    data: ChartData<T, C>,
}

impl<T: NoteExt, C: CustomEvent> Chart<T, C> {
    /// Construct a chart and validate its BGA references.
    ///
    /// The embedded [`ChartData`] is already valid. This constructor additionally
    /// checks that every BGA event's `resource_id` falls within
    /// [`bga_resources`](crate::ChartInfo::bga_resources).
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when a BGA event references an out-of-range
    /// resource.
    pub fn new(
        song: SongInfo,
        chart: ChartInfo,
        data: ChartData<T, C>,
    ) -> Result<Self, ChartDataError> {
        let validated = Self { song, chart, data };
        validated.validate_bga()?;
        Ok(validated)
    }

    /// Return the song-level metadata.
    #[must_use]
    pub const fn song(&self) -> &SongInfo {
        &self.song
    }

    /// Return the chart-level metadata and resources.
    #[must_use]
    pub const fn chart_info(&self) -> &ChartInfo {
        &self.chart
    }

    /// Return the gameplay data.
    #[must_use]
    pub const fn data(&self) -> &ChartData<T, C> {
        &self.data
    }

    /// Validate BGA resource references.
    ///
    /// [`ChartData`] validates every other invariant at construction, so this
    /// method only checks the cross-structure BGA reference.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError::DanglingBgaResource`] when an event references
    /// an out-of-range resource.
    fn validate_bga(&self) -> Result<(), ChartDataError> {
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
    /// See [`ChartData::try_map_events`]. BGA references are revalidated.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when the mapped events violate a chart
    /// invariant.
    pub fn try_map_events<U: NoteExt>(
        self,
        f: impl FnMut(Event<T, C>) -> Event<U, C>,
    ) -> Result<Chart<U, C>, ChartDataError> {
        let Self { song, chart, data } = self;
        Chart::<U, C>::new(song, chart, data.try_map_events(f)?)
    }

    /// Transform every event through a mapping-and-filter function, changing
    /// both the `T` and `C` generic parameters.
    ///
    /// See [`ChartData::try_filter_map_events`]. BGA references are revalidated.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when the mapped events violate a chart
    /// invariant.
    pub fn try_filter_map_events<U: NoteExt, D: CustomEvent>(
        self,
        f: impl FnMut(Event<T, C>) -> Option<Event<U, D>>,
    ) -> Result<Chart<U, D>, ChartDataError> {
        let Self { song, chart, data } = self;
        Chart::<U, D>::new(song, chart, data.try_filter_map_events(f)?)
    }
}
