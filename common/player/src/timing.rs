//! Pre-computed timing cache for O(log n) tick-to-Duration conversion.
//!
//! [`TimingCache`] is constructed from a [`TimingTrack`] at Player creation
//! time and avoids rebuilding the event list on every query. Internal
//! computation uses `f64`; the public API returns [`Duration`].

use std::time::Duration;

use bmsrs_chart::TimingTrack;

/// A segment of constant BPM in the timing track.
struct BpmSegment {
    /// Tick at which this segment starts.
    start_tick: u64,
    /// Wall-clock seconds at `start_tick` (excluding stops).
    start_seconds: f64,
    /// BPM during this segment.
    bpm: f64,
}

/// Pre-computed timing data for fast tick-to-Duration conversion.
///
/// The conversion is split into two parts:
///
/// 1. **Base time** — linear interpolation within constant-BPM segments.
/// 2. **Stop pauses** — cumulative pause time from all stops strictly before
///    the target tick.
///
/// Both parts use binary search, giving O(log n) per query.
pub struct TimingCache {
    /// Sorted BPM segments (by `start_tick`).
    bpm_segments: Vec<BpmSegment>,
    /// Sorted `(stop_tick, cumulative_pause_seconds)` pairs.
    stop_cumsum: Vec<(u64, f64)>,
    /// Ticks per quarter note.
    resolution: u64,
}

impl TimingCache {
    /// Build the cache from a [`TimingTrack`] and resolution.
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    pub(crate) fn new(timing: &TimingTrack, resolution: u64) -> Self {
        debug_assert!(resolution > 0, "resolution must be positive");
        debug_assert!(timing.init_bpm > 0.0, "init_bpm must be positive");

        let res = resolution as f64;

        // Build BPM segments (cumulative seconds without stops).
        let mut bpm_segments = vec![BpmSegment {
            start_tick: 0,
            start_seconds: 0.0,
            bpm: timing.init_bpm,
        }];

        let mut current_tick = 0u64;
        let mut current_seconds = 0.0f64;
        let mut current_bpm = timing.init_bpm;

        for bc in &timing.bpm_changes {
            if bc.tick > current_tick {
                current_seconds += (bc.tick - current_tick) as f64 / res * 60.0 / current_bpm;
                current_tick = bc.tick;
            }
            current_bpm = bc.bpm;
            bpm_segments.push(BpmSegment {
                start_tick: current_tick,
                start_seconds: current_seconds,
                bpm: current_bpm,
            });
        }

        // Build stop cumulative pauses (sorted by tick).
        let mut sorted_stops = timing.stops.clone();
        sorted_stops.sort_by_key(|s| s.tick);

        let mut stop_cumsum = Vec::with_capacity(sorted_stops.len());
        let mut total_pause = 0.0f64;
        for stop in &sorted_stops {
            let bpm = segment_bpm_at_tick(&bpm_segments, stop.tick);
            total_pause += stop.duration as f64 / res * 60.0 / bpm;
            stop_cumsum.push((stop.tick, total_pause));
        }

        Self {
            bpm_segments,
            stop_cumsum,
            resolution,
        }
    }

    /// Convert a tick position to wall-clock [`Duration`].
    ///
    /// Stops at the target tick itself are NOT counted (matching
    /// [`TimingTrack::tick_to_duration`] semantics).
    #[expect(clippy::cast_precision_loss, reason = "tick fits in f64")]
    #[expect(
        clippy::indexing_slicing,
        reason = "idx from saturating_sub on partition_point, always valid"
    )]
    pub(crate) fn tick_to_duration(&self, tick: u64) -> Duration {
        let res = self.resolution as f64;

        // Base time from BPM segments.
        let idx = self
            .bpm_segments
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1);
        let seg = &self.bpm_segments[idx];
        let base = seg.start_seconds + (tick - seg.start_tick) as f64 / res * 60.0 / seg.bpm;

        // Add cumulative stop pauses strictly before tick.
        let stop_idx = self.stop_cumsum.partition_point(|(t, _)| *t < tick);
        let stop_pause = if stop_idx > 0 {
            self.stop_cumsum[stop_idx - 1].1
        } else {
            0.0
        };

        Duration::from_secs_f64(base + stop_pause)
    }

    /// Return the BPM active at `tick`.
    pub(crate) fn bpm_at_tick(&self, tick: u64) -> f64 {
        segment_bpm_at_tick(&self.bpm_segments, tick)
    }

    /// Convert wall-clock [`Duration`] to the nearest tick position.
    ///
    /// This is the inverse of [`tick_to_duration`](Self::tick_to_duration).
    /// Time spent in stops does not advance the tick.
    ///
    /// Uses binary search on [`tick_to_duration`](Self::tick_to_duration)
    /// for O(log² n) complexity.  The search upper bound is generous
    /// (1000 measures past the last BPM segment) to cover any valid time.
    #[must_use]
    pub(crate) fn duration_to_tick(&self, duration: Duration) -> u64 {
        let target = duration.as_secs_f64();
        if target <= 0.0 {
            return 0;
        }

        // Upper bound: 1000 measures past the last known BPM segment.
        let last_segment_tick = self.bpm_segments.last().map_or(0, |s| s.start_tick);
        let upper = last_segment_tick + self.resolution * 4 * 1000;

        // Binary search: find the last tick whose time ≤ target.
        let mut lo = 0u64;
        let mut hi = upper.max(1);

        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.tick_to_duration(mid).as_secs_f64() <= target {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        lo.saturating_sub(1)
    }
}

/// Binary-search BPM segments for the active BPM at `tick`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use bmsrs_chart::{BpmChange, StopEvent};

    const RES: u64 = 240;

    #[test]
    fn constant_bpm_tick_zero_is_zero() {
        let timing = TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        };
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(0);
        assert_eq!(result, Duration::ZERO);
    }

    #[test]
    fn constant_bpm_120_one_beat_is_half_second() {
        let timing = TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        };
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(240);
        assert_eq!(result, Duration::from_millis(500));
    }

    #[test]
    fn bpm_change_segment_boundary() {
        let timing = TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![BpmChange {
                tick: 240,
                bpm: 60.0,
            }],
            stops: vec![],
        };
        let cache = TimingCache::new(&timing, RES);

        // 0-240 at 120 BPM = 0.5s, 240-480 at 60 BPM = 1.0s.
        let result = cache.tick_to_duration(480);
        assert_eq!(result, Duration::from_millis(1500));
    }

    #[test]
    fn stop_strictly_before_target_adds_pause() {
        let timing = TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![StopEvent {
                tick: 240,
                duration: 240,
            }],
        };
        let cache = TimingCache::new(&timing, RES);

        // Stop at 240 is strictly before 241, so pause is included.
        let result = cache.tick_to_duration(241);
        let expected = 0.5 + 0.5 + 1.0 / 480.0;
        assert!((result.as_secs_f64() - expected).abs() < 1e-9);
    }

    #[test]
    fn stop_at_target_excludes_pause() {
        let timing = TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![StopEvent {
                tick: 240,
                duration: 240,
            }],
        };
        let cache = TimingCache::new(&timing, RES);

        let result = cache.tick_to_duration(240);
        assert_eq!(result, Duration::from_millis(500));
    }

    #[test]
    fn matches_timing_track_constant_bpm() {
        let timing = TimingTrack {
            init_bpm: 150.0,
            bpm_changes: vec![],
            stops: vec![],
        };
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 480, 960, 1920] {
            let expected = timing.tick_to_duration(tick, RES);
            let actual = cache.tick_to_duration(tick);
            assert_eq!(actual, expected, "mismatch at tick {tick}");
        }
    }

    #[test]
    fn matches_timing_track_with_bpm_changes_and_stops() {
        let timing = TimingTrack {
            init_bpm: 150.0,
            bpm_changes: vec![
                BpmChange {
                    tick: 480,
                    bpm: 200.0,
                },
                BpmChange {
                    tick: 1200,
                    bpm: 100.0,
                },
            ],
            stops: vec![StopEvent {
                tick: 960,
                duration: 480,
            }],
        };
        let cache = TimingCache::new(&timing, RES);

        for tick in [0u64, 100, 240, 479, 480, 959, 960, 961, 1200, 2400] {
            let expected = timing.tick_to_duration(tick, RES);
            let actual = cache.tick_to_duration(tick);
            assert_eq!(actual, expected, "mismatch at tick {tick}");
        }
    }

    #[test]
    fn bpm_at_tick_returns_correct_bpm() {
        let timing = TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![BpmChange {
                tick: 480,
                bpm: 200.0,
            }],
            stops: vec![],
        };
        let cache = TimingCache::new(&timing, RES);

        assert!((cache.bpm_at_tick(0) - 120.0).abs() < 1e-9);
        assert!((cache.bpm_at_tick(480) - 200.0).abs() < 1e-9);
        assert!((cache.bpm_at_tick(960) - 200.0).abs() < 1e-9);
    }
}
