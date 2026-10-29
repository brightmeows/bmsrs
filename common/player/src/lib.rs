//! Pure simulation layer for music game charts.
//!
//! [`Player<T, C>`] wraps a [`Chart<T, C>`] and provides a time-based query
//! interface for game rendering and scheduling. It is a pure simulation layer
//! with no I/O, rendering, judging, or scoring.
//!
//! # Usage
//!
//! ```
//! use std::num::NonZeroU8;
//! use std::time::Duration;
//! use bmsrs_chart::{
//!     Chart, SongInfo, ChartInfo, ChartData, Event, EventKind, Lane, LnJudgeHint, LnLifeHint,
//!     LnTypeHint, NoteKind, NoteSide, TimingTrack,
//! };
//! use bmsrs_player::Player;
//!
//! let chart: Chart = Chart {
//!     song: SongInfo::default(),
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
//!             480,
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
//!
//! let mut player = Player::new(chart).unwrap();
//! assert_eq!(player.current_tick(), 0);
//! player.advance(Duration::from_secs(1));
//! assert_eq!(player.current_time(), Duration::from_secs(1));
//! ```

pub(crate) mod scroll_cache;
mod speed_cache;

use std::ops::Bound;
use std::ops::RangeBounds;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BgaResource, Chart, ChartDataError, CustomEvent, Event, EventKind, NoCustomEvent,
    NoteExt, NoteKind,
};

use crate::scroll_cache::ScrollCache;
use crate::speed_cache::SpeedCache;

/// Stateful simulator that tracks playback progress through a [`Chart`].
///
/// The player keeps the current playback time (wall-clock time) and provides
/// queries for notes, BGM, and visual events in a given range. Tick positions
/// are derived from playback time on demand; all time advancement is expressed
/// as wall-clock [`Duration`].
///
/// Why time-driven: many instants inside a STOP segment map to the same tick, so
/// `tick` to `time` is no longer injective; using ticks as state would make
/// small per-frame `advance` steps get stuck inside a stop forever. With time as
/// state and ticks as a derived value, `advance` is pure accumulation, and any
/// step size can cross a stop segment.
///
/// The player is a pure simulation layer: no audio playback, rendering, input
/// handling, judging, or scoring.
pub struct Player<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Chart being played.
    chart: Chart<T, C>,
    /// Precomputed scroll speed cache for O(log n) queries.
    scroll_cache: ScrollCache,
    /// Precomputed SPEED spacing interpolation cache.
    speed_cache: SpeedCache,
    /// Current playback time (the only state field; tick positions are derived
    /// on demand).
    current_time: Duration,
}

impl<T: NoteExt, C: CustomEvent> Player<T, C> {
    /// Create a new player from a chart, starting at tick 0.
    ///
    /// Validates the chart data and sorts events internally; callers need no
    /// preprocessing.
    ///
    /// # Errors
    ///
    /// Returns [`ChartDataError`] when the chart is invalid (a `ChartData`
    /// invariant fails or a BGA resource reference is out of range).
    pub fn new(mut chart: Chart<T, C>) -> Result<Self, ChartDataError> {
        chart.data.sort_events();
        chart.validate()?;
        let resolution = chart.data.timing.resolution();
        let scroll_cache = ScrollCache::build(&chart.data.events, resolution);
        let speed_cache = SpeedCache::build(&chart.data.events);
        Ok(Self {
            chart,
            scroll_cache,
            speed_cache,
            current_time: Duration::ZERO,
        })
    }

    // Time control

    /// Advance playback by a wall-clock delta.
    ///
    /// Playback time accumulates `delta` directly (O(1); overflow clamps to
    /// [`Duration::MAX`]); tick positions are derived on demand in
    /// [`Self::current_tick`]. During a STOP, ticks do not advance but playback
    /// time keeps flowing. Small per-frame `advance` steps can cross a stop
    /// segment without needing a single step over the whole stop duration.
    pub const fn advance(&mut self, delta: Duration) {
        self.current_time = self.current_time.saturating_add(delta);
    }

    /// Seek to an absolute wall-clock time.
    pub const fn seek(&mut self, target: Duration) {
        self.current_time = target;
    }

    /// Reset playback to tick 0.
    pub const fn reset(&mut self) {
        self.current_time = Duration::ZERO;
    }

    // Time queries

    /// Current playback position in ticks.
    ///
    /// Derived from the current playback time (O(log n) binary search). This
    /// value stops during a stop while [`Self::current_time`] keeps flowing.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.chart.data.timing.duration_to_tick(self.current_time)
    }

    /// Current playback position as wall-clock [`Duration`].
    ///
    /// This value keeps flowing during a stop (decoupled from tick position); for
    /// "before the pause" semantics, convert [`Self::current_tick`] through
    /// [`Self::tick_to_duration`](Self::tick_to_duration).
    #[must_use]
    pub const fn current_time(&self) -> Duration {
        self.current_time
    }

    /// Convert a tick position into wall-clock [`Duration`].
    ///
    /// Delegates to
    /// [`TimingTrack::tick_to_duration`](bmsrs_chart::TimingTrack::tick_to_duration).
    /// Uses an O(log n) binary search internally.
    #[must_use]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
        self.chart.data.timing.tick_to_duration(tick)
    }

    /// Convert wall-clock [`Duration`] into the nearest tick position.
    ///
    /// Delegates to
    /// [`TimingTrack::duration_to_tick`](bmsrs_chart::TimingTrack::duration_to_tick).
    /// Uses an O(log n) binary search internally.
    #[must_use]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        self.chart.data.timing.duration_to_tick(duration)
    }

    /// Total chart duration.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.chart.data.duration()
    }

    /// Current BPM at the playback position.
    #[must_use]
    pub fn current_bpm(&self) -> f64 {
        self.chart.data.timing.bpm_at_tick(self.current_tick())
    }

    // Event queries

    /// Compute the tick range corresponding to a visible time window.
    ///
    /// `reaction` is the visible history duration below the judgment line, and
    /// `lookahead` is the preview duration above it. Returns
    /// `(start_tick, end_tick)`, which can be passed directly to
    /// [`events_in_range`](Self::events_in_range).
    ///
    /// Performs two
    /// [`TimingTrack::duration_to_tick`](bmsrs_chart::TimingTrack::duration_to_tick)
    /// calls (O(log n)), encapsulating the `current_time +/- duration` to tick
    /// conversion.
    #[must_use]
    pub fn visible_tick_range(&self, reaction: Duration, lookahead: Duration) -> (u64, u64) {
        let current = self.current_time();
        let start = current.checked_sub(reaction).unwrap_or(Duration::ZERO);
        let end = current.checked_add(lookahead).unwrap_or(Duration::MAX);
        let timing = &self.chart.data.timing;
        (timing.duration_to_tick(start), timing.duration_to_tick(end))
    }

    /// Return all events in `range`.
    ///
    /// Events are sorted ascending by tick (guaranteed by the chart).
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    #[must_use]
    pub fn events_in_range(&self, range: impl RangeBounds<u64>) -> &[Event<T, C>] {
        let (start, end) = self.event_range_indices(range);
        &self.chart.data.events[start..end]
    }

    /// Return an iterator over Note events in `range`.
    pub fn notes_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Note { .. }))
    }

    /// Return an iterator over judgment-relevant Note events in `range`.
    ///
    /// Judgment-relevant notes are normal notes and long notes (excluding
    /// invisible notes and mines, which are handled differently).
    pub fn notes_for_judgement(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.notes_in_range(range).filter(|e| {
            matches!(
                e.kind,
                EventKind::Note {
                    kind: NoteKind::Normal | NoteKind::Long { .. },
                    ..
                }
            )
        })
    }

    /// Return the audio asset table.
    #[must_use]
    pub fn audio_assets(&self) -> &[AudioAsset] {
        &self.chart.data.audio_assets
    }

    // Visual queries

    /// Return the scroll speed multiplier at `tick`.
    ///
    /// When several scroll events share a tick, the last one wins. When no
    /// scroll event has occurred yet, returns `1.0`.
    ///
    /// Uses the precomputed scroll speed cache; query complexity is O(log n).
    #[must_use]
    pub fn scroll_rate_at(&self, tick: u64) -> f64 {
        self.scroll_cache.rate_at(tick)
    }

    /// Return the cumulative scroll position at `tick`, in beats.
    ///
    /// Position is the integral of scroll speed over time. With no SCROLL events,
    /// position is `tick / resolution` (standard beat alignment).
    #[must_use]
    pub fn scroll_position_at(&self, tick: u64) -> f64 {
        self.scroll_cache.position_at(tick)
    }

    /// Return the interpolated SPEED spacing multiplier at `tick`.
    ///
    /// Performs linear interpolation between adjacent `EventKind::Speed`
    /// keyframes. Before the first keyframe it is `1.0`; after the last it is the
    /// last keyframe's value.
    #[must_use]
    pub fn spacing_at(&self, tick: u64) -> f64 {
        self.speed_cache.spacing_at(tick)
    }

    /// Return the BGA resources.
    #[must_use]
    pub fn bga_resources(&self) -> &[BgaResource] {
        &self.chart.chart.bga_resources
    }

    // Chart access

    /// Borrow the underlying chart.
    #[must_use]
    pub const fn chart(&self) -> &Chart<T, C> {
        &self.chart
    }

    /// Consume the player and return the chart.
    #[must_use]
    pub fn into_chart(self) -> Chart<T, C> {
        self.chart
    }

    // Private helpers

    /// Map [`RangeBounds<u64>`] to `(start index, end index)` in
    /// `self.chart.data.events`.
    fn event_range_indices(&self, range: impl RangeBounds<u64>) -> (usize, usize) {
        let start_tick = match range.start_bound() {
            Bound::Included(t) => *t,
            Bound::Excluded(t) => t.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end_tick = match range.end_bound() {
            Bound::Included(t) => t.saturating_add(1),
            Bound::Excluded(t) => *t,
            Bound::Unbounded => u64::MAX,
        };
        let start = self
            .chart
            .data
            .events
            .partition_point(|e| e.tick() < start_tick);
        let end = self
            .chart
            .data
            .events
            .partition_point(|e| e.tick() < end_tick);
        (start, end)
    }
}
