//! BMSON → `Chart` conversion processor.
//!
//! [`BmsonProcessor`] converts a `bmson_def::Bmson` (v2 root schema) into
//! a format-agnostic `Chart<BmsonNoteExt>` by applying a [`BmsonLayout`] mode
//! family ([`Beat`], [`Pms`], or [`GenericLayout`] for n-keys).
//!
//! Mode families and the [`BmsonLayout`] trait live in the [`layout`] module.
//!
//! # Pipeline
//!
//! ```text
//! bmson_def::Bmson → BmsonProcessor::process(bmson, layout) → Chart<BmsonNoteExt>
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

use bmson_def::{BpmEvent, StopEvent as BmsonStopEvent};
use bmsrs_chart::{
    AudioAsset, BgaLayer, BgaResource, BpmChange, Chart, ChartData, ChartInfo, Event, Lane,
    NoteExt, NoteKind, NoteSide, SongInfo, StopEvent, TimingTrack,
};
use thiserror::Error;

use crate::layout::{Beat, BmsonLayout, GenericLayout, Pms};

use crate::slice::slice_channel;

/// Per-note extension data for BMSON format.
///
/// Carries optional fields from `bmson_def::NoteEvent` that are not part
/// of the core chart model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BmsonNoteExt {
    /// Note volume (percent, DJ.NEXT extension).
    pub vol: Option<i8>,
    /// Note pan (DJ.NEXT extension).
    pub pan: Option<i8>,
    /// Release-sound / BSS flag (bmson `up`).
    pub release_sound: Option<bool>,
    /// beatoraja long-note mode (bmson `t`, 1=LN/2=CN/3=HCN).
    pub beatoraja_ln_mode: Option<u64>,
    /// Per-note LN type hint override (bmson v2).
    pub ln_type_hint: Option<String>,
    /// Per-note LN judgement hint override (bmson v2).
    pub ln_judge_hint: Option<String>,
    /// Per-note LN life hint override (bmson v2).
    pub ln_life_hint: Option<String>,
}

impl NoteExt for BmsonNoteExt {}

/// Errors that can occur during BMSON processing.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// `init_bpm` must be strictly positive.
    #[error("init_bpm must be positive, got {0}")]
    InvalidBpm(f64),
}

/// Zero-sized processor that converts [`bmson_def::Bmson`] into
/// [`Chart<BmsonNoteExt>`].
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
    pub fn process<L>(bmson: &bmson_def::Bmson<'_>) -> Result<Chart<BmsonNoteExt>, ProcessError>
    where
        L: BmsonLayout,
    {
        Self::process_body(bmson, &|x| L::map_x(x))
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
    ) -> Result<Chart<BmsonNoteExt>, ProcessError> {
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
    pub fn process_default(
        bmson: &bmson_def::Bmson<'_>,
    ) -> Result<Chart<BmsonNoteExt>, ProcessError> {
        match bmson.chart_data.mode_hint {
            bmson_def::ModeHint::Popn5k | bmson_def::ModeHint::Popn9k => {
                Self::process::<Pms>(bmson)
            }
            bmson_def::ModeHint::Generic(n) => {
                let keys = u16::try_from(n).unwrap_or(0);
                Self::process_nkeys(bmson, keys)
            }
            _ => Self::process::<Beat>(bmson),
        }
    }

    /// Public entry point for custom decode logic (internal use).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "BGA header/event ids are in the u32 range for practical charts"
    )]
    fn process_body(
        bmson: &bmson_def::Bmson<'_>,
        decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    ) -> Result<Chart<BmsonNoteExt>, ProcessError> {
        let data = &bmson.chart_data;

        if data.init_bpm <= 0.0 {
            return Err(ProcessError::InvalidBpm(data.init_bpm));
        }

        let timing = build_timing(data);
        let resolution = data.resolution;
        let playable_pulses = collect_playable_pulses(&data.sound_channels);

        let (mut audio_assets, mut events) = process_sound_channels(
            &data.sound_channels,
            decode,
            &timing,
            resolution,
            &playable_pulses,
        );

        process_mine_channels(&bmson.mine_channels, decode, &mut audio_assets, &mut events);
        process_key_channels(&bmson.key_channels, decode, &mut audio_assets, &mut events);

        events.extend(data.bpm_events.iter().map(|e| Event::Bpm {
            tick: e.y,
            bpm: e.bpm,
        }));
        events.extend(data.stop_events.iter().map(|e| Event::Stop {
            tick: e.y,
            duration: e.duration,
        }));
        events.extend(bmson.scroll_events.iter().map(|e| Event::Scroll {
            tick: e.y,
            rate: e.rate,
        }));

        events.extend(build_bar_lines(data.lines.as_deref(), resolution, &events));

        let bga = &bmson.chart_info.bga;

        for e in &bga.bga_events {
            events.push(Event::Bga {
                tick: e.y,
                layer: BgaLayer::Base,
                resource_id: e.id as u32,
            });
        }
        for e in &bga.layer_events {
            events.push(Event::Bga {
                tick: e.y,
                layer: BgaLayer::Layer,
                resource_id: e.id as u32,
            });
        }
        for e in &bga.poor_events {
            events.push(Event::Bga {
                tick: e.y,
                layer: BgaLayer::Poor,
                resource_id: e.id as u32,
            });
        }

        events.sort_by_key(Event::tick);

        let song_info = build_song_info(bmson);
        let chart_info = build_chart_info(bmson);

        Ok(Chart {
            song: song_info,
            chart: chart_info,
            data: ChartData {
                resolution,
                timing,
                judge_multiplier: data.judge_multiplier,
                life_multiplier: data.life_multiplier,
                events,
                audio_assets,
            },
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

/// Process sound channels: slicing, note/BGM event creation.
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_sound_channels(
    channels: &[bmson_def::SoundChannel<'_>],
    decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    timing: &TimingTrack,
    resolution: u64,
    playable_pulses: &BTreeSet<u64>,
) -> (Vec<AudioAsset>, Vec<Event<BmsonNoteExt>>) {
    let mut audio_assets = Vec::new();
    let mut events = Vec::new();

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
                    events.push(Event::Bgm {
                        tick: ne.y,
                        audio_index: idx,
                    });
                }
            } else {
                let Some((side, lane)) = decode(ne.x) else {
                    continue;
                };
                let kind = if ne.l > 0 {
                    NoteKind::Long { duration: ne.l }
                } else {
                    NoteKind::Normal
                };
                events.push(Event::Note {
                    tick: ne.y,
                    side,
                    lane,
                    kind,
                    audio_index: audio_idx,
                    ext: build_note_ext(ne),
                });
            }
        }

        audio_assets.extend(sliced.assets);
    }

    (audio_assets, events)
}

/// Build [`BmsonNoteExt`] from a BMSON [`NoteEvent`](bmson_def::NoteEvent).
fn build_note_ext(ne: &bmson_def::NoteEvent) -> BmsonNoteExt {
    BmsonNoteExt {
        vol: ne.vol,
        pan: ne.pan,
        release_sound: ne.up,
        beatoraja_ln_mode: ne.t.map(ln_mode_to_u64),
        ln_type_hint: ne.ln_type_hint.map(ln_type_to_str),
        ln_judge_hint: ne.ln_judge_hint.map(ln_judge_to_str),
        ln_life_hint: ne.ln_life_hint.map(ln_life_to_str),
    }
}

/// Convert a `LnMode` discriminant to a numeric beatoraja LN-mode value.
const fn ln_mode_to_u64(m: bmson_def::LnMode) -> u64 {
    match m {
        bmson_def::LnMode::Cn => 2,
        bmson_def::LnMode::Hcn => 3,
        _ => 1,
    }
}

/// Convert `LnType` to its bmson v2 string representation.
fn ln_type_to_str(lt: bmson_def::LnType) -> String {
    match lt {
        bmson_def::LnType::Cn => "cn",
        _ => "ln",
    }
    .to_owned()
}

/// Convert `LnJudge` to its bmson v2 string representation.
fn ln_judge_to_str(lj: bmson_def::LnJudge) -> String {
    match lj {
        bmson_def::LnJudge::Ticks => "ticks",
        _ => "normal",
    }
    .to_owned()
}

/// Convert `LnLife` to its bmson v2 string representation.
fn ln_life_to_str(ll: bmson_def::LnLife) -> String {
    match ll {
        bmson_def::LnLife::Ticks => "ticks",
        _ => "normal",
    }
    .to_owned()
}

/// Process mine channels: each channel contributes one whole-file `AudioAsset`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_mine_channels(
    channels: &[bmson_def::MineChannel<'_>],
    decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    audio_assets: &mut Vec<AudioAsset>,
    events: &mut Vec<Event<BmsonNoteExt>>,
) {
    for mc in channels {
        let mine_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: mc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for mn in &mc.notes {
            let Some((side, lane)) = decode(mn.x) else {
                continue;
            };
            events.push(Event::Note {
                tick: mn.y,
                side,
                lane,
                kind: NoteKind::Mine { damage: mn.damage },
                audio_index: Some(mine_audio_idx),
                ext: BmsonNoteExt::default(),
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
    decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    audio_assets: &mut Vec<AudioAsset>,
    events: &mut Vec<Event<BmsonNoteExt>>,
) {
    for kc in channels {
        let key_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: kc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for kn in &kc.notes {
            let Some((side, lane)) = decode(kn.x) else {
                continue;
            };
            events.push(Event::Note {
                tick: kn.y,
                side,
                lane,
                kind: NoteKind::Invisible,
                audio_index: Some(key_audio_idx),
                ext: BmsonNoteExt::default(),
            });
        }
    }
}

/// Build [`SongInfo`] from BMSON song info.
fn build_song_info(bmson: &bmson_def::Bmson<'_>) -> SongInfo {
    SongInfo {
        title: bmson.song_info.title.to_owned(),
        artist: bmson.song_info.artist.to_owned(),
        genre: bmson.song_info.genre.to_owned(),
        subartists: bmson
            .chart_info
            .subartists
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
    }
}

/// Build [`ChartInfo`] from BMSON chart info.
fn build_chart_info(bmson: &bmson_def::Bmson<'_>) -> ChartInfo {
    let bga = &bmson.chart_info.bga;
    let bga_resources: Vec<BgaResource> = bga
        .bga_header
        .iter()
        .map(|h| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "BGA header count fits in u32"
            )]
            BgaResource {
                id: h.id as u32,
                path: h.name.to_path_buf(),
            }
        })
        .collect();

    ChartInfo {
        subtitle: bmson.chart_info.subtitle.to_owned(),
        chart_name: bmson.chart_info.chart_name.to_owned(),
        level: bmson.chart_info.level,
        back_image: bmson
            .chart_info
            .back_image
            .map(|p| p.to_string_lossy().into_owned()),
        eyecatch_image: bmson
            .chart_info
            .eyecatch_image
            .map(|p| p.to_string_lossy().into_owned()),
        banner_image: bmson
            .chart_info
            .banner_image
            .map(|p| p.to_string_lossy().into_owned()),
        preview_music: bmson
            .chart_info
            .preview_music
            .map(|p| p.to_string_lossy().into_owned()),
        bga_resources,
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

/// Build bar line events from the BMSON `lines` field.
///
/// `None` → auto-generate 4/4 bar lines (every `resolution * 4` pulses)
/// from 0 to the last event tick.
fn build_bar_lines(
    lines: Option<&[bmson_def::BarLine]>,
    resolution: u64,
    events: &[Event<BmsonNoteExt>],
) -> Vec<Event<BmsonNoteExt>> {
    if let Some(vec) = lines {
        return vec.iter().map(|bl| Event::Bar { tick: bl.y }).collect();
    }

    let last_tick = events.last().map_or(0, Event::tick);
    let step = resolution * 4;
    let count = last_tick / step + 1;
    (0..=count).map(|i| Event::Bar { tick: i * step }).collect()
}
