//! Pure simulation layer for music game charts.
//!
//! [`Player<T, C>`] wraps a [`Chart<T, C>`] and derives its timing track, scroll
//! caches, and note index from the chart's event stream. It provides time,
//! event, scroll, and note queries for game rendering and scheduling, but does
//! not judge, render, or perform I/O.
//!
//! # Time model
//!
//! The player keeps wall-clock [`Duration`] as its only mutable state. Ticks are
//! derived from time through [`TimingTrack`], which is built from
//! [`bmsrs_chart::EventKind::Bpm`] and [`bmsrs_chart::EventKind::Stop`] events.
//! During a STOP, time keeps flowing while ticks stay frozen.
//!
//! # Judgment points
//!
//! [`NoteTable`] assigns stable [`NoteId`]s and keeps descriptive
//! [`NotePoint`]s. It does not decide whether a note is judged: `Normal +
//! Release` is a release keysound/BSS marker, `Long + Release` is an MSS
//! endpoint that the caller resolves with its Ln/Cn/Hcn policy.
//!
//! # Usage
//!
//! ```
//! use std::num::NonZeroU8;
//! use std::time::Duration;
//! use bmsrs_chart::{
//!     Chart, SongInfo, ChartInfo, ChartData, Event, Lane, Note, NoteKind, NoteSide,
//! };
//! use bmsrs_player::Player;
//!
//! let data: ChartData = ChartData::new(
//!     120.0,
//!     240,
//!     vec![Event::note(
//!         480,
//!         Note::<()>::new(
//!             NoteSide::P1,
//!             Lane::Key(NonZeroU8::new(1).unwrap()),
//!             NoteKind::Normal,
//!             bmsrs_chart::NoteRole::Normal,
//!             bmsrs_chart::NoteAudio::none(),
//!         ),
//!     )],
//!     vec![],
//! )
//! .unwrap();
//! let chart = Chart::new(SongInfo::default(), ChartInfo::default(), data).unwrap();
//!
//! let mut player = Player::new(chart);
//! assert_eq!(player.current_tick(), 0);
//! player.advance(Duration::from_secs(1));
//! assert_eq!(player.current_time(), Duration::from_secs(1));
//! assert_eq!(player.current_tick(), 480);
//! ```

pub mod note_table;
pub mod viewport;

mod scroll_cache;
mod speed_cache;

use std::ops::{Bound, Range, RangeBounds};
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BgaResource, Chart, CustomEvent, Event, NoCustomEvent, NoteExt, TimingTrack,
};

use crate::scroll_cache::ScrollCache;
use crate::speed_cache::SpeedCache;

pub use crate::note_table::{LongNotePair, NoteId, NotePoint, NoteRef, NoteTable};
pub use crate::viewport::{Viewport, VisibleLine, VisibleLongNote, VisibleNote};

/// Stateful simulator that tracks playback progress through a [`Chart`].
///
/// The player keeps the current playback time and derives timing, scroll, and
/// note indexes from the chart's event stream. It does not judge, render, or
/// schedule audio.
pub struct Player<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Chart being played.
    chart: Chart<T, C>,
    /// Timing track derived from the chart's event stream.
    timing: TimingTrack,
    /// Precomputed scroll speed cache.
    scroll_cache: ScrollCache,
    /// Precomputed SPEED spacing cache.
    speed_cache: SpeedCache,
    /// Stable note index.
    note_table: NoteTable,
    /// Current playback time (the only mutable state).
    current_time: Duration,
}

impl<T: NoteExt, C: CustomEvent> Player<T, C> {
    /// Create a player from an already validated chart.
    ///
    /// [`Chart`] guarantees sorted events, valid timing values, and resolved
    /// references, so this constructor derives the timing track, scroll caches,
    /// and note table without revalidating.
    ///
    /// # Panics
    ///
    /// Panics only if the [`Chart`] invariant is violated. Public constructors
    /// make that unreachable.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "Chart guarantees a valid initial BPM and resolution"
    )]
    pub fn new(chart: Chart<T, C>) -> Self {
        let timing = TimingTrack::from_events(
            chart.data().init_bpm(),
            chart.data().resolution(),
            chart.data().events(),
        )
        .expect("validated chart has valid timing");
        let scroll_cache = ScrollCache::build(chart.data().events(), timing.resolution());
        let speed_cache = SpeedCache::build(chart.data().events(), &timing);
        let note_table = NoteTable::build(chart.data().events());
        Self {
            chart,
            timing,
            scroll_cache,
            speed_cache,
            note_table,
            current_time: Duration::ZERO,
        }
    }

    // Time control

    /// Advance playback by a wall-clock delta.
    ///
    /// Playback time accumulates `delta` directly (O(1); overflow clamps to
    /// [`Duration::MAX`]). Tick positions are derived on demand.
    pub const fn advance(&mut self, delta: Duration) {
        self.current_time = self.current_time.saturating_add(delta);
    }

    /// Seek to an absolute wall-clock time.
    pub const fn seek(&mut self, target: Duration) {
        self.current_time = target;
    }

    /// Reset playback to time zero.
    pub const fn reset(&mut self) {
        self.current_time = Duration::ZERO;
    }

    // Time queries

    /// Current playback position in ticks, rounded to the nearest tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.timing.duration_to_tick(self.current_time)
    }

    /// Current playback position in continuous ticks.
    ///
    /// This is the value renderers should use for smooth scroll positions; it
    /// freezes during a STOP.
    #[must_use]
    pub fn current_tick_f64(&self) -> f64 {
        self.timing.duration_to_tick_f64(self.current_time)
    }

    /// Current playback position as wall-clock [`Duration`].
    ///
    /// This value keeps flowing during a STOP.
    #[must_use]
    pub const fn current_time(&self) -> Duration {
        self.current_time
    }

    /// Convert a tick position into wall-clock [`Duration`].
    #[must_use]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
        self.timing.tick_to_duration(tick)
    }

    /// Convert wall-clock [`Duration`] into the nearest tick position.
    #[must_use]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        self.timing.duration_to_tick(duration)
    }

    /// Convert wall-clock [`Duration`] into a continuous tick position.
    #[must_use]
    pub fn duration_to_tick_f64(&self, duration: Duration) -> f64 {
        self.timing.duration_to_tick_f64(duration)
    }

    /// Alias for [`duration_to_tick_f64`](Self::duration_to_tick_f64).
    #[must_use]
    pub fn time_to_tick_f64(&self, time: Duration) -> f64 {
        self.duration_to_tick_f64(time)
    }

    /// Total chart duration at the last event tick.
    ///
    /// This is the render duration of the last event, not a tail after it.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.timing.tick_to_duration(self.chart.data().last_tick())
    }

    /// BPM in effect at the current playback position.
    #[must_use]
    pub fn current_bpm(&self) -> f64 {
        self.timing.bpm_at_tick(self.current_tick())
    }

    /// Borrow the derived timing track.
    #[must_use]
    pub const fn timing(&self) -> &TimingTrack {
        &self.timing
    }

    // Event queries

    /// Return all events in `range`, sorted ascending by tick.
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on the same vector"
    )]
    #[must_use]
    pub fn events_in_range(&self, range: impl RangeBounds<u64>) -> &[Event<T, C>] {
        let (start, end) = self.event_range_indices(range);
        &self.chart.data().events()[start..end]
    }

    /// Return the full sorted event stream.
    #[must_use]
    pub fn events(&self) -> &[Event<T, C>] {
        self.chart.data().events()
    }

    /// Return events whose activation time falls in `[start, end)`.
    ///
    /// The tick range is a coarse filter; the exact time check uses
    /// [`tick_to_duration`](Self::tick_to_duration), so STOP boundaries are
    /// handled correctly.
    pub fn events_in_time_range(
        &self,
        range: Range<Duration>,
    ) -> impl Iterator<Item = &Event<T, C>> {
        let (start, end) = (range.start, range.end);
        let start_tick = floor_tick(self.duration_to_tick_f64(start));
        let end_tick = ceil_tick(self.duration_to_tick_f64(end));
        self.events_in_range(start_tick..=end_tick)
            .iter()
            .filter(move |event| {
                let trigger = self.tick_to_duration(event.tick());
                trigger >= start && trigger < end
            })
    }

    /// Return the first event strictly after `tick`.
    #[must_use]
    pub fn next_event_after(&self, tick: u64) -> Option<&Event<T, C>> {
        self.events_in_range(tick.saturating_add(1)..).first()
    }

    /// Return note refs in `range`.
    pub fn notes_in_range(
        &self,
        range: impl RangeBounds<u64>,
    ) -> impl Iterator<Item = NoteRef<'_, T, C>> {
        let events = self.chart.data().events();
        self.note_table
            .notes_in_range(range)
            .map(move |point| NoteTable::note_ref(point, events))
    }

    /// Return a note ref by stable id.
    #[must_use]
    pub fn note(&self, id: NoteId) -> Option<NoteRef<'_, T, C>> {
        let point = self.note_table.note(id)?;
        Some(NoteTable::note_ref(point, self.chart.data().events()))
    }

    /// Return every long-note pair.
    #[must_use]
    pub fn long_notes(&self) -> &[LongNotePair] {
        self.note_table.long_notes()
    }

    /// Return the audio asset table.
    #[must_use]
    pub fn audio_assets(&self) -> &[AudioAsset] {
        self.chart.data().audio_assets()
    }

    // Visual queries

    /// Return the scroll speed multiplier at `tick`.
    #[must_use]
    pub fn scroll_rate_at(&self, tick: u64) -> f64 {
        self.scroll_cache.rate_at(tick)
    }

    /// Return the cumulative scroll position at `tick`, in beats.
    #[must_use]
    pub fn scroll_position_at(&self, tick: u64) -> f64 {
        self.scroll_cache.position_at(tick)
    }

    /// Return the cumulative scroll position at a continuous tick position.
    #[must_use]
    pub fn scroll_position_at_f64(&self, tick: f64) -> f64 {
        self.scroll_cache.position_at_f64(tick)
    }

    /// Return the scroll distance from `from_tick` to `to_tick`, in beats.
    #[must_use]
    pub fn scroll_delta(&self, from_tick: f64, to_tick: f64) -> f64 {
        self.scroll_position_at_f64(to_tick) - self.scroll_position_at_f64(from_tick)
    }

    /// Return the interpolated SPEED spacing multiplier at `time`.
    #[must_use]
    pub fn spacing_at_time(&self, time: Duration) -> f64 {
        self.speed_cache.spacing_at_time(time)
    }

    /// Return the BGA resources.
    #[must_use]
    pub fn bga_resources(&self) -> &[BgaResource] {
        &self.chart.chart_info().bga_resources
    }

    /// Borrow the note table.
    #[must_use]
    pub const fn note_table(&self) -> &NoteTable {
        &self.note_table
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

    /// Map a tick range to `(start, end)` indices in the event vector.
    fn event_range_indices(&self, range: impl RangeBounds<u64>) -> (usize, usize) {
        let events = self.chart.data().events();
        let start = match range.start_bound() {
            Bound::Included(tick) => events.partition_point(|event| event.tick() < *tick),
            Bound::Excluded(tick) => events.partition_point(|event| event.tick() <= *tick),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(tick) => events.partition_point(|event| event.tick() <= *tick),
            Bound::Excluded(tick) => events.partition_point(|event| event.tick() < *tick),
            Bound::Unbounded => events.len(),
        };
        (start, end.max(start))
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped non-negative time converts to a practical tick index"
)]
/// Round a continuous tick value down to a non-negative tick index.
const fn floor_tick(value: f64) -> u64 {
    value.max(0.0).floor() as u64
}

/// Round a continuous tick value up to a non-negative tick index.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped non-negative time converts to a practical tick index"
)]
const fn ceil_tick(value: f64) -> u64 {
    value.max(0.0).ceil() as u64
}
