//! Timing track for tick to `Duration` conversion.
//!
//! [`TimingTrack`] holds the initial BPM, BPM change events, and stop events. It
//! provides [`TimingTrack::tick_to_duration`] and
//! [`TimingTrack::duration_to_tick`] for conversion between chart positions
//! (ticks) and wall-clock time ([`Duration`]).
//!
//! Internal computation uses `f64` arithmetic (BPM values are inherently
//! floating point). `f64` to [`Duration`] conversion happens only at public API
//! boundaries, through [`Duration::from_secs_f64`] and
//! [`Duration::as_secs_f64`].

use std::time::Duration;

/// Errors that occur during `TimingTrack` construction or validation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TimingTrackError {
    /// Initial BPM is invalid (must be non-zero and finite).
    InvalidBpm {
        /// Invalid BPM value.
        bpm: f64,
    },
    /// Beat resolution `resolution` is zero (must be positive).
    ZeroResolution,
}

impl std::fmt::Display for TimingTrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBpm { bpm } => {
                write!(f, "invalid initial BPM: {bpm} (must be non-zero finite)")
            }
            Self::ZeroResolution => write!(f, "resolution must be positive, got 0"),
        }
    }
}

impl std::error::Error for TimingTrackError {}

impl TimingTrackError {
    /// Return the BPM value that caused the error; returns `0.0` for
    /// [`ZeroResolution`](Self::ZeroResolution).
    #[must_use]
    pub const fn bpm(&self) -> f64 {
        match self {
            Self::InvalidBpm { bpm } => *bpm,
            Self::ZeroResolution => 0.0,
        }
    }
}

/// Timing information for converting tick positions into wall-clock time.
///
/// All events are at absolute tick positions. Processors convert format-specific
/// positions (BMSON ticks, BMS measures) into ticks.
///
/// Internally uses precomputed BPM segments and a stop cumulative sum, making
/// every query O(log n).
#[derive(Clone, Debug)]
pub struct TimingTrack {
    /// Initial BPM at tick 0.
    pub(crate) init_bpm: f64,
    /// Ticks per quarter note (beat resolution).
    resolution: u64,
    /// BPM change events, sorted ascending by tick.
    pub(crate) bpm_changes: Vec<BpmChange>,
    /// Stop (pause) events, sorted ascending by tick.
    pub(crate) stops: Vec<StopEvent>,
    /// BPM segments sorted by `start_tick`.
    bpm_segments: Vec<BpmSegment>,
    /// Sorted `(stop_tick, cumulative_pause_seconds)` pairs.
    stop_cumsum: Vec<(u64, f64)>,
    /// Inverse lookup segments for `duration_to_tick` (ascending by `sec_lo`).
    inv: Vec<InvSeg>,
}

impl TimingTrack {
    /// Return the initial BPM.
    #[must_use]
    pub const fn init_bpm(&self) -> f64 {
        self.init_bpm
    }

    /// Return the BPM change events slice.
    ///
    /// Invalid BPM changes are dropped at construction, so the returned slice
    /// contains only effective changes.
    #[must_use]
    pub fn bpm_changes(&self) -> &[BpmChange] {
        &self.bpm_changes
    }

    /// Return the stop events slice.
    #[must_use]
    pub fn stops(&self) -> &[StopEvent] {
        &self.stops
    }

    /// Return the beat resolution.
    #[must_use]
    pub const fn resolution(&self) -> u64 {
        self.resolution
    }
}

/// BPM change event.
#[derive(Clone, Debug, PartialEq)]
pub struct BpmChange {
    /// Tick position where the BPM changes.
    pub tick: u64,
    /// New BPM (beats per minute).
    pub bpm: f64,
}

impl BpmChange {
    /// Return whether a BPM value is valid (finite and non-zero).
    ///
    /// BPM changes with invalid values (`0.0`, `NaN`, `+/-Inf`) are dropped during
    /// timing track construction, keeping the previous valid BPM, matching
    /// beatoraja behavior. Both processors share this predicate so the timing
    /// track and the unified event stream filter identically.
    #[must_use]
    pub const fn is_valid(bpm: f64) -> bool {
        bpm.is_finite() && bpm != 0.0
    }
}

/// Stop (pause) event.
///
/// When playback reaches `tick`, scrolling pauses for `duration` ticks (computed
/// with the current BPM). Multiple stops at the same tick accumulate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopEvent {
    /// Tick position where the stop begins.
    pub tick: u64,
    /// Duration in ticks.
    pub duration: u64,
}

/// Internal event representation for the merged timeline.
#[derive(Clone, Copy, Debug)]
enum TimingEvent {
    /// BPM change; ordered before Stop when at the same tick.
    Bpm(f64),
    /// Stop, with duration in ticks.
    Stop(u64),
}

/// Safely convert seconds into [`Duration`], avoiding overflow panics.
///
/// Negative values and NaN clamp to [`Duration::ZERO`]; values above
/// [`Duration::MAX`] clamp to [`Duration::MAX`]. Values in the normal range are
/// unaffected.
fn safe_from_secs_f64(secs: f64) -> Duration {
    Duration::try_from_secs_f64(secs.max(0.0)).unwrap_or(Duration::MAX)
}

impl TimingTrack {
    /// Create a new timing track.
    ///
    /// Precomputes all internal segments at construction; later queries are
    /// O(log n).
    ///
    /// # Errors
    ///
    /// - Returns [`TimingTrackError::ZeroResolution`] when `resolution == 0`.
    /// - Returns [`TimingTrackError::InvalidBpm`] when `init_bpm` is invalid
    ///   (zero, NaN, or infinity).
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    pub fn new(
        init_bpm: f64,
        mut bpm_changes: Vec<BpmChange>,
        mut stops: Vec<StopEvent>,
        resolution: u64,
    ) -> Result<Self, TimingTrackError> {
        if resolution == 0 {
            return Err(TimingTrackError::ZeroResolution);
        }
        if !init_bpm.is_finite() || init_bpm == 0.0 {
            return Err(TimingTrackError::InvalidBpm { bpm: init_bpm });
        }

        let res = resolution as f64;

        // Sort once at construction to avoid clone+sort downstream.
        bpm_changes.sort_by_key(|bc| bc.tick);
        // Drop invalid BPM changes before storing so bpm_changes() and
        // bpm_at_tick see the same set; bpm_segments is built from that same set.
        bpm_changes.retain(|bc| BpmChange::is_valid(bc.bpm));
        stops.sort_by_key(|s| s.tick);

        // Build BPM segments
        let mut bpm_segments = vec![BpmSegment {
            start_tick: 0,
            start_seconds: 0.0,
            bpm: init_bpm,
        }];

        let mut current_tick = 0u64;
        let mut current_seconds = 0.0f64;
        let mut current_bpm = init_bpm;

        for bc in &bpm_changes {
            if bc.tick > current_tick {
                current_seconds += (bc.tick - current_tick) as f64 / res * 60.0 / current_bpm.abs();
                current_tick = bc.tick;
            }
            current_bpm = bc.bpm;
            bpm_segments.push(BpmSegment {
                start_tick: current_tick,
                start_seconds: current_seconds,
                bpm: current_bpm,
            });
        }

        // Build stop cumulative sum
        let mut stop_cumsum = Vec::with_capacity(stops.len());
        let mut total_pause = 0.0f64;
        for stop in &stops {
            let bpm = segment_bpm_at_tick(&bpm_segments, stop.tick);
            total_pause += stop.duration as f64 / res * 60.0 / bpm.abs();
            stop_cumsum.push((stop.tick, total_pause));
        }

        // Build inverse segments (bpm_changes and stops are already sorted).
        let inv = merge_inv(init_bpm, &bpm_changes, &stops, resolution);

        Ok(Self {
            init_bpm,
            resolution,
            bpm_changes,
            stops,
            bpm_segments,
            stop_cumsum,
            inv,
        })
    }

    /// Create a timing track with only the initial BPM, no changes, and no
    /// stops.
    ///
    /// Equivalent to `TimingTrack::new(bpm, vec![], vec![], resolution)` without
    /// manually passing empty vectors; more concise in tests and simple cases.
    ///
    /// # Errors
    ///
    /// Returns [`TimingTrackError::InvalidBpm`] when `init_bpm` is invalid, and
    /// [`TimingTrackError::ZeroResolution`] when `resolution == 0`.
    pub fn simple(init_bpm: f64, resolution: u64) -> Result<Self, TimingTrackError> {
        Self::new(init_bpm, Vec::new(), Vec::new(), resolution)
    }

    /// Convert a tick position into wall-clock time [`Duration`].
    ///
    /// At a tick with a stop event, the returned time is the moment **before**
    /// the pause (per the BMSON specification, notes at that tick activate before
    /// the pause). Stops strictly before the target tick contribute their full
    /// pause duration.
    ///
    /// Performs an O(log n) binary search over precomputed segments.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "idx from saturating_sub on partition_point, always valid"
    )]
    pub fn tick_to_duration(&self, tick: u64) -> Duration {
        let base = self.base_seconds_at_tick(tick);

        // Cumulative stop pause strictly before the target tick.
        let stop_idx = self.stop_cumsum.partition_point(|(t, _)| *t < tick);
        let pause = if stop_idx > 0 {
            self.stop_cumsum[stop_idx - 1].1
        } else {
            0.0
        };

        safe_from_secs_f64(base + pause)
    }

    /// Convert a tick position into musical time excluding STOP pauses.
    ///
    /// Unlike [`tick_to_duration`](Self::tick_to_duration), this excludes pause
    /// durations of all STOPs before the target tick. Audio slicing uses this
    /// method for `AudioAsset.start` and `duration` because audio pauses during a
    /// STOP, so the paused segment should not be skipped in the source file.
    #[must_use]
    pub fn tick_to_musical_duration(&self, tick: u64) -> Duration {
        safe_from_secs_f64(self.base_seconds_at_tick(tick))
    }

    /// Return the base seconds at `tick`, excluding STOP pauses.
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "idx from saturating_sub on partition_point, always valid"
    )]
    fn base_seconds_at_tick(&self, tick: u64) -> f64 {
        let res = self.resolution as f64;
        let idx = self
            .bpm_segments
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1);
        let seg = &self.bpm_segments[idx];
        seg.start_seconds + (tick - seg.start_tick) as f64 / res * 60.0 / seg.bpm.abs()
    }

    /// Convert wall-clock time [`Duration`] into the nearest tick position.
    ///
    /// This is the inverse of [`tick_to_duration`](Self::tick_to_duration). Time
    /// spent inside a stop does not advance ticks.
    ///
    /// Performs a binary search over precomputed inverse segments, O(log n).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded result is within u64 range"
    )]
    #[expect(clippy::cast_sign_loss, reason = "remaining time is non-negative")]
    #[expect(
        clippy::indexing_slicing,
        reason = "inv non-empty (always has initial seg); idx from saturating_sub on partition_point"
    )]
    pub fn duration_to_tick(&self, duration: Duration) -> u64 {
        let target = duration.as_secs_f64();
        if target <= 0.0 {
            return 0;
        }

        let idx = self
            .inv
            .partition_point(|s| s.sec_lo <= target)
            .saturating_sub(1);
        let seg = &self.inv[idx];

        if seg.bpm == 0.0 {
            // Frozen segment (inside a stop): ticks do not advance.
            return seg.tick;
        }

        // Interpolate within a linear segment: dt seconds maps to
        // dt * res * |bpm| / 60 ticks.
        let dt = target - seg.sec_lo;
        seg.tick + (dt * self.resolution as f64 * seg.bpm.abs() / 60.0).round() as u64
    }

    /// Return the BPM in effect at `tick`.
    ///
    /// Delegates to [`BpmLookup`], sharing the same implementation as the
    /// pre-construction lightweight lookup.
    #[must_use]
    pub fn bpm_at_tick(&self, tick: u64) -> f64 {
        BpmLookup::new(self.init_bpm, &self.bpm_changes).bpm_at_tick(tick)
    }
}

impl Default for TimingTrack {
    #[expect(
        clippy::expect_used,
        reason = "120 BPM and resolution 240 are hardcoded constants that are always valid"
    )]
    fn default() -> Self {
        Self::new(120.0, vec![], vec![], 240).expect("120 BPM at resolution 240 is always valid")
    }
}

impl PartialEq for TimingTrack {
    fn eq(&self, other: &Self) -> bool {
        self.init_bpm == other.init_bpm
            && self.resolution == other.resolution
            && self.bpm_changes == other.bpm_changes
            && self.stops == other.stops
    }
}

// init_bpm is guaranteed not to be NaN (validated by TimingTrack::new), so
// PartialEq satisfies Eq's reflexivity requirement.
impl Eq for TimingTrack {}

/// A segment of the timing track with constant BPM.
#[derive(Clone, Debug)]
struct BpmSegment {
    /// Starting tick of this segment.
    start_tick: u64,
    /// Wall-clock seconds at `start_tick` (excluding stops).
    start_seconds: f64,
    /// BPM within this segment.
    bpm: f64,
}

/// One inverse-lookup segment for `duration_to_tick`.
///
/// Splits the monotonic `total_seconds(tick)` function into segments: linear
/// segments with constant BPM (ticks advance with time) and stop-frozen segments
/// (ticks stay fixed while time skips the stop duration). Each segment's `sec_lo`
/// is strictly increasing, enabling O(log n) binary search.
#[derive(Clone, Debug)]
struct InvSeg {
    /// Wall-clock time at the start of this segment (seconds).
    sec_lo: f64,
    /// Tick position at `sec_lo`.
    tick: u64,
    /// BPM of this segment; `0.0` means a frozen segment (ticks do not advance
    /// during a stop).
    bpm: f64,
}

/// Lightweight lookup for querying the effective BPM at a given tick.
///
/// Suitable for querying BPM before constructing a [`TimingTrack`] (for example,
/// converting STP milliseconds into ticks) and any one-off or low-frequency query
/// that does not need a full timing track.
///
/// Semantically identical to [`TimingTrack::bpm_at_tick`] without building a full
/// timing track.
#[derive(Clone, Debug)]
pub struct BpmLookup<'a> {
    /// Initial BPM at tick 0.
    init_bpm: f64,
    /// BPM change events, sorted ascending by `tick`.
    bpm_changes: &'a [BpmChange],
}

impl BpmLookup<'_> {
    /// Construct a `BpmLookup`.
    ///
    /// `bpm_changes` must be sorted ascending by `tick` (the caller guarantees
    /// this; [`BpmChange`] itself has no ordering invariant). `init_bpm` is
    /// **not** validated (unlike [`TimingTrack::new`]): passing zero or a
    /// non-finite value makes [`bpm_at_tick`](Self::bpm_at_tick) return that
    /// invalid value. Callers that depend on conversion results must ensure
    /// `init_bpm` is valid; in the BMS pipeline, [`TimingTrack::new`] performs
    /// final validation at the later construction point.
    #[must_use]
    pub const fn new(init_bpm: f64, bpm_changes: &[BpmChange]) -> BpmLookup<'_> {
        BpmLookup {
            init_bpm,
            bpm_changes,
        }
    }

    /// Return the BPM in effect at `tick` (the last BPM change at or before
    /// `tick`).
    ///
    /// Returns the `init_bpm` passed at construction when `tick` precedes every
    /// BPM change.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "the match arm guarantees idx >= 1, so idx-1 is in bounds"
    )]
    pub fn bpm_at_tick(&self, tick: u64) -> f64 {
        match self.bpm_changes.partition_point(|bc| bc.tick <= tick) {
            0 => self.init_bpm,
            idx => self.bpm_changes[idx - 1].bpm,
        }
    }
}

/// Binary-search the BPM segments and return the BPM in effect at `tick`.
#[expect(
    clippy::indexing_slicing,
    reason = "idx from saturating_sub on partition_point, always valid"
)]
fn segment_bpm_at_tick(bpm_segments: &[BpmSegment], tick: u64) -> f64 {
    let idx = bpm_segments
        .partition_point(|s| s.start_tick <= tick)
        .saturating_sub(1);
    bpm_segments[idx].bpm
}

/// Merge sorted BPM change and stop events.
///
/// Both inputs are assumed sorted ascending by `tick`; at the same tick, BPM
/// comes before Stop ("speed will first change, then the music pauses").
fn merge_sorted_events(bpm_changes: &[BpmChange], stops: &[StopEvent]) -> Vec<(u64, TimingEvent)> {
    let mut events = Vec::with_capacity(bpm_changes.len() + stops.len());
    let mut bpm_iter = bpm_changes.iter().peekable();
    let mut stop_iter = stops.iter().peekable();
    loop {
        match (bpm_iter.peek(), stop_iter.peek()) {
            (Some(bc), Some(st)) if bc.tick <= st.tick => {
                events.push((bc.tick, TimingEvent::Bpm(bc.bpm)));
                bpm_iter.next();
            }
            (Some(bc), None) => {
                events.push((bc.tick, TimingEvent::Bpm(bc.bpm)));
                bpm_iter.next();
            }
            (_, Some(st)) => {
                events.push((st.tick, TimingEvent::Stop(st.duration)));
                stop_iter.next();
            }
            (None, None) => break,
        }
    }
    events
}

/// Build the inverse lookup segments for `duration_to_tick`.
///
/// Merges BPM changes and stop events, walks them ascending by
/// `(tick, is_stop)`, accumulates wall-clock seconds, and records
/// `(sec_lo, tick, bpm)` at each breakpoint. A stop produces a `bpm = 0.0` frozen
/// segment, after which a linear segment continues at the same tick with the same
/// BPM.
#[expect(clippy::cast_precision_loss, reason = "tick/duration fit in f64")]
fn merge_inv(
    init_bpm: f64,
    bpm_changes: &[BpmChange],
    stops: &[StopEvent],
    resolution: u64,
) -> Vec<InvSeg> {
    let res_f = resolution as f64;

    let events = merge_sorted_events(bpm_changes, stops);
    let mut inv = Vec::with_capacity(events.len() * 2 + 1);
    let mut cur_tick = 0u64;
    let mut cur_sec = 0.0f64;
    let mut cur_bpm = init_bpm;
    inv.push(InvSeg {
        sec_lo: 0.0,
        tick: 0,
        bpm: init_bpm,
    });

    for (tick, ev) in events {
        if tick > cur_tick {
            cur_sec += (tick - cur_tick) as f64 / res_f * 60.0 / cur_bpm.abs();
            cur_tick = tick;
        }
        match ev {
            TimingEvent::Bpm(b) => {
                cur_bpm = b;
                inv.push(InvSeg {
                    sec_lo: cur_sec,
                    tick: cur_tick,
                    bpm: b,
                });
            }
            TimingEvent::Stop(d) => {
                let d_sec = d as f64 / res_f * 60.0 / cur_bpm.abs();
                inv.push(InvSeg {
                    sec_lo: cur_sec,
                    tick: cur_tick,
                    bpm: 0.0,
                });
                cur_sec += d_sec;
                inv.push(InvSeg {
                    sec_lo: cur_sec,
                    tick: cur_tick,
                    bpm: cur_bpm,
                });
            }
        }
    }
    inv
}

#[cfg(test)]
mod tests {
    use super::*;

    const RES: u64 = 240;

    // Basic functionality (direct tests of the new TimingTrack)

    #[test]
    fn constant_bpm_tick_zero_is_zero() {
        let timing = TimingTrack::simple(120.0, RES).unwrap();
        let result = timing.tick_to_duration(0);
        assert_eq!(result, Duration::ZERO);
    }

    #[test]
    fn constant_bpm_120_one_beat_is_half_second() {
        let timing = TimingTrack::simple(120.0, RES).unwrap();
        let result = timing.tick_to_duration(240);
        assert_eq!(result, Duration::from_millis(500));
    }

    #[test]
    fn bpm_change_segment_boundary() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: 60.0,
            }],
            vec![],
            RES,
        )
        .unwrap();
        let result = timing.tick_to_duration(480);
        assert_eq!(result, Duration::from_millis(1500));
    }

    #[test]
    fn stop_strictly_before_target_adds_pause() {
        let timing = TimingTrack::new(
            120.0,
            vec![],
            vec![StopEvent {
                tick: 240,
                duration: 240,
            }],
            RES,
        )
        .unwrap();
        let result = timing.tick_to_duration(241);
        let expected = 0.5 + 0.5 + 1.0 / 480.0;
        assert!((result.as_secs_f64() - expected).abs() < 1e-9);
    }

    #[test]
    fn stop_at_target_excludes_pause() {
        let timing = TimingTrack::new(
            120.0,
            vec![],
            vec![StopEvent {
                tick: 240,
                duration: 240,
            }],
            RES,
        )
        .unwrap();
        let result = timing.tick_to_duration(240);
        assert_eq!(result, Duration::from_millis(500));
    }

    #[test]
    fn bpm_at_tick_returns_correct_bpm() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 480,
                bpm: 200.0,
            }],
            vec![],
            RES,
        )
        .unwrap();

        assert!((timing.bpm_at_tick(0) - 120.0).abs() < 1e-9);
        assert!((timing.bpm_at_tick(480) - 200.0).abs() < 1e-9);
        assert!((timing.bpm_at_tick(960) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn unsorted_bpm_changes_sorted_internally() {
        let timing = TimingTrack::new(
            120.0,
            vec![
                BpmChange {
                    tick: 480,
                    bpm: 200.0,
                },
                BpmChange {
                    tick: 240,
                    bpm: 60.0,
                },
            ],
            vec![],
            RES,
        )
        .unwrap();

        assert_eq!(timing.tick_to_duration(480), Duration::from_millis(1500));
    }

    #[test]
    fn zero_bpm_change_skipped() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: 0.0,
            }],
            vec![],
            RES,
        )
        .unwrap();

        assert_eq!(timing.tick_to_duration(480), Duration::from_secs(1));
    }

    #[test]
    fn nan_bpm_change_skipped() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: f64::NAN,
            }],
            vec![],
            RES,
        )
        .unwrap();

        assert_eq!(timing.tick_to_duration(480), Duration::from_secs(1));
    }

    #[test]
    fn inf_bpm_change_skipped() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: f64::INFINITY,
            }],
            vec![],
            RES,
        )
        .unwrap();

        assert_eq!(timing.tick_to_duration(480), Duration::from_secs(1));
    }

    #[test]
    fn valid_bpm_after_invalid_one_works() {
        let timing = TimingTrack::new(
            120.0,
            vec![
                BpmChange {
                    tick: 240,
                    bpm: 0.0,
                },
                BpmChange {
                    tick: 480,
                    bpm: 60.0,
                },
            ],
            vec![],
            RES,
        )
        .unwrap();

        assert_eq!(timing.tick_to_duration(720), Duration::from_secs(2));
    }

    #[test]
    fn extreme_stop_does_not_panic() {
        let timing = TimingTrack::new(
            0.1,
            vec![],
            vec![StopEvent {
                tick: 0,
                duration: u64::MAX,
            }],
            RES,
        )
        .unwrap();

        assert_eq!(timing.tick_to_duration(1), Duration::MAX);
    }

    // Construction errors

    #[test]
    fn new_rejects_zero_init_bpm() {
        let result = TimingTrack::new(0.0, vec![], vec![], RES);
        assert_eq!(result, Err(TimingTrackError::InvalidBpm { bpm: 0.0 }));
    }

    #[test]
    fn new_accepts_negative_init_bpm() {
        let timing = TimingTrack::new(-120.0, vec![], vec![], RES).unwrap();
        assert!((timing.init_bpm() - (-120.0)).abs() < 1e-9);
    }

    #[test]
    fn new_rejects_nan_init_bpm() {
        let result = TimingTrack::new(f64::NAN, vec![], vec![], RES);
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_infinite_init_bpm() {
        let result = TimingTrack::new(f64::INFINITY, vec![], vec![], RES);
        assert!(result.is_err());
    }

    #[test]
    fn simple_rejects_invalid_bpm() {
        let result = TimingTrack::simple(0.0, RES);
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_zero_resolution() {
        let result = TimingTrack::new(120.0, vec![], vec![], 0);
        assert_eq!(result, Err(TimingTrackError::ZeroResolution));
    }

    #[test]
    fn simple_rejects_zero_resolution() {
        let result = TimingTrack::simple(120.0, 0);
        assert_eq!(result, Err(TimingTrackError::ZeroResolution));
    }

    #[test]
    fn regression_roundtrip_consistency() {
        let timing = TimingTrack::new(
            150.0,
            vec![
                BpmChange {
                    tick: 480,
                    bpm: 200.0,
                },
                BpmChange {
                    tick: 1200,
                    bpm: 100.0,
                },
            ],
            vec![StopEvent {
                tick: 960,
                duration: 480,
            }],
            RES,
        )
        .unwrap();

        for tick in [0u64, 100, 240, 479, 480, 959, 960, 961, 1200, 2400] {
            let dur = timing.tick_to_duration(tick);
            let back = timing.duration_to_tick(dur);
            assert_eq!(
                back, tick,
                "roundtrip failed at tick {tick}: dur={dur:?}, back={back}",
            );
        }
    }
}
