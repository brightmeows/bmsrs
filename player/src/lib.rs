//! Pure simulation layer for rhythm game charts.
//!
//! [`Player<T>`] wraps a [`Chart<T>`] and provides time-based queries for
//! gameplay rendering and scheduling. It is a pure simulation layer with
//! no I/O, rendering, judgement, or scoring.
//!
//! # Usage
//!
//! ```
//! use std::time::Duration;
//! use bmsrs_chart::{
//!     Chart, ChartMetadata, DefaultNoteData, Note, NoteKind, TimingTrack, Bga,
//! };
//! use bmsrs_player::Player;
//!
//! let chart = Chart {
//!     metadata: ChartMetadata::default(),
//!     resolution: 240,
//!     lane_count: 8,
//!     timing: TimingTrack {
//!         init_bpm: 120.0,
//!         bpm_changes: vec![],
//!         stops: vec![],
//!     },
//!     judge_multiplier: 1.0,
//!     life_multiplier: 1.0,
//!     notes: vec![Note {
//!         tick: 480,
//!         audio: None,
//!         data: DefaultNoteData {
//!             lane: 0,
//!             kind: NoteKind::Normal,
//!         },
//!     }],
//!     bgm: vec![],
//!     audio_assets: vec![],
//!     bar_lines: vec![],
//!     scroll_events: vec![],
//!     bga: Bga::default(),
//! };
//!
//! let mut player = Player::new(chart);
//! assert_eq!(player.current_tick(), 0);
//! player.advance(Duration::from_secs(1));
//! assert_eq!(player.current_time(), Duration::from_secs(1));
//! ```

mod timing;

use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BarLine, BgaTimelineEvent, BgmEvent, Chart, Note, NoteData, NoteKind,
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
pub struct Player<T: NoteData> {
    /// The chart being played.
    chart: Chart<T>,
    /// Pre-computed timing cache for O(log n) queries.
    cache: TimingCache,
    /// Current playback position in ticks.
    current_tick: u64,
}

impl<T: NoteData> Player<T> {
    /// Create a new player from a chart, starting at tick 0.
    #[must_use]
    pub fn new(chart: Chart<T>) -> Self {
        let resolution = chart.resolution;
        let cache = TimingCache::new(&chart.timing, resolution);
        Self {
            chart,
            cache,
            current_tick: 0,
        }
    }

    // Time control

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
    pub fn reset(&mut self) {
        self.current_tick = 0;
    }

    // Time queries

    /// Current playback position in ticks.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
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
    /// Delegates to
    /// [`TimingTrack::duration_to_tick`](bmsrs_chart::TimingTrack::duration_to_tick).
    #[must_use]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        self.chart
            .timing
            .duration_to_tick(duration, self.chart.resolution)
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

    // Note queries

    /// Return notes within `[from_tick, to_tick)`.
    ///
    /// Notes are sorted by tick ascending (guaranteed by the chart).
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    #[must_use]
    pub fn notes_in_range(&self, from_tick: u64, to_tick: u64) -> &[Note<T>] {
        let start = self.chart.notes.partition_point(|n| n.tick < from_tick);
        let end = self.chart.notes.partition_point(|n| n.tick < to_tick);
        &self.chart.notes[start..end]
    }

    /// Return an iterator over notes in `lane` within `[from_tick, to_tick)`.
    pub fn notes_in_lane(
        &self,
        lane: u16,
        from_tick: u64,
        to_tick: u64,
    ) -> impl Iterator<Item = &Note<T>> {
        self.notes_in_range(from_tick, to_tick)
            .iter()
            .filter(move |n| n.data.lane() == lane)
    }

    /// Return an iterator over judgement-relevant notes in
    /// `[from_tick, to_tick)`.
    ///
    /// Judgement-relevant notes are normal and long notes (not invisible
    /// or mines, which have different handling).
    pub fn notes_for_judgement(
        &self,
        from_tick: u64,
        to_tick: u64,
    ) -> impl Iterator<Item = &Note<T>> {
        self.notes_in_range(from_tick, to_tick)
            .iter()
            .filter(|n| matches!(n.data.kind(), NoteKind::Normal | NoteKind::Long { .. }))
    }

    // Audio queries

    /// Return BGM events within `[from_tick, to_tick)`.
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    #[must_use]
    pub fn bgm_in_range(&self, from_tick: u64, to_tick: u64) -> &[BgmEvent] {
        let start = self.chart.bgm.partition_point(|e| e.tick < from_tick);
        let end = self.chart.bgm.partition_point(|e| e.tick < to_tick);
        &self.chart.bgm[start..end]
    }

    /// Return the audio assets table.
    #[must_use]
    pub fn audio_assets(&self) -> &[AudioAsset] {
        &self.chart.audio_assets
    }

    // Visual queries

    /// Return the scroll-speed multiplier at `tick`.
    ///
    /// If multiple scroll events exist at the same tick, the last one wins.
    /// Returns `1.0` if no scroll event has occurred.
    #[must_use]
    pub fn scroll_rate_at(&self, tick: u64) -> f64 {
        let mut rate = 1.0;
        for sc in &self.chart.scroll_events {
            if sc.tick <= tick {
                rate = sc.rate;
            } else {
                break;
            }
        }
        rate
    }

    /// Return bar lines within `[from_tick, to_tick)`.
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    #[must_use]
    pub fn bar_lines_in_range(&self, from_tick: u64, to_tick: u64) -> &[BarLine] {
        let start = self.chart.bar_lines.partition_point(|b| b.tick < from_tick);
        let end = self.chart.bar_lines.partition_point(|b| b.tick < to_tick);
        &self.chart.bar_lines[start..end]
    }

    /// Return BGA events on the base layer within `[from_tick, to_tick)`.
    #[must_use]
    pub fn bga_events_in_range(&self, from_tick: u64, to_tick: u64) -> &[BgaTimelineEvent] {
        Self::events_in_range(&self.chart.bga.events, from_tick, to_tick)
    }

    /// Return BGA events on the overlay layer within `[from_tick, to_tick)`.
    #[must_use]
    pub fn bga_layer_events_in_range(&self, from_tick: u64, to_tick: u64) -> &[BgaTimelineEvent] {
        Self::events_in_range(&self.chart.bga.layer_events, from_tick, to_tick)
    }

    /// Return BGA events on the poor (miss) layer within `[from_tick, to_tick)`.
    #[must_use]
    pub fn bga_poor_events_in_range(&self, from_tick: u64, to_tick: u64) -> &[BgaTimelineEvent] {
        Self::events_in_range(&self.chart.bga.poor_events, from_tick, to_tick)
    }

    /// Return BGA resources.
    #[must_use]
    pub fn bga_resources(&self) -> &[bmsrs_chart::BgaResource] {
        &self.chart.bga.resources
    }

    // Chart access

    /// Borrow the underlying chart.
    #[must_use]
    pub fn chart(&self) -> &Chart<T> {
        &self.chart
    }

    /// Consume the player and return the chart.
    #[must_use]
    pub fn into_chart(self) -> Chart<T> {
        self.chart
    }

    /// Binary-search a sorted event vector for a tick range.
    #[expect(
        clippy::indexing_slicing,
        reason = "indices from partition_point on same vector"
    )]
    fn events_in_range(
        events: &[BgaTimelineEvent],
        from_tick: u64,
        to_tick: u64,
    ) -> &[BgaTimelineEvent] {
        let start = events.partition_point(|e| e.tick < from_tick);
        let end = events.partition_point(|e| e.tick < to_tick);
        &events[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmsrs_chart::{Bga, ChartMetadata, DefaultNoteData, ScrollChangeEvent, TimingTrack};

    fn make_chart(notes: Vec<Note>) -> Chart {
        Chart {
            metadata: ChartMetadata::default(),
            resolution: 240,
            lane_count: 8,
            timing: TimingTrack {
                init_bpm: 120.0,
                bpm_changes: vec![],
                stops: vec![],
            },
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            notes,
            bgm: vec![],
            audio_assets: vec![],
            bar_lines: vec![],
            scroll_events: vec![],
            bga: Bga::default(),
        }
    }

    fn note(tick: u64, lane: u16, kind: NoteKind) -> Note {
        Note {
            tick,
            audio: None,
            data: DefaultNoteData { lane, kind },
        }
    }

    #[test]
    fn new_player_starts_at_tick_zero() {
        let player = Player::new(make_chart(vec![]));
        assert_eq!(player.current_tick(), 0);
        assert_eq!(player.current_time(), Duration::ZERO);
    }

    #[test]
    fn advance_one_second_updates_position() {
        let mut player = Player::new(make_chart(vec![]));
        player.advance(Duration::from_secs(1));
        assert_eq!(player.current_time(), Duration::from_secs(1));
    }

    #[test]
    fn advance_multiple_increments_accumulate() {
        let mut player = Player::new(make_chart(vec![]));
        player.advance(Duration::from_millis(500));
        player.advance(Duration::from_millis(500));
        assert_eq!(player.current_time(), Duration::from_secs(1));
    }

    #[test]
    fn seek_sets_absolute_position() {
        let mut player = Player::new(make_chart(vec![]));
        player.advance(Duration::from_secs(2));
        player.seek(Duration::from_millis(500));
        assert_eq!(player.current_time(), Duration::from_millis(500));
    }

    #[test]
    fn reset_returns_to_zero() {
        let mut player = Player::new(make_chart(vec![]));
        player.advance(Duration::from_secs(2));
        player.reset();
        assert_eq!(player.current_tick(), 0);
    }

    #[test]
    fn tick_to_duration_matches_timing_track() {
        let chart = make_chart(vec![]);
        let player = Player::new(chart);

        for tick in [0u64, 120, 240, 480, 960] {
            let expected = TimingTrack {
                init_bpm: 120.0,
                bpm_changes: vec![],
                stops: vec![],
            }
            .tick_to_duration(tick, 240);
            let actual = player.tick_to_duration(tick);
            assert_eq!(actual, expected, "mismatch at tick {tick}");
        }
    }

    #[test]
    fn notes_in_range_returns_subset() {
        let chart = make_chart(vec![
            note(0, 0, NoteKind::Normal),
            note(240, 1, NoteKind::Normal),
            note(480, 0, NoteKind::Normal),
            note(720, 1, NoteKind::Normal),
            note(960, 0, NoteKind::Normal),
        ]);
        let player = Player::new(chart);

        let range = player.notes_in_range(240, 720);
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].tick, 240);
        assert_eq!(range[1].tick, 480);
    }

    #[test]
    fn notes_in_lane_filters_by_lane() {
        let chart = make_chart(vec![
            note(0, 0, NoteKind::Normal),
            note(240, 1, NoteKind::Normal),
            note(480, 0, NoteKind::Normal),
        ]);
        let player = Player::new(chart);

        let lane0: Vec<&Note> = player.notes_in_lane(0, 0, 960).collect();
        assert_eq!(lane0.len(), 2);
        assert_eq!(lane0[0].tick, 0);
        assert_eq!(lane0[1].tick, 480);
    }

    #[test]
    fn notes_for_judgement_excludes_invisible_and_mines() {
        let chart = make_chart(vec![
            note(0, 0, NoteKind::Normal),
            note(240, 0, NoteKind::Invisible),
            note(480, 0, NoteKind::Mine { damage: 1.0 }),
            note(720, 0, NoteKind::Long { duration: 240 }),
        ]);
        let player = Player::new(chart);

        let judge_notes: Vec<&Note> = player.notes_for_judgement(0, 960).collect();
        assert_eq!(judge_notes.len(), 2);
        assert_eq!(judge_notes[0].tick, 0);
        assert_eq!(judge_notes[1].tick, 720);
    }

    #[test]
    fn bgm_in_range_returns_events() {
        let chart: Chart = Chart {
            metadata: ChartMetadata::default(),
            resolution: 240,
            lane_count: 8,
            timing: TimingTrack {
                init_bpm: 120.0,
                bpm_changes: vec![],
                stops: vec![],
            },
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            notes: vec![],
            bgm: vec![
                BgmEvent { tick: 0, audio: 0 },
                BgmEvent {
                    tick: 480,
                    audio: 1,
                },
                BgmEvent {
                    tick: 960,
                    audio: 2,
                },
            ],
            audio_assets: vec![],
            bar_lines: vec![],
            scroll_events: vec![],
            bga: Bga::default(),
        };
        let player = Player::new(chart);

        let range = player.bgm_in_range(240, 960);
        assert_eq!(range.len(), 1);
        assert_eq!(range[0].tick, 480);
    }

    #[test]
    fn scroll_rate_at_returns_latest_multiplier() {
        let chart: Chart = Chart {
            metadata: ChartMetadata::default(),
            resolution: 240,
            lane_count: 8,
            timing: TimingTrack {
                init_bpm: 120.0,
                bpm_changes: vec![],
                stops: vec![],
            },
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            notes: vec![],
            bgm: vec![],
            audio_assets: vec![],
            bar_lines: vec![],
            scroll_events: vec![
                ScrollChangeEvent {
                    tick: 240,
                    rate: 2.0,
                },
                ScrollChangeEvent {
                    tick: 720,
                    rate: 0.5,
                },
            ],
            bga: Bga::default(),
        };
        let player = Player::new(chart);

        assert!((player.scroll_rate_at(0) - 1.0).abs() < 1e-9);
        assert!((player.scroll_rate_at(240) - 2.0).abs() < 1e-9);
        assert!((player.scroll_rate_at(480) - 2.0).abs() < 1e-9);
        assert!((player.scroll_rate_at(720) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn bar_lines_in_range_returns_subset() {
        let chart: Chart = Chart {
            metadata: ChartMetadata::default(),
            resolution: 240,
            lane_count: 8,
            timing: TimingTrack {
                init_bpm: 120.0,
                bpm_changes: vec![],
                stops: vec![],
            },
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            notes: vec![],
            bgm: vec![],
            audio_assets: vec![],
            bar_lines: vec![
                BarLine { tick: 0 },
                BarLine { tick: 960 },
                BarLine { tick: 1920 },
            ],
            scroll_events: vec![],
            bga: Bga::default(),
        };
        let player = Player::new(chart);

        let range = player.bar_lines_in_range(500, 2000);
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].tick, 960);
        assert_eq!(range[1].tick, 1920);
    }

    #[test]
    fn current_bpm_returns_active_bpm() {
        use bmsrs_chart::BpmChange;
        let chart: Chart = Chart {
            metadata: ChartMetadata::default(),
            resolution: 240,
            lane_count: 8,
            timing: TimingTrack {
                init_bpm: 120.0,
                bpm_changes: vec![BpmChange {
                    tick: 480,
                    bpm: 200.0,
                }],
                stops: vec![],
            },
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            notes: vec![],
            bgm: vec![],
            audio_assets: vec![],
            bar_lines: vec![],
            scroll_events: vec![],
            bga: Bga::default(),
        };
        let mut player = Player::new(chart);

        assert!((player.current_bpm() - 120.0).abs() < 1e-9);
        player.advance(Duration::from_secs(10));
        assert!((player.current_bpm() - 200.0).abs() < 1e-9);
    }

    #[test]
    fn into_chart_returns_original_chart() {
        let chart = make_chart(vec![note(0, 0, NoteKind::Normal)]);
        let player = Player::new(chart);
        let recovered = player.into_chart();
        assert_eq!(recovered.notes.len(), 1);
    }
}
