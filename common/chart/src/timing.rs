//! Timing track for tick ↔ Duration conversion.
//!
//! [`TimingTrack`] holds the initial BPM, BPM change events, and stop events.
//! It provides [`TimingTrack::tick_to_duration`] and [`TimingTrack::duration_to_tick`]
//! for converting between chart positions (ticks) and wall-clock time
//! ([`Duration`]).
//!
//! Internal computation uses `f64` arithmetic (BPM values are inherently
//! floating-point). The `f64` ↔ [`Duration`] conversion happens only at the
//! public API boundary via [`Duration::from_secs_f64`] and
//! [`Duration::as_secs_f64`].

use std::time::Duration;

/// Timing information for converting tick positions to wall-clock time.
///
/// All events are at absolute tick positions. The processor is responsible
/// for converting format-specific positions (BMSON pulses, BMS measures) to ticks.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingTrack {
    /// Initial BPM at tick 0.
    pub init_bpm: f64,
    /// BPM change events, sorted by tick ascending.
    pub bpm_changes: Vec<BpmChange>,
    /// Stop (pause) events, sorted by tick ascending.
    pub stops: Vec<StopEvent>,
}

/// A BPM change event.
#[derive(Clone, Debug, PartialEq)]
pub struct BpmChange {
    /// Tick position where the BPM changes.
    pub tick: u64,
    /// New BPM (beats per minute).
    pub bpm: f64,
}

/// A stop (pause) event.
///
/// When the playback reaches `tick`, the scroll halts for `duration` ticks
/// worth of time (at the current BPM). Multiple stops at the same tick
/// accumulate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopEvent {
    /// Tick where the stop begins.
    pub tick: u64,
    /// Duration in ticks.
    pub duration: u64,
}

/// Internal event representation for the merged timeline.
#[derive(Clone, Copy)]
enum TimingEvent {
    /// BPM change — `is_stop = false` ensures BPM sorts before Stop at the same tick.
    Bpm(f64),
    /// Stop with duration in ticks.
    Stop(u64),
}

impl TimingEvent {
    /// Returns `true` if this is a [`TimingEvent::Stop`].
    const fn is_stop(self) -> bool {
        matches!(self, Self::Stop(_))
    }
}

impl TimingTrack {
    /// Build a sorted event list from `bpm_changes` and stops.
    ///
    /// At the same tick, BPM changes sort before stops (per BMSON spec:
    /// "speed will first change, then the music pauses").
    fn build_events(&self) -> Vec<(u64, TimingEvent)> {
        let mut events: Vec<(u64, TimingEvent)> = Vec::new();
        for bc in &self.bpm_changes {
            events.push((bc.tick, TimingEvent::Bpm(bc.bpm)));
        }
        for st in &self.stops {
            events.push((st.tick, TimingEvent::Stop(st.duration)));
        }
        // Sort by tick, then BPM (false) before Stop (true).
        events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.is_stop().cmp(&b.1.is_stop())));
        events
    }

    /// Convert a tick position to wall-clock [`Duration`].
    ///
    /// At a tick with a stop event, the returned time is **before** the pause
    /// (notes at that tick are activated before the pause, per BMSON spec).
    /// Stops at ticks strictly before the target contribute their full pause time.
    ///
    /// # Panics (debug only)
    ///
    /// In debug builds, asserts `resolution > 0` and `init_bpm > 0`.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "larger function; compiler can decide inlining"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "lossless or explicitly-rounded numeric conversion"
    )]
    pub fn tick_to_duration(&self, tick: u64, resolution: u64) -> Duration {
        debug_assert!(resolution > 0, "resolution must be > 0");
        debug_assert!(self.init_bpm > 0.0, "init_bpm must be > 0");

        let res = resolution as f64;
        let events = self.build_events();

        let mut seconds = 0.0f64;
        let mut current_tick = 0u64;
        let mut current_bpm = self.init_bpm;

        for (event_tick, event) in &events {
            if *event_tick > tick {
                break;
            }
            // Advance playback to event_tick.
            if *event_tick > current_tick {
                let delta = (*event_tick - current_tick) as f64;
                seconds += delta / res * 60.0 / current_bpm;
                current_tick = *event_tick;
            }
            match event {
                TimingEvent::Bpm(bpm) => {
                    current_bpm = *bpm;
                }
                TimingEvent::Stop(duration) => {
                    // Stop time is only counted when the stop is strictly before
                    // the target tick. At the target tick itself, the time is
                    // before the pause (per spec).
                    if *event_tick < tick {
                        seconds += *duration as f64 / res * 60.0 / current_bpm;
                    }
                }
            }
        }

        // Remaining time from the last event to the target tick.
        if tick > current_tick {
            let delta = (tick - current_tick) as f64;
            seconds += delta / res * 60.0 / current_bpm;
        }

        Duration::from_secs_f64(seconds)
    }

    /// Convert wall-clock [`Duration`] to the nearest tick position.
    ///
    /// This is the inverse of [`tick_to_duration`](Self::tick_to_duration).
    /// Time spent in stops does not advance the tick.
    ///
    /// # Panics (debug only)
    ///
    /// In debug builds, asserts `resolution > 0` and `init_bpm > 0`.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "tick values fit in f64 mantissa for practical chart lengths"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded result is within u64 range"
    )]
    #[expect(clippy::cast_sign_loss, reason = "remaining time is non-negative")]
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "larger function; compiler can decide inlining"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "lossless or explicitly-rounded numeric conversion"
    )]
    pub fn duration_to_tick(&self, duration: Duration, resolution: u64) -> u64 {
        debug_assert!(resolution > 0, "resolution must be > 0");
        debug_assert!(self.init_bpm > 0.0, "init_bpm must be > 0");

        let seconds = duration.as_secs_f64();

        if seconds <= 0.0 {
            return 0;
        }

        let res = resolution as f64;
        let events = self.build_events();

        let mut remaining = seconds;
        let mut current_tick = 0u64;
        let mut current_bpm = self.init_bpm;

        for (event_tick, event) in &events {
            // Advance playback to event_tick.
            if *event_tick > current_tick {
                let delta_ticks = *event_tick - current_tick;
                let delta_seconds = delta_ticks as f64 / res * 60.0 / current_bpm;
                if delta_seconds >= remaining {
                    return current_tick + (remaining * res * current_bpm / 60.0).round() as u64;
                }
                remaining -= delta_seconds;
                current_tick = *event_tick;
            }

            match event {
                TimingEvent::Bpm(bpm) => {
                    current_bpm = *bpm;
                }
                TimingEvent::Stop(stop_duration) => {
                    let stop_seconds = *stop_duration as f64 / res * 60.0 / current_bpm;
                    if stop_seconds >= remaining {
                        // Target is within the stop — tick doesn't advance.
                        return current_tick;
                    }
                    remaining -= stop_seconds;
                }
            }
        }

        // Target is beyond all events.
        current_tick + (remaining * res * current_bpm / 60.0).round() as u64
    }
}
