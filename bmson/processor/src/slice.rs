//! Sound channel slicing algorithm.
//!
//! Each [`SoundChannel`](bmsrs_bmson_def::SoundChannel) bundles one audio file with all
//! of its [`NoteEvent`]s. The processor precomputes audio slices so the player
//! can look them up by index at runtime and get an [`AudioAsset`] with its
//! length already set.
//!
//! # Algorithm
//!
//! 1. Collect the unique tick (`y`) positions from all note events.
//! 2. Sort them in ascending order.
//! 3. For each tick, decide whether it is a **restart point**: if *any* note at
//!    that tick has `c: false`, it is a restart (mixed `c` flags at the same tick
//!    count as a restart).
//! 4. Convert each tick to chart time through [`TimingTrack`].
//! 5. Create one [`AudioAsset`] per tick *P<sub>i</sub>*, with `start` determined
//!    by the continuation flag:
//!    - **Restart** (`c: false`): `start = 0` (play from the beginning of the
//!      file).
//!    - **Continuation** (`c: true`): `start` is the chart time at
//!      *P<sub>i</sub>* minus the chart time at the most recent restart point
//!      (play from where the audio would be if it had never restarted).
//! 6. `duration` is the chart-time difference to the next tick; the last slice
//!    uses `None`.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use bmsrs_bmson_def::SoundChannel;
use bmsrs_chart::{AudioAsset, TimingTrack};

/// Output of slicing one sound channel.
///
/// `pulse_to_index` maps a note event tick to the index of its [`AudioAsset`] in
/// this slice's `assets` vector.
pub struct SlicedChannel {
    /// Audio assets, ordered by ascending tick.
    pub assets: Vec<AudioAsset>,
    /// Tick to audio-asset index lookup table.
    pub pulse_to_index: BTreeMap<u64, usize>,
}

/// Slice one sound channel into precomputed [`AudioAsset`]s.
///
/// See the [module documentation](self) for the algorithm. `timing` is a
/// precomputed [`TimingTrack`] providing O(log n) tick-to-musical-time
/// conversion (compared with the O(n) scan of
/// [`TimingTrack::tick_to_musical_duration`], this reduces the overall
/// complexity from O(N*M) to O(M*log N) on tick-dense channels).
///
/// [`TimingTrack`]: bmsrs_chart::TimingTrack
/// [`TimingTrack::tick_to_musical_duration`]: bmsrs_chart::TimingTrack::tick_to_musical_duration
pub fn slice_channel(
    channel: &SoundChannel<'_>,
    timing: &TimingTrack,
    name: &Path,
) -> SlicedChannel {
    // 1. Collect unique tick positions. BTreeSet iteration is already ascending,
    //    so no second sort is needed.
    let pulses: Vec<u64> = channel
        .note_events
        .iter()
        .map(|n| n.y)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // 2. Mark restart points: ticks where *any* note has c:false.
    //    (Per the specification, mixed c flags at the same tick mean a restart.)
    let restart_pulses: BTreeSet<u64> = channel
        .note_events
        .iter()
        .filter(|n| !n.c)
        .map(|n| n.y)
        .collect();

    // 3. Convert each tick to chart time.
    let pulse_times: Vec<(u64, Duration)> = pulses
        .iter()
        .map(|&p| (p, timing.tick_to_musical_duration(p)))
        .collect();

    // 4. Build AudioAssets with the correct audio start offsets.
    let mut assets = Vec::with_capacity(pulse_times.len());
    let mut pulse_to_index = BTreeMap::new();
    // Shared path: every slice in the channel references the same Arc, only
    // incrementing the refcount.
    let shared_path = Arc::from(name.to_path_buf());
    // Record the chart time of the most recent restart point.
    let mut last_restart_time = Duration::ZERO;

    for (i, &(pulse, chart_time)) in pulse_times.iter().enumerate() {
        // The first tick always counts as a restart (there is no prior audio
        // context to continue).
        let is_restart = i == 0 || restart_pulses.contains(&pulse);

        let audio_start = if is_restart {
            Duration::ZERO
        } else {
            chart_time.saturating_sub(last_restart_time)
        };

        if is_restart {
            last_restart_time = chart_time;
        }

        let duration = pulse_times
            .get(i + 1)
            .map(|&(_, next_time)| next_time.saturating_sub(chart_time));

        let asset = AudioAsset {
            path: Arc::clone(&shared_path),
            start: audio_start,
            duration,
        };
        pulse_to_index.insert(pulse, assets.len());
        assets.push(asset);
    }

    SlicedChannel {
        assets,
        pulse_to_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmsrs_bmson_def::NoteEvent;
    use bmsrs_chart::{StopEvent, TimingTrack};
    use std::path::Path;
    use std::time::Duration;

    const RES: u64 = 240;

    fn timing_120() -> TimingTrack {
        TimingTrack::simple(120.0, RES).unwrap()
    }

    #[test]
    fn three_unique_pulses_produce_three_assets() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 2,
                    y: 240,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 480,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120(), channel.name);

        assert_eq!(result.assets.len(), 3);
        assert_eq!(result.pulse_to_index.len(), 3);
        // Tick 0: first tick means restart, so start=0.
        assert!(result.assets[0].start.is_zero());
        assert_eq!(result.assets[0].duration, Some(Duration::from_millis(500)));
        // Tick 240: c:true continues from the latest restart (0), so
        // start = chart(240) - chart(0) = 0.5s.
        assert_eq!(result.assets[1].start, Duration::from_millis(500));
        assert_eq!(result.assets[1].duration, Some(Duration::from_millis(500)));
        // Tick 480: c:false means restart, so start=0.
        assert!(result.assets[2].start.is_zero());
        assert!(result.assets[2].duration.is_none());
    }

    #[test]
    fn stop_between_continuation_and_next_slice_excludes_pause() {
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
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 480,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing, channel.name);

        assert_eq!(result.assets[1].duration, Some(Duration::from_millis(500)));
    }

    #[test]
    fn duplicate_pulse_deduplicated() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 2,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120(), channel.name);

        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.pulse_to_index.get(&240), Some(&0));
        // First tick means restart, so start=0.
        assert!(result.assets[0].start.is_zero());
    }

    #[test]
    fn first_slice_starts_at_zero() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![NoteEvent {
                x: 1,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            }],
        };

        let result = slice_channel(&channel, &timing_120(), channel.name);

        assert!(result.assets[0].start.is_zero());
        assert!(result.assets[0].duration.is_none());
    }

    #[test]
    fn intermediate_slice_duration_is_gap() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120(), channel.name);

        // 120 BPM at resolution 240: tick 0 maps to 0s, tick 240 to 0.5s.
        assert!(result.assets[0].start.is_zero());
        let d0 = result.assets[0].duration.expect("first slice has duration");
        assert_eq!(d0, Duration::from_millis(500));
        // The second tick is a restart, so start=0.
        assert!(result.assets[1].start.is_zero());
    }

    #[test]
    fn last_slice_duration_is_none() {
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 480,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120(), channel.name);

        assert!(result.assets.last().is_some_and(|a| a.duration.is_none()));
    }

    #[test]
    fn continuation_pulse_offsets_from_last_restart() {
        // Tick sequence: 0 (c:false, restart), 240 (c:true, continuation),
        // 480 (c:true, continuation), 720 (c:false, restart), 840 (c:true,
        // continuation).
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 0,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 480,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 720,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 1,
                    y: 840,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120(), channel.name);

        // 5 unique ticks give 5 audio assets.
        assert_eq!(result.assets.len(), 5);

        // Asset 0 (tick 0, restart): start=0.
        assert!(result.assets[0].start.is_zero());
        // Asset 1 (tick 240, c:true): offset from the latest restart (0),
        // 0.5s.
        assert_eq!(result.assets[1].start, Duration::from_millis(500));
        // Asset 2 (tick 480, c:true): offset from the latest restart (0), 1.0s.
        assert_eq!(result.assets[2].start, Duration::from_secs(1));
        // Asset 3 (tick 720, restart): start=0.
        assert!(result.assets[3].start.is_zero());
        // Asset 4 (tick 840, c:true): offset from the latest restart (720),
        // chart(840) - chart(720) = 0.25s.
        assert_eq!(result.assets[4].start, Duration::from_millis(250));
    }

    #[test]
    fn mixed_c_at_same_pulse_treated_as_restart() {
        // Two notes at tick 240: one c:true and one c:false, so it counts as a
        // restart.
        let channel = SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                NoteEvent {
                    x: 1,
                    y: 240,
                    l: 0,
                    c: true,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
                NoteEvent {
                    x: 2,
                    y: 240,
                    l: 0,
                    c: false,
                    t: None,
                    up: None,
                    ln_type_hint: None,
                    ln_judge_hint: None,
                    ln_life_hint: None,
                    vol: None,
                    pan: None,
                },
            ],
        };

        let result = slice_channel(&channel, &timing_120(), channel.name);
        // The first tick always counts as a restart.
        assert!(result.assets[0].start.is_zero());
    }
}
