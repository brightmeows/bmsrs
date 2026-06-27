//! Pure simulation layer for rhythm game charts.
//!
//! [`Player<T, C>`] wraps a [`Chart<T, C>`] and provides time-based queries
//! for gameplay rendering and scheduling. It is a pure simulation layer with
//! no I/O, rendering, judgement, or scoring.
//!
//! # Usage
//!
//! ```
//! use std::num::NonZeroU8;
//! use std::time::Duration;
//! use bmsrs_chart::{
//!     Chart, ChartMetadata, Event, Lane, NoteKind, NoteSide, TimingTrack,
//! };
//! use bmsrs_player::Player;
//!
//! let chart: Chart = Chart {
//!     metadata: ChartMetadata::default(),
//!     resolution: 240,
//!     timing: TimingTrack {
//!         init_bpm: 120.0,
//!         bpm_changes: vec![],
//!         stops: vec![],
//!     },
//!     judge_multiplier: 1.0,
//!     life_multiplier: 1.0,
//!     events: vec![Event::Note {
//!         tick: 480,
//!         side: NoteSide::P1,
//!         lane: Lane::Key(NonZeroU8::new(1).unwrap()),
//!         kind: NoteKind::Normal,
//!         audio_index: None,
//!         ext: (),
//!     }],
//!     audio_assets: vec![],
//!     bga_resources: vec![],
//! };
//!
//! let mut player = Player::new(chart);
//! assert_eq!(player.current_tick(), 0);
//! player.advance(Duration::from_secs(1));
//! assert_eq!(player.current_time(), Duration::from_secs(1));
//! ```

mod timing;

use std::ops::Bound;
use std::ops::RangeBounds;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BgaResource, Chart, CustomEvent, Event, Lane, NoCustomEvent, NoteExt, NoteKind,
    NoteSide,
};

use crate::timing::TimingCache;

/// Stateful simulator that tracks playback position over a [`Chart`].
///
/// The player maintains a current tick position and provides queries for
/// notes, BGM, and visual events within ranges. All time advances are in
/// wall-clock [`Duration`]; tick positions are derived via an internal
/// pre-computed timing cache.
///
/// The player is a pure simulation layer: no audio playback, rendering,
/// input handling, judgement, or scoring.
pub struct Player<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// The chart being played.
    chart: Chart<T, C>,
    /// Pre-computed timing cache for O(log n) queries.
    cache: TimingCache,
    /// Current playback position in ticks.
    current_tick: u64,
}

impl<T: NoteExt, C: CustomEvent> Player<T, C> {
    /// Create a new player from a chart, starting at tick 0.
    #[must_use]
    pub fn new(chart: Chart<T, C>) -> Self {
        let resolution = chart.resolution;
        let cache = TimingCache::new(&chart.timing, resolution);
        Self {
            chart,
            cache,
            current_tick: 0,
        }
    }

    // ─── Time control ─────────────────────────────────────────────────

    /// Advance playback by `delta` of wall-clock time.
    ///
    /// The player's tick position is updated to the tick corresponding to
    /// `current_time + delta`. Time spent in stops does not advance the tick.
    pub fn advance(&mut self, delta: Duration) {
        let new_time = self.current_time() + delta;
        self.current_tick = self
            .chart
            .timing
            .duration_to_tick(new_time, self.chart.resolution);
    }

    /// Seek to an absolute wall-clock time.
    pub fn seek(&mut self, target: Duration) {
        self.current_tick = self
            .chart
            .timing
            .duration_to_tick(target, self.chart.resolution);
    }

    /// Reset playback to tick 0.
    pub const fn reset(&mut self) {
        self.current_tick = 0;
    }

    // ─── Time queries ─────────────────────────────────────────────────

    /// Current playback position in ticks.
    #[must_use]
    pub const fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Current playback position as wall-clock [`Duration`].
    #[must_use]
    pub fn current_time(&self) -> Duration {
        self.cache.tick_to_duration(self.current_tick)
    }

    /// Convert a tick position to wall-clock [`Duration`] using the cached
    /// timing data.
    ///
    /// This is faster than calling
    /// [`TimingTrack::tick_to_duration`](bmsrs_chart::TimingTrack::tick_to_duration)
    /// directly, using O(log n) binary search instead of O(n) iteration.
    #[must_use]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
        self.cache.tick_to_duration(tick)
    }

    /// Convert wall-clock [`Duration`] to the nearest tick position.
    ///
    /// Uses the pre-computed timing cache for O(log² n) performance.
    #[must_use]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        self.cache.duration_to_tick(duration)
    }

    /// Total chart duration.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.chart.duration()
    }

    /// Current BPM at the playback position.
    #[must_use]
    pub fn current_bpm(&self) -> f64 {
        self.cache.bpm_at_tick(self.current_tick)
    }

    // ─── Event queries ────────────────────────────────────────────────

    /// Return all events within `range`.
    ///
    /// Events are sorted by tick ascending (guaranteed by the chart).
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    #[must_use]
    pub fn events_in_range(&self, range: impl RangeBounds<u64>) -> &[Event<T, C>] {
        let (start, end) = self.event_range_indices(range);
        &self.chart.events[start..end]
    }

    /// Return an iterator over Note events within `range`.
    pub fn notes_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e, Event::Note { .. }))
    }

    /// Return an iterator over Note events at `(side, lane)` within `range`.
    pub fn notes_in_lane(
        &self,
        side: NoteSide,
        lane: Lane,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.notes_in_range(range).filter(
            move |e| matches!(e, Event::Note { side: s, lane: l, .. } if *s == side && *l == lane),
        )
    }

    /// Return an iterator over judgement-relevant Note events within
    /// `range`.
    ///
    /// Judgement-relevant notes are normal and long notes (not invisible
    /// or mines, which have different handling).
    pub fn notes_for_judgement(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.notes_in_range(range).filter(|e| {
            matches!(
                e,
                Event::Note {
                    kind: NoteKind::Normal | NoteKind::Long { .. },
                    ..
                }
            )
        })
    }

    /// Return an iterator over BGM events within `range`.
    pub fn bgm_in_range(&self, range: impl RangeBounds<u64>) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e, Event::Bgm { .. }))
    }

    /// Return the audio assets table.
    #[must_use]
    pub fn audio_assets(&self) -> &[AudioAsset] {
        &self.chart.audio_assets
    }

    // ─── Visual queries ───────────────────────────────────────────────

    /// Return the scroll-speed multiplier at `tick`.
    ///
    /// If multiple scroll events exist at the same tick, the last one wins.
    /// Returns `1.0` if no scroll event has occurred.
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    #[must_use]
    pub fn scroll_rate_at(&self, tick: u64) -> f64 {
        let end = self.chart.events.partition_point(|e| e.tick() <= tick);
        let mut rate = 1.0;
        for event in &self.chart.events[..end] {
            if let Event::Scroll { rate: sc_rate, .. } = event {
                rate = *sc_rate;
            }
        }
        rate
    }

    /// Return an iterator over Bar events within `range`.
    pub fn bar_lines_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e, Event::Bar { .. }))
    }

    /// Return an iterator over BGA events within `range`.
    ///
    /// The caller can further filter by [`bmsrs_chart::BgaLayer`] if needed.
    pub fn bga_events_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        self.events_in_range(range)
            .iter()
            .filter(|e| matches!(e, Event::Bga { .. }))
    }

    /// Return BGA resources.
    #[must_use]
    pub fn bga_resources(&self) -> &[BgaResource] {
        &self.chart.bga_resources
    }

    // ─── Chart access ─────────────────────────────────────────────────

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

    // ─── Private helpers ──────────────────────────────────────────────

    /// Map a [`RangeBounds<u64>`] to `(start_index, end_index)` into
    /// `self.chart.events`.
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
        let start = self.chart.events.partition_point(|e| e.tick() < start_tick);
        let end = self.chart.events.partition_point(|e| e.tick() < end_tick);
        (start, end)
    }
}
