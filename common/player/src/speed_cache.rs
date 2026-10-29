//! Precomputed SPEED keyframe interpolation cache.
//!
//! [`SpeedCache`] stores sorted SPEED keyframes and linearly interpolates
//! spacing values between adjacent keyframes at query time.

use bmsrs_chart::{CustomEvent, Event, EventKind, NoteExt};

/// SPEED keyframe: tick position and spacing multiplier.
#[derive(Clone, Copy, Debug)]
struct SpeedKeyframe {
    /// Absolute tick position of the keyframe.
    tick: u64,
    /// Spacing multiplier at this keyframe.
    rate: f64,
}

/// SPEED spacing interpolation cache.
///
/// Keyframes are collected from [`EventKind::Speed`] events at construction.
/// `spacing_at(tick)` linearly interpolates between adjacent keyframes; before
/// the first keyframe it returns `1.0` (the default spacing).
pub struct SpeedCache {
    /// Keyframes sorted ascending by tick.
    keyframes: Vec<SpeedKeyframe>,
}

impl SpeedCache {
    /// Build the cache from a sorted event list.
    ///
    /// Only `Speed` variants are visited; other event types are skipped. The
    /// input `events` must be sorted ascending by [`Event::tick`] (guaranteed by
    /// the chart).
    pub fn build<T: NoteExt, C: CustomEvent>(events: &[Event<T, C>]) -> Self {
        let keyframes: Vec<SpeedKeyframe> = events
            .iter()
            .filter_map(|e| {
                if let EventKind::Speed { rate } = &e.kind {
                    Some(SpeedKeyframe {
                        tick: e.tick(),
                        rate: *rate,
                    })
                } else {
                    None
                }
            })
            .collect();
        Self { keyframes }
    }

    /// Return the interpolated spacing multiplier at `tick`.
    ///
    /// With no keyframes or a query before the first keyframe, returns `1.0`.
    /// After the last keyframe it returns the last keyframe's spacing. Between
    /// two keyframes it performs linear interpolation.
    pub fn spacing_at(&self, tick: u64) -> f64 {
        // Empty keyframe table: default spacing 1.0.
        let Some(first) = self.keyframes.first() else {
            return 1.0;
        };
        if tick < first.tick {
            return 1.0;
        }

        // Find the first keyframe with tick > query position.
        let next_idx = self.keyframes.partition_point(|k| k.tick <= tick);
        if next_idx == 0 {
            return 1.0;
        }
        if next_idx >= self.keyframes.len() {
            // keyframes is non-empty (first() already confirmed an element).
            #[expect(clippy::unwrap_used, reason = "keyframes confirmed non-empty above")]
            return self.keyframes.last().unwrap().rate;
        }

        // Both keyframes[next_idx - 1] and keyframes[next_idx] exist:
        // next_idx > 0 (the == 0 branch above) and next_idx < len (the >= len
        // branch above).
        //
        // Direct indexing is clearer than .get().unwrap() and needs no extra
        // expect.
        #[expect(clippy::indexing_slicing, reason = "bounds confirmed by checks above")]
        let (prev, next) = (&self.keyframes[next_idx - 1], &self.keyframes[next_idx]);
        // Multiple SPEED keyframes at the same tick (malformed input) would
        // divide by zero and produce NaN; skip interpolation and take the later
        // keyframe's value (matching "later overrides earlier").
        if next.tick == prev.tick {
            return next.rate;
        }
        #[expect(clippy::cast_precision_loss, reason = "tick offsets fit in f64")]
        let t = (tick - prev.tick) as f64 / (next.tick - prev.tick) as f64;
        (next.rate - prev.rate).mul_add(t, prev.rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_speed_events(pairs: &[(u64, f64)]) -> Vec<Event<(), bmsrs_chart::NoCustomEvent>> {
        pairs
            .iter()
            .map(|&(tick, rate)| Event::speed(tick, rate))
            .collect()
    }

    #[test]
    fn no_events_returns_default() {
        let events: Vec<Event<(), bmsrs_chart::NoCustomEvent>> = vec![];
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at(9999) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn before_first_keyframe_returns_default() {
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(0) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at(479) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn after_last_keyframe_returns_last_rate() {
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(480) - 0.5).abs() < 1e-9);
        assert!((cache.spacing_at(9999) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn at_keyframe_exact() {
        let events = make_speed_events(&[(480, 0.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(480) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn interpolation_between_keyframes() {
        // Keyframes: tick 0 at 0.5, tick 960 at 1.5.
        // Tick 480 is the midpoint, expected 1.0.
        let events = make_speed_events(&[(0, 0.5), (960, 1.5)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(0) - 0.5).abs() < 1e-9);
        assert!((cache.spacing_at(480) - 1.0).abs() < 1e-9);
        assert!((cache.spacing_at(960) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn duplicate_tick_speed_events_no_nan() {
        // Malformed input: two SPEED keyframes at the same tick. The
        // interpolation denominator is zero, so NaN must be avoided (return the
        // later keyframe's value).
        let events = make_speed_events(&[(480, 0.5), (480, 1.5), (960, 1.0)]);
        let cache = SpeedCache::build(&events);
        let result = cache.spacing_at(720);
        assert!(result.is_finite(), "spacing must be finite, got {result}");
    }

    #[test]
    fn bmspec_6_multiple_speed_interpolation() {
        // Simulate the bmspec-6 multi-value scenario:
        // SPEED01=0.5 at tick 960 (beat 4)
        // SPEED02=1.5 at tick 1440 (beat 6)
        // SPEED03=1.0 at tick 1920 (beat 8)
        let events = make_speed_events(&[(960, 0.5), (1440, 1.5), (1920, 1.0)]);
        let cache = SpeedCache::build(&events);
        assert!((cache.spacing_at(960) - 0.5).abs() < 1e-9, "keyframe 01");
        // Interpolation from keyframe 01 to 02: tick 1200 is the midpoint of 960
        // and 1440.
        // 0.5 + (1.5 - 0.5) * 0.5 = 1.0
        assert!(
            (cache.spacing_at(1200) - 1.0).abs() < 1e-9,
            "interpolation midpoint"
        );
        assert!((cache.spacing_at(1440) - 1.5).abs() < 1e-9, "keyframe 02");
        // Beat 7 = tick 1680: 75% between 1440 (1.5) and 1920 (1.0).
        // 1.5 + (1.0 - 1.5) * (1680-1440)/(1920-1440) = 1.5 + (-0.5 * 0.5) = 1.25
        assert!(
            (cache.spacing_at(1680) - 1.25).abs() < 1e-9,
            "interpolation at beat 7"
        );
        assert!((cache.spacing_at(1920) - 1.0).abs() < 1e-9, "keyframe 03");
    }
}
