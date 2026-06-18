//! BMSON → `Chart` conversion processor.
//!
//! [`BmsonProcessor`] converts a `bmson_def::Bmson` (v2 root schema) into
//! a format-agnostic `Chart` by applying a [`BmsonLayout`] mode family
//! ([`Beat`], [`Pms`], or [`GenericLayout`] for n-keys).
//!
//! Mode families and the [`BmsonLayout`] trait live in the [`layout`] module.
//!
//! # Pipeline
//!
//! ```text
//! bmson_def::Bmson → BmsonProcessor::process(bmson, layout) → Chart<NoteData>
//! ```
//!
//! For v0/v1 files, convert to the root schema first via `Bmson::from`.
//!
//! # Slicing
//!
//! Each `bmson_def::SoundChannel` is sliced into pre-computed
//! `AudioAsset`s at every unique note pulse (see the internal `slice`
//! module for details).

mod slice;

pub mod layout;

use std::collections::BTreeSet;
use std::time::Duration;

use bmson_def::{BGAEvent, BpmEvent, ModeHint, StopEvent as BmsonStopEvent};
use bmsrs_chart::{
    AudioAsset, BarLine, Bga, BgaResource, BgaTimelineEvent, BgmEvent, BpmChange, Chart,
    ChartMetadata, Note, NoteData, NoteDataLike, NoteKind, ScrollChangeEvent, StopEvent,
    TimingTrack,
};
use thiserror::Error;

use crate::layout::{Beat, BmsonLayout, GenericLayout, Pms};

use crate::slice::slice_channel;

/// Errors that can occur during BMSON processing.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// `init_bpm` must be strictly positive.
    #[error("init_bpm must be positive, got {0}")]
    InvalidBpm(f64),
}

/// Zero-sized processor that converts [`bmson_def::Bmson`] into [`Chart`].
///
/// Call [`process`](Self::process) with a mode-family layout, or
/// [`process_default`](Self::process_default) to select one from `mode_hint`.
pub struct BmsonProcessor;

impl BmsonProcessor {
    /// Process a BMSON chart with a stateless mode-family layout.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if `init_bpm` is not positive.
    pub fn process<L>(bmson: &bmson_def::Bmson<'_>) -> Result<Chart<NoteData>, ProcessError>
    where
        L: BmsonLayout,
    {
        Self::process_body(bmson, &L::map_x)
    }

    /// Process a BMSON chart with a generic-nkeys layout.
    ///
    /// This is the only stateful layout family — call this directly instead
    /// of [`process`](Self::process) when the mode hint is `generic-nkeys`.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if `init_bpm` is not positive.
    pub fn process_nkeys(
        bmson: &bmson_def::Bmson<'_>,
        keys: u16,
    ) -> Result<Chart<NoteData>, ProcessError> {
        let layout = GenericLayout { keys };
        Self::process_body(bmson, &|x| layout.map_x(x))
    }

    /// Process a BMSON chart, selecting the mode family from `mode_hint`.
    ///
    /// `beat-*` and `dj-*` hints map to [`Beat`]; `popn-*` to [`Pms`];
    /// `generic-nkeys` to [`GenericLayout`] with the given key count; anything
    /// else falls back to [`Beat`] (the BMSON default).
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if `init_bpm` is not positive.
    pub fn process_default(bmson: &bmson_def::Bmson<'_>) -> Result<Chart<NoteData>, ProcessError> {
        match bmson.chart_data.mode_hint {
            ModeHint::Popn5k | ModeHint::Popn9k => Self::process::<Pms>(bmson),
            ModeHint::Generic(n) => {
                let keys = u16::try_from(n).unwrap_or(0);
                Self::process_nkeys(bmson, keys)
            }
            // All beat-* and dj-* variants, plus Other/unknown → Beat.
            _ => Self::process::<Beat>(bmson),
        }
    }

    /// Public entry point for custom decode logic (internal use).
    fn process_body(
        bmson: &bmson_def::Bmson<'_>,
        decode: &impl Fn(u64) -> Option<NoteData>,
    ) -> Result<Chart<NoteData>, ProcessError> {
        let data = &bmson.chart_data;

        if data.init_bpm <= 0.0 {
            return Err(ProcessError::InvalidBpm(data.init_bpm));
        }

        let timing = build_timing(data);
        let resolution = data.resolution;
        let playable_pulses = collect_playable_pulses(&data.sound_channels);

        let (mut audio_assets, mut notes, bgm) = process_sound_channels(
            &data.sound_channels,
            decode,
            &timing,
            resolution,
            &playable_pulses,
        );

        process_mine_channels(&bmson.mine_channels, decode, &mut audio_assets, &mut notes);
        process_key_channels(&bmson.key_channels, decode, &mut audio_assets, &mut notes);

        notes.sort_by_key(|n| n.tick);

        let bar_lines = build_bar_lines(data.lines.as_deref(), resolution, &notes, &bgm);
        let scroll_events = build_scroll_events(&bmson.scroll_events);
        let bga = build_bga(&bmson.chart_info.bga);
        let metadata = build_metadata(bmson);

        Ok(Chart {
            metadata,
            resolution,
            timing,
            judge_multiplier: data.judge_multiplier,
            life_multiplier: data.life_multiplier,
            notes,
            bgm,
            audio_assets,
            bar_lines,
            scroll_events,
            bga,
        })
    }
}

/// Collect all pulse positions that have at least one playable note.
fn collect_playable_pulses(channels: &[bmson_def::SoundChannel<'_>]) -> BTreeSet<u64> {
    channels
        .iter()
        .flat_map(|ch| ch.note_events.iter())
        .filter(|n| n.x > 0)
        .map(|n| n.y)
        .collect()
}

/// Build [`TimingTrack`] from BMSON chart data.
fn build_timing(data: &bmson_def::ChartData<'_>) -> TimingTrack {
    TimingTrack {
        init_bpm: data.init_bpm,
        bpm_changes: data.bpm_events.iter().map(build_bpm_change).collect(),
        stops: data.stop_events.iter().map(build_stop_event).collect(),
    }
}

/// Process sound channels: slicing, note creation, BGM events.
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_sound_channels(
    channels: &[bmson_def::SoundChannel<'_>],
    decode: &impl Fn(u64) -> Option<NoteData>,
    timing: &TimingTrack,
    resolution: u64,
    playable_pulses: &BTreeSet<u64>,
) -> (Vec<AudioAsset>, Vec<Note<NoteData>>, Vec<BgmEvent>) {
    let mut audio_assets = Vec::new();
    let mut notes = Vec::new();
    let mut bgm = Vec::new();

    for channel in channels {
        let sliced = slice_channel(channel, timing, resolution);

        for ne in &channel.note_events {
            let audio_idx = sliced
                .pulse_to_index
                .get(&ne.y)
                .copied()
                .map(|idx| (idx + audio_assets.len()) as u32);

            if ne.is_bgm() {
                if playable_pulses.contains(&ne.y) {
                    continue;
                }
                if let Some(idx) = audio_idx {
                    bgm.push(BgmEvent {
                        tick: ne.y,
                        audio: idx,
                    });
                }
            } else {
                let Some(nd) = decode(ne.x) else {
                    continue;
                };
                notes.push(Note {
                    tick: ne.y,
                    audio: audio_idx,
                    data: NoteData {
                        kind: if ne.l > 0 {
                            NoteKind::Long { duration: ne.l }
                        } else {
                            NoteKind::Normal
                        },
                        ..nd
                    },
                });
            }
        }

        audio_assets.extend(sliced.assets);
    }

    (audio_assets, notes, bgm)
}

/// Process mine channels: each channel contributes one whole-file `AudioAsset`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_mine_channels(
    channels: &[bmson_def::MineChannel<'_>],
    decode: &impl Fn(u64) -> Option<NoteData>,
    audio_assets: &mut Vec<AudioAsset>,
    notes: &mut Vec<Note<NoteData>>,
) {
    for mc in channels {
        let mine_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: mc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for mn in &mc.notes {
            let Some(nd) = decode(mn.x) else {
                continue;
            };
            notes.push(Note {
                tick: mn.y,
                audio: Some(mine_audio_idx),
                data: NoteData {
                    kind: NoteKind::Mine { damage: mn.damage },
                    ..nd
                },
            });
        }
    }
}

/// Process key (invisible) channels: same structure as mine channels.
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_key_channels(
    channels: &[bmson_def::KeyChannel<'_>],
    decode: &impl Fn(u64) -> Option<NoteData>,
    audio_assets: &mut Vec<AudioAsset>,
    notes: &mut Vec<Note<NoteData>>,
) {
    for kc in channels {
        let key_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: kc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for kn in &kc.notes {
            let Some(nd) = decode(kn.x) else {
                continue;
            };
            notes.push(Note {
                tick: kn.y,
                audio: Some(key_audio_idx),
                data: NoteData {
                    kind: NoteKind::Invisible,
                    ..nd
                },
            });
        }
    }
}

/// Build [`ChartMetadata`] from BMSON song/chart info.
fn build_metadata(bmson: &bmson_def::Bmson<'_>) -> ChartMetadata {
    ChartMetadata {
        title: bmson.song_info.title.to_owned(),
        subtitle: bmson.chart_info.subtitle.to_owned(),
        artist: bmson.song_info.artist.to_owned(),
        subartists: bmson
            .chart_info
            .subartists
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        genre: bmson.song_info.genre.to_owned(),
        chart_name: bmson.chart_info.chart_name.to_owned(),
        level: bmson.chart_info.level,
    }
}

/// Build a [`BpmChange`] from a BMSON [`BpmEvent`].
const fn build_bpm_change(e: &BpmEvent) -> BpmChange {
    BpmChange {
        tick: e.y,
        bpm: e.bpm,
    }
}

/// Build a [`StopEvent`] from a BMSON stop event.
const fn build_stop_event(e: &BmsonStopEvent) -> StopEvent {
    StopEvent {
        tick: e.y,
        duration: e.duration,
    }
}

/// Build scroll-speed change events from BMSON scroll events.
fn build_scroll_events(events: &[bmson_def::ScrollEvent]) -> Vec<ScrollChangeEvent> {
    events
        .iter()
        .map(|e| ScrollChangeEvent {
            tick: e.y,
            rate: e.rate,
        })
        .collect()
}

/// Build bar lines from the BMSON `lines` field.
///
/// `None` → auto-generate 4/4 bar lines (every `resolution * 4` pulses)
/// from 0 to the last event tick.
fn build_bar_lines(
    lines: Option<&[bmson_def::BarLine]>,
    resolution: u64,
    notes: &[Note<impl NoteDataLike>],
    bgm: &[BgmEvent],
) -> Vec<BarLine> {
    if let Some(vec) = lines {
        return vec.iter().map(|bl| BarLine { tick: bl.y }).collect();
    }

    // Auto-generate 4/4 bar lines.
    let last_tick = notes
        .last()
        .map_or(0, |n| n.tick)
        .max(bgm.last().map_or(0, |e| e.tick));
    let step = resolution * 4;
    let count = step.checked_div(step).map_or(1, |_| last_tick / step + 1);
    (0..=count).map(|i| BarLine { tick: i * step }).collect()
}

/// Build [`Bga`] from BMSON BGA data.
#[expect(clippy::cast_possible_truncation, reason = "BGA ids fit in u32")]
fn build_bga(bga: &bmson_def::BGA<'_>) -> Bga {
    Bga {
        resources: bga
            .bga_header
            .iter()
            .map(|h| BgaResource {
                id: h.id as u32,
                path: h.name.to_path_buf(),
            })
            .collect(),
        events: bga.bga_events.iter().map(build_bga_event).collect(),
        layer_events: bga.layer_events.iter().map(build_bga_event).collect(),
        poor_events: bga.poor_events.iter().map(build_bga_event).collect(),
    }
}

/// Build a [`BgaTimelineEvent`] from a BMSON [`BGAEvent`].
#[expect(clippy::cast_possible_truncation, reason = "BGA ids fit in u32")]
const fn build_bga_event(e: &BGAEvent) -> BgaTimelineEvent {
    BgaTimelineEvent {
        tick: e.y,
        resource_id: e.id as u32,
    }
}
