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
//! 3. Convert each pulse to wall-clock seconds via [`TimingTrack`].
//! 4. For each pulse *P<sub>i</sub>* create an [`AudioAsset`] whose
//!    `start` is the seconds value at *P<sub>i</sub>* and whose
//!    `duration` is the difference to the next pulse (or `None` for
//!    the final slice).

use std::collections::BTreeMap;

use bmson_def::SoundChannel;
use bmsrs_chart::{AudioAsset, TimingTrack};

/// Output of slicing one sound channel.
///
/// `pulse_to_index` maps a note-event pulse to the index of its
/// [`AudioAsset`] within the slice's `assets` vector.
pub(crate) struct SlicedChannel {
    /// Audio assets in pulse-ascending order.
    pub assets: Vec<AudioAsset>,
    /// Pulse → asset-index lookup.
    pub pulse_to_index: BTreeMap<u64, usize>,
}

/// Slice a sound channel into pre-computed [`AudioAsset`]s.
///
/// See the [module docs](self) for the algorithm.
pub(crate) fn slice_channel(
    channel: &SoundChannel<'_>,
    timing: &TimingTrack,
    resolution: u64,
) -> SlicedChannel {
    // Collect unique pulse positions in ascending order (BTreeSet deduplicates).
    let mut pulses: Vec<u64> = channel
        .note_events
        .iter()
        .map(|n| n.y)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    pulses.sort_unstable();

    // Convert each pulse to seconds.
    let pulse_secs: Vec<(u64, f64)> = pulses
        .iter()
        .map(|&p| (p, timing.tick_to_seconds(p, resolution)))
        .collect();

    // Build AudioAssets.
    let mut assets = Vec::with_capacity(pulse_secs.len());
    let mut pulse_to_index = BTreeMap::new();

    for (i, &(pulse, start)) in pulse_secs.iter().enumerate() {
        let duration = pulse_secs
            .get(i + 1)
            .map(|&(_, next_start)| next_start - start);
        let asset = AudioAsset {
            path: channel.name.to_path_buf(),
            start,
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

        assert!(result.assets[0].start.abs() < 1e-9);
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

        // At 120 BPM, resolution 240: pulse 0→0.0s, pulse 240→0.5s
        assert!((result.assets[0].start - 0.0).abs() < 1e-9);
        let d0 = result.assets[0].duration.expect("first slice has duration");
        assert!((d0 - 0.5).abs() < 1e-9);
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
}
