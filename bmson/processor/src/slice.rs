//! Sound-channel slicing algorithm.
//!
//! Each [`SoundChannel`](bmson_def::SoundChannel) bundles one audio file
//! with all its [`NoteEvent`]s. The processor pre-computes audio slices
//! so that the player can simply look up a pre-sized [`AudioAsset`] by
//! index at runtime.
//!
//! # Algorithm
//!
//! 1. Collect unique pulse (`y`) positions from all note events.
//! 2. Sort ascending.
//! 3. For each pulse, determine whether it is a **restart point**:
//!    a pulse is a restart if *any* note at that pulse has
//!    `c: false` (mixed `c` flags at the same pulse → treat as restart).
//! 4. Convert each pulse to chart time via [`TimingTrack`].
//! 5. For each pulse *P<sub>i</sub>* create an [`AudioAsset`] whose
//!    `start` depends on the continuation flag:
//!    - **Restart** (`c: false`): `start = 0` (play from beginning of file).
//!    - **Continue** (`c: true`): `start` = chart time at *P<sub>i</sub>*
//!      minus chart time at the most recent restart point (play from where
//!      the audio would be without restarting).
//! 6. `duration` is the chart-time difference to the next pulse, or `None`
//!    for the final slice.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Duration;

use bmson_def::SoundChannel;
use bmsrs_chart::{AudioAsset, TimingTrack};

/// Output of slicing one sound channel.
///
/// `pulse_to_index` maps a note-event pulse to the index of its
/// [`AudioAsset`] within the slice's `assets` vector.
pub struct SlicedChannel {
    /// Audio assets in pulse-ascending order.
    pub assets: Vec<AudioAsset>,
    /// Pulse → asset-index lookup.
    pub pulse_to_index: BTreeMap<u64, usize>,
}

/// Slice a sound channel into pre-computed [`AudioAsset`]s.
///
/// See the [module docs](self) for the algorithm.
pub fn slice_channel(
    channel: &SoundChannel<'_>,
    timing: &TimingTrack,
    resolution: u64,
) -> SlicedChannel {
    // 1. Collect unique pulse positions in ascending order.
    let mut pulses: Vec<u64> = channel
        .note_events
        .iter()
        .map(|n| n.y)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    pulses.sort_unstable();

    // 2. Identify restart points: pulses where *any* note has c:false.
    //    (Mixed c flags at the same pulse → treat as restart, per spec.)
    let restart_pulses: BTreeSet<u64> = channel
        .note_events
        .iter()
        .filter(|n| !n.c)
        .map(|n| n.y)
        .collect();

    // 3. Convert each pulse to chart time.
    let pulse_times: Vec<(u64, Duration)> = pulses
        .iter()
        .map(|&p| (p, timing.tick_to_duration(p, resolution)))
        .collect();

    // 4. Build AudioAssets with correct audio-start offsets.
    let mut assets = Vec::with_capacity(pulse_times.len());
    let mut pulse_to_index = BTreeMap::new();
    // Tracks chart time of the most recent restart point.
    let mut last_restart_time = Duration::ZERO;

    for (i, &(pulse, chart_time)) in pulse_times.iter().enumerate() {
        // The first pulse is always treated as a restart (no prior audio
        // context to continue from).
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
            path: channel.name.to_path_buf(),
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
    use bmson_def::NoteEvent;
    use std::path::Path;

    const RES: u64 = 240;

    fn timing_120() -> TimingTrack {
        TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        }
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

        let result = slice_channel(&channel, &timing_120(), RES);

        assert_eq!(result.assets.len(), 3);
        assert_eq!(result.pulse_to_index.len(), 3);
        // Pulse 0: first pulse → restart → start=0
        assert!(result.assets[0].start.is_zero());
        assert_eq!(result.assets[0].duration, Some(Duration::from_millis(500)));
        // Pulse 240: c:true → continue from last restart (0) → start = chart(240) - chart(0) = 0.5s
        assert_eq!(result.assets[1].start, Duration::from_millis(500));
        assert_eq!(result.assets[1].duration, Some(Duration::from_millis(500)));
        // Pulse 480: c:false → restart → start=0
        assert!(result.assets[2].start.is_zero());
        assert!(result.assets[2].duration.is_none());
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

        let result = slice_channel(&channel, &timing_120(), RES);

        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.pulse_to_index.get(&240), Some(&0));
        // First pulse → restart → start=0
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

        let result = slice_channel(&channel, &timing_120(), RES);

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

        let result = slice_channel(&channel, &timing_120(), RES);

        // At 120 BPM, resolution 240: pulse 0→0s, pulse 240→0.5s
        assert!(result.assets[0].start.is_zero());
        let d0 = result.assets[0].duration.expect("first slice has duration");
        assert_eq!(d0, Duration::from_millis(500));
        // Second pulse is a restart → start=0
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

        let result = slice_channel(&channel, &timing_120(), RES);

        assert!(result.assets.last().is_some_and(|a| a.duration.is_none()));
    }

    #[test]
    fn continuation_pulse_offsets_from_last_restart() {
        // Pulses at 0 (c:false, restart), 240 (c:true, continue),
        // 480 (c:true, continue), 720 (c:false, restart), 840 (c:true, continue).
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

        let result = slice_channel(&channel, &timing_120(), RES);

        // 5 unique pulses → 5 assets
        assert_eq!(result.assets.len(), 5);

        // Asset 0 (pulse 0, restart): start=0
        assert!(result.assets[0].start.is_zero());
        // Asset 1 (pulse 240, c:true): offset from last restart (0) → 0.5s
        assert_eq!(result.assets[1].start, Duration::from_millis(500));
        // Asset 2 (pulse 480, c:true): offset from last restart (0) → 1.0s
        assert_eq!(result.assets[2].start, Duration::from_secs(1));
        // Asset 3 (pulse 720, restart): start=0
        assert!(result.assets[3].start.is_zero());
        // Asset 4 (pulse 840, c:true): offset from last restart (720) → chart(840) - chart(720) = 0.25s
        assert_eq!(result.assets[4].start, Duration::from_millis(250));
    }

    #[test]
    fn mixed_c_at_same_pulse_treated_as_restart() {
        // Two notes at pulse 240: one c:true, one c:false → treat as restart.
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

        let result = slice_channel(&channel, &timing_120(), RES);
        // First pulse → always treated as restart
        assert!(result.assets[0].start.is_zero());
    }
}
