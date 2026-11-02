//! Precomputed SPEED keyframe interpolation cache.
//!
//! SPEED is a presentation multiplier. beatoraja interpolates it in the time
//! domain (`LaneRenderer.getCurrentSpeed(microtime)`), so this cache stores each
//! keyframe's wall-clock time and interpolates with
//! [`SpeedCache::spacing_at_time`].

use std::time::Duration;

use bmsrs_chart::{CustomEvent, Event, EventKind, NoteExt, TimingTrack};

/// SPEED keyframe: wall-clock time and spacing multiplier.
#[derive(Clone, Copy, Debug)]
struct SpeedKeyframe {
    /// Wall-clock time of the keyframe.
    time: Duration,
    /// Spacing multiplier at this keyframe.
    rate: f64,
}

/// SPEED spacing interpolation cache.
///
/// Keyframes are collected from [`EventKind::Speed`] events at construction.
/// `spacing_at_time` linearly interpolates between adjacent keyframes. Before
/// the first keyframe it interpolates from `1.0` at time zero; after the last it
/// returns the last keyframe's value.
pub struct SpeedCache {
    /// Keyframes sorted ascending by time.
    keyframes: Vec<SpeedKeyframe>,
}

impl SpeedCache {
    /// Build the cache from a sorted event list and its timing track.
    ///
    /// Only `Speed` variants are visited; other event types are skipped. The
    /// input `events` must be sorted ascending by [`Event::tick`] (guaranteed by
    /// the chart). The timing track converts each event tick into its wall-clock
    /// activation time.
    pub fn build<T: NoteExt, C: CustomEvent>(events: &[Event<T, C>], timing: &TimingTrack) -> Self {
        let keyframes: Vec<SpeedKeyframe> = events
            .iter()
            .filter_map(|event| {
                if let EventKind::Speed { rate } = &event.kind {
                    Some(SpeedKeyframe {
                        time: timing.tick_to_duration(event.tick()),
                        rate: *rate,
                    })
                } else {
                    None
                }
            })
            .collect();
        Self { keyframes }
    }

    /// Return the interpolated spacing multiplier at `time`.
    ///
    /// With no keyframes, returns `1.0`. Before the first keyframe, interpolates
    /// from `1.0` at time zero, matching beatoraja. After the last keyframe,
    /// returns the last keyframe's spacing.
    pub fn spacing_at_time(&self, time: Duration) -> f64 {
        let Some(first) = self.keyframes.first() else {
            return 1.0;
        };

        let next_index = self
            .keyframes
            .partition_point(|keyframe| keyframe.time <= time);
        let previous_keyframe = next_index
            .checked_sub(1)
            .and_then(|index| self.keyframes.get(index));
        let next_keyframe = self.keyframes.get(next_index);

        match (previous_keyframe, next_keyframe) {
            (Some(previous), Some(next)) => {
                interpolate(previous.time, previous.rate, next.time, next.rate, time)
            }
            (Some(previous), None) => previous.rate,
            (None, Some(next)) => interpolate(Duration::ZERO, 1.0, next.time, next.rate, time),
            (None, None) => first.rate,
        }
    }
}

/// Linear interpolation between two keyframes, clamped to the segment.
fn interpolate(
    start_time: Duration,
    start_rate: f64,
    end_time: Duration,
    end_rate: f64,
    time: Duration,
) -> f64 {
    if end_time <= start_time {
        return end_rate;
    }
    let span = end_time.saturating_sub(start_time).as_secs_f64();
    let elapsed = time.saturating_sub(start_time).as_secs_f64();
    let ratio = (elapsed / span).clamp(0.0, 1.0);
    (end_rate - start_rate).mul_add(ratio, start_rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmsrs_chart::{BpmChange, NoCustomEvent};

    fn make_timing() -> TimingTrack {
        TimingTrack::simple(120.0, 240).unwrap()
    }

    fn make_speed_events(pairs: &[(u64, f64)]) -> Vec<Event<(), NoCustomEvent>> {
        pairs
            .iter()
            .map(|&(tick, rate)| Event::speed(tick, rate))
            .collect()
    }

    #[test]
    fn no_events_returns_default() {
        let timing = make_timing();
        let events = make_speed_events(&[]);
        let cache = SpeedCache::build(&events, &timing);
        assert!((cache.spacing_at_time(Duration::ZERO) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at_time(Duration::from_secs(10)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn before_first_keyframe_interpolates_from_one() {
        let timing = make_timing();
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events, &timing);

        // tick 240 is 0.5s at 120 BPM; the keyframe is 1.0s.
        assert!((cache.spacing_at_time(Duration::from_secs_f64(0.5)) - 0.75).abs() < 1e-9);
    }

    #[test]
    fn after_last_keyframe_returns_last_rate() {
        let timing = make_timing();
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events, &timing);
        assert!((cache.spacing_at_time(Duration::from_secs(10)) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn at_keyframe_exact() {
        let timing = make_timing();
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events, &timing);
        assert!((cache.spacing_at_time(Duration::from_secs(1)) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn interpolation_between_keyframes() {
        let timing = make_timing();
        let events = make_speed_events(&[(0, 0.5), (960, 1.5)]);
        let cache = SpeedCache::build(&events, &timing);

        assert!((cache.spacing_at_time(Duration::ZERO) - 0.5).abs() < 1e-9);
        assert!((cache.spacing_at_time(Duration::from_secs(1)) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at_time(Duration::from_secs(2)) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn bpm_change_uses_time_axis() {
        let timing = TimingTrack::new(
            120.0,
            vec![BpmChange {
                tick: 240,
                bpm: 240.0,
            }],
            vec![],
            240,
        )
        .unwrap();
        let events = make_speed_events(&[(0, 1.0), (480, 2.0)]);
        let cache = SpeedCache::build(&events, &timing);

        // tick 240 is 0.5s; tick 480 is 0.75s, so the 0.5s point is two thirds
        // of the way from 1.0 to 2.0.
        assert!((cache.spacing_at_time(Duration::from_secs_f64(0.5)) - (5.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn duplicate_keyframes_do_not_produce_nan() {
        let timing = make_timing();
        let events = make_speed_events(&[(480, 0.5), (480, 1.5), (960, 1.0)]);
        let cache = SpeedCache::build(&events, &timing);
        let result = cache.spacing_at_time(Duration::from_secs_f64(1.5));
        assert!(result.is_finite(), "spacing must be finite, got {result}");
    }
}
