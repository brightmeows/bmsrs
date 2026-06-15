//! BMSON → [`Chart`] conversion processor.
//!
//! [`BmsonProcessor`] converts a [`bmson_def::Bmson`] (v2 root schema) into
//! a format-agnostic [`Chart<T>`] by applying a [`BmsonMapping`] layout.
//!
//! # Pipeline
//!
//! ```text
//! bmson_def::Bmson → BmsonProcessor::process(bmson, layout) → Chart<L::NoteData>
//! ```
//!
//! For v0/v1 files, convert to the root schema first via `Bmson::from`.
//!
//! # Slicing
//!
//! Each [`bmson_def::SoundChannel`] is sliced into pre-computed
//! [`AudioAsset`]s at every unique note pulse (see the internal `slice` module).
//! for details.

mod slice;

use std::collections::BTreeSet;
use std::time::Duration;

use bmson_def::{BGAEvent, BpmEvent, ModeHint, NoteEvent, StopEvent as BmsonStopEvent};
use bmsrs_chart::{
    AudioAsset, BarLine, Beat5k, Beat7k, Beat10k, Beat14k, Bga, BgaResource, BgaTimelineEvent,
    BgmEvent, BpmChange, Chart, ChartMetadata, DefaultNoteData, GenericLayout, Layout, Note,
    NoteData, NoteKind, Popn5k, Popn9k, ScrollChangeEvent, StopEvent, TimingTrack,
};
use thiserror::Error;

use crate::slice::slice_channel;

/// Errors that can occur during BMSON processing.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// `init_bpm` must be strictly positive.
    #[error("init_bpm must be positive, got {0}")]
    InvalidBpm(f64),
}

/// Layout extension that maps BMSON channel numbers to chart lanes and
/// constructs [`NoteData`] from BMSON note events.
///
/// Implement this trait on a [`Layout`] type to define how BMSON player
/// channels (`x`) map to on-screen lanes, and how note data is constructed
/// for regular notes, mines, and invisible notes.
///
/// Default implementations are provided for all built-in layout types
/// ([`Beat7k`], [`Beat5k`], [`Beat14k`], [`Beat10k`], [`Popn9k`], [`Popn5k`],
/// [`GenericLayout`]).
pub trait BmsonMapping: Layout {
    /// Map a BMSON player channel (`x`) to a zero-based lane index.
    ///
    /// Returns `None` if the channel is unmapped (note is discarded).
    fn map_x(&self, x: u64) -> Option<u16>;

    /// Construct note data for a regular (playable) note event.
    fn make_note_data(&self, note: &NoteEvent, lane: u16) -> Self::NoteData;

    /// Construct note data for a mine note.
    fn make_mine_data(&self, lane: u16, damage: f64) -> Self::NoteData;

    /// Construct note data for an invisible (key) note.
    fn make_invisible_data(&self, lane: u16) -> Self::NoteData;
}

/// Zero-sized processor that converts [`bmson_def::Bmson`] into [`Chart`].
///
/// Call [`process`](Self::process) with a layout, or
/// [`process_default`](Self::process_default) to infer the layout from
/// `mode_hint`.
pub struct BmsonProcessor;

impl BmsonProcessor {
    /// Process a BMSON chart with an explicit layout.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if `init_bpm` is not positive.
    pub fn process<L>(
        bmson: &bmson_def::Bmson<'_>,
        layout: &L,
    ) -> Result<Chart<L::NoteData>, ProcessError>
    where
        L: BmsonMapping,
    {
        let data = &bmson.chart_data;

        if data.init_bpm <= 0.0 {
            return Err(ProcessError::InvalidBpm(data.init_bpm));
        }

        let timing = build_timing(data);
        let resolution = data.resolution;
        let playable_pulses = collect_playable_pulses(&data.sound_channels);

        let (audio_assets, notes, bgm) = process_sound_channels(
            &data.sound_channels,
            layout,
            &timing,
            resolution,
            &playable_pulses,
        );

        let mut audio_assets = audio_assets;
        let mut notes = notes;

        process_mine_channels(&bmson.mine_channels, layout, &mut audio_assets, &mut notes);
        process_key_channels(&bmson.key_channels, layout, &mut audio_assets, &mut notes);

        notes.sort_by_key(|n| n.tick);

        let bar_lines = build_bar_lines(data.lines.as_deref(), resolution, &notes, &bgm);
        let scroll_events = build_scroll_events(&bmson.scroll_events);
        let bga = build_bga(&bmson.chart_info.bga);
        let metadata = build_metadata(bmson);

        Ok(Chart {
            metadata,
            resolution,
            lane_count: layout.lane_count(),
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

    /// Process a BMSON chart, inferring the layout from `mode_hint`.
    ///
    /// Equivalent to calling [`process`](Self::process) with the matching
    /// built-in layout type. Unrecognised mode hints fall back to
    /// [`Beat7k`] (the BMSON default).
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if `init_bpm` is not positive.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "generic key count fits in u16 for practical layouts"
    )]
    pub fn process_default(
        bmson: &bmson_def::Bmson<'_>,
    ) -> Result<Chart<DefaultNoteData>, ProcessError> {
        match bmson.chart_data.mode_hint {
            ModeHint::Beat5k | ModeHint::Dj5k | ModeHint::Dj5kOnly | ModeHint::DjRuby => {
                Self::process(bmson, &Beat5k)
            }
            ModeHint::Beat14k | ModeHint::Dj14k => Self::process(bmson, &Beat14k),
            ModeHint::Beat10k | ModeHint::Dj10k => Self::process(bmson, &Beat10k),
            ModeHint::Popn9k => Self::process(bmson, &Popn9k),
            ModeHint::Popn5k => Self::process(bmson, &Popn5k),
            ModeHint::Generic(n) => Self::process(bmson, &GenericLayout { keys: n as u16 }),
            _ => Self::process(bmson, &Beat7k),
        }
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
fn process_sound_channels<L: BmsonMapping>(
    channels: &[bmson_def::SoundChannel<'_>],
    layout: &L,
    timing: &TimingTrack,
    resolution: u64,
    playable_pulses: &BTreeSet<u64>,
) -> (Vec<AudioAsset>, Vec<Note<L::NoteData>>, Vec<BgmEvent>) {
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
                let Some(lane) = layout.map_x(ne.x) else {
                    continue;
                };
                notes.push(Note {
                    tick: ne.y,
                    audio: audio_idx,
                    data: layout.make_note_data(ne, lane),
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
fn process_mine_channels<L: BmsonMapping>(
    channels: &[bmson_def::MineChannel<'_>],
    layout: &L,
    audio_assets: &mut Vec<AudioAsset>,
    notes: &mut Vec<Note<L::NoteData>>,
) {
    for mc in channels {
        let mine_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: mc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for mn in &mc.notes {
            let Some(lane) = layout.map_x(mn.x) else {
                continue;
            };
            notes.push(Note {
                tick: mn.y,
                audio: Some(mine_audio_idx),
                data: layout.make_mine_data(lane, mn.damage),
            });
        }
    }
}

/// Process key (invisible) channels: same structure as mine channels.
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_key_channels<L: BmsonMapping>(
    channels: &[bmson_def::KeyChannel<'_>],
    layout: &L,
    audio_assets: &mut Vec<AudioAsset>,
    notes: &mut Vec<Note<L::NoteData>>,
) {
    for kc in channels {
        let key_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: kc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for kn in &kc.notes {
            let Some(lane) = layout.map_x(kn.x) else {
                continue;
            };
            notes.push(Note {
                tick: kn.y,
                audio: Some(key_audio_idx),
                data: layout.make_invisible_data(lane),
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

/// Implement `BmsonMapping` for a layout whose `map_x` is a pure function
/// of the channel `x`.
macro_rules! impl_bmson_mapping {
    ($ty:ty, |$x:ident| $body:expr) => {
        impl BmsonMapping for $ty {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "channel value is bounded by layout"
            )]
            #[inline]
            fn map_x(&self, $x: u64) -> Option<u16> {
                $body
            }

            #[inline]
            fn make_note_data(&self, note: &NoteEvent, lane: u16) -> Self::NoteData {
                default_note_data(note, lane)
            }

            #[inline]
            fn make_mine_data(&self, lane: u16, damage: f64) -> Self::NoteData {
                DefaultNoteData {
                    lane,
                    kind: NoteKind::Mine { damage },
                }
            }

            #[inline]
            fn make_invisible_data(&self, lane: u16) -> Self::NoteData {
                DefaultNoteData {
                    lane,
                    kind: NoteKind::Invisible,
                }
            }
        }
    };
}

impl_bmson_mapping!(Beat7k, |x| match x {
    1..=8 => Some((x - 1) as u16),
    _ => None,
});

impl_bmson_mapping!(Beat5k, |x| match x {
    1..=5 => Some((x - 1) as u16),
    8 => Some(5),
    _ => None,
});

impl_bmson_mapping!(Beat14k, |x| match x {
    1..=16 => Some((x - 1) as u16),
    _ => None,
});

impl_bmson_mapping!(Beat10k, |x| match x {
    1..=5 => Some((x - 1) as u16),
    8 => Some(5),
    9..=13 => Some((x - 9 + 6) as u16),
    16 => Some(11),
    _ => None,
});

impl_bmson_mapping!(Popn9k, |x| match x {
    1..=9 => Some((x - 1) as u16),
    _ => None,
});

impl_bmson_mapping!(Popn5k, |x| match x {
    1..=5 => Some((x - 1) as u16),
    _ => None,
});

/// Build [`DefaultNoteData`] from a BMSON note event and lane.
fn default_note_data(note: &NoteEvent, lane: u16) -> DefaultNoteData {
    DefaultNoteData {
        lane,
        kind: if note.l > 0 {
            NoteKind::Long { duration: note.l }
        } else {
            NoteKind::Normal
        },
    }
}

// GenericLayout needs `self.keys`, so it has an explicit impl.
impl BmsonMapping for GenericLayout {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x is bounded by self.keys which is u16"
    )]
    #[inline]
    fn map_x(&self, x: u64) -> Option<u16> {
        if x >= 1 && x <= u64::from(self.keys) {
            Some((x - 1) as u16)
        } else {
            None
        }
    }

    #[inline]
    fn make_note_data(&self, note: &NoteEvent, lane: u16) -> Self::NoteData {
        default_note_data(note, lane)
    }

    #[inline]
    fn make_mine_data(&self, lane: u16, damage: f64) -> Self::NoteData {
        DefaultNoteData {
            lane,
            kind: NoteKind::Mine { damage },
        }
    }

    #[inline]
    fn make_invisible_data(&self, lane: u16) -> Self::NoteData {
        DefaultNoteData {
            lane,
            kind: NoteKind::Invisible,
        }
    }
}

/// Build a [`BpmChange`] from a BMSON [`BpmEvent`].
fn build_bpm_change(e: &BpmEvent) -> BpmChange {
    BpmChange {
        tick: e.y,
        bpm: e.bpm,
    }
}

/// Build a [`StopEvent`] from a BMSON stop event.
fn build_stop_event(e: &BmsonStopEvent) -> StopEvent {
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
    notes: &[Note<impl NoteData>],
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
fn build_bga_event(e: &BGAEvent) -> BgaTimelineEvent {
    BgaTimelineEvent {
        tick: e.y,
        resource_id: e.id as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmson_def::{MineChannel, MineNote, ScrollEvent, SoundChannel};
    use std::path::Path;

    const TITLE: &str = "Test Song";
    const ARTIST: &str = "Test Artist";
    const GENRE: &str = "Test Genre";

    fn make_bmson(sound_channels: Vec<SoundChannel<'_>>) -> bmson_def::Bmson<'_> {
        bmson_def::Bmson {
            version: "2.0.0",
            song_info: bmson_def::SongInfo {
                title: TITLE,
                artist: ARTIST,
                genre: GENRE,
            },
            chart_info: bmson_def::ChartInfo {
                subtitle: "",
                subartists: vec![],
                chart_name: "NORMAL",
                level: 5,
                back_image: None,
                eyecatch_image: None,
                banner_image: None,
                preview_music: None,
                title_image: None,
                bga: bmson_def::BGA {
                    bga_header: vec![],
                    bga_events: vec![],
                    layer_events: vec![],
                    poor_events: vec![],
                },
            },
            chart_data: bmson_def::ChartData {
                mode_hint: ModeHint::Beat7k,
                ln_type_hint: bmson_def::LnType::Ln,
                ln_judge_hint: bmson_def::LnJudge::Normal,
                ln_life_hint: bmson_def::LnLife::Normal,
                init_bpm: 120.0,
                judge_multiplier: 1.0,
                life_multiplier: 1.0,
                resolution: 240,
                lines: None,
                bpm_events: vec![],
                stop_events: vec![],
                sound_channels,
                judge_deltas: None,
                life_deltas: None,
            },
            scroll_events: vec![],
            mine_channels: vec![],
            key_channels: vec![],
        }
    }

    fn ne(x: u64, y: u64, l: u64) -> NoteEvent {
        NoteEvent {
            x,
            y,
            l,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }
    }

    #[test]
    fn beat7k_maps_keys_and_scratch() {
        assert_eq!(Beat7k.map_x(1), Some(0));
        assert_eq!(Beat7k.map_x(7), Some(6));
        assert_eq!(Beat7k.map_x(8), Some(7));
        assert_eq!(Beat7k.map_x(9), None);
        assert_eq!(Beat7k.map_x(0), None);
    }

    #[test]
    fn beat5k_maps_keys_and_scratch() {
        assert_eq!(Beat5k.map_x(1), Some(0));
        assert_eq!(Beat5k.map_x(5), Some(4));
        assert_eq!(Beat5k.map_x(8), Some(5));
        assert_eq!(Beat5k.map_x(6), None);
    }

    #[test]
    fn beat14k_maps_all_sixteen_channels() {
        assert_eq!(Beat14k.map_x(1), Some(0));
        assert_eq!(Beat14k.map_x(8), Some(7));
        assert_eq!(Beat14k.map_x(9), Some(8));
        assert_eq!(Beat14k.map_x(16), Some(15));
        assert_eq!(Beat14k.map_x(17), None);
    }

    #[test]
    fn beat10k_maps_split_layout() {
        assert_eq!(Beat10k.map_x(1), Some(0));
        assert_eq!(Beat10k.map_x(5), Some(4));
        assert_eq!(Beat10k.map_x(8), Some(5));
        assert_eq!(Beat10k.map_x(9), Some(6));
        assert_eq!(Beat10k.map_x(13), Some(10));
        assert_eq!(Beat10k.map_x(16), Some(11));
        assert_eq!(Beat10k.map_x(6), None);
    }

    #[test]
    fn popn9k_maps_nine_keys() {
        assert_eq!(Popn9k.map_x(1), Some(0));
        assert_eq!(Popn9k.map_x(9), Some(8));
        assert_eq!(Popn9k.map_x(10), None);
    }

    #[test]
    fn generic_layout_maps_by_keys() {
        let layout = GenericLayout { keys: 4 };
        assert_eq!(layout.map_x(1), Some(0));
        assert_eq!(layout.map_x(4), Some(3));
        assert_eq!(layout.map_x(5), None);
    }

    #[test]
    fn short_note_is_normal() {
        let note = ne(1, 0, 0);
        let data = Beat7k.make_note_data(&note, 0);
        assert_eq!(data.kind, NoteKind::Normal);
    }

    #[test]
    fn long_note_preserves_duration() {
        let note = ne(1, 0, 480);
        let data = Beat7k.make_note_data(&note, 0);
        assert_eq!(data.kind, NoteKind::Long { duration: 480 });
    }

    #[test]
    fn mine_data_has_damage() {
        let data = Beat7k.make_mine_data(3, 0.5);
        assert_eq!(data.kind, NoteKind::Mine { damage: 0.5 });
    }

    #[test]
    fn invisible_data_is_invisible() {
        let data = Beat7k.make_invisible_data(2);
        assert_eq!(data.kind, NoteKind::Invisible);
    }

    #[test]
    fn bgm_discarded_when_playable_at_same_pulse() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                ne(1, 240, 0), // playable at pulse 240
                ne(0, 240, 0), // BGM at same pulse -> discarded
            ],
        }]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.bgm.len(), 0);
        assert_eq!(chart.notes.len(), 1);
    }

    #[test]
    fn bgm_kept_when_no_playable_at_same_pulse() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![
                ne(1, 0, 0),   // playable at pulse 0
                ne(0, 240, 0), // BGM at pulse 240 -> kept
            ],
        }]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.bgm.len(), 1);
        assert_eq!(chart.bgm[0].tick, 240);
    }

    #[test]
    fn process_basic_chart() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 0), ne(2, 240, 0), ne(1, 480, 0)],
        }]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.notes.len(), 3);
        assert_eq!(chart.notes[0].tick, 0);
        assert_eq!(chart.notes[0].data.lane(), 0);
        assert_eq!(chart.notes[1].tick, 240);
        assert_eq!(chart.notes[1].data.lane(), 1);
        assert_eq!(chart.notes[2].tick, 480);
        assert_eq!(chart.notes[2].data.lane(), 0);
        assert_eq!(chart.audio_assets.len(), 3);
        assert_eq!(chart.audio_assets[0].path, Path::new("demo.wav"));
    }

    #[test]
    fn process_long_note() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 480)],
        }]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.notes.len(), 1);
        assert_eq!(chart.notes[0].data.kind, NoteKind::Long { duration: 480 });
    }

    #[test]
    fn process_invalid_bpm_returns_error() {
        let mut bmson = make_bmson(vec![]);
        bmson.chart_data.init_bpm = 0.0;

        let result = BmsonProcessor::process(&bmson, &Beat7k);
        assert!(result.is_err());
    }

    #[test]
    fn process_default_uses_beat7k() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 0)],
        }]);

        let chart = BmsonProcessor::process_default(&bmson).expect("processing succeeds");

        assert_eq!(chart.lane_count, 8);
        assert_eq!(chart.notes.len(), 1);
    }

    #[test]
    fn process_generates_auto_bar_lines() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 960, 0)],
        }]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert!(!chart.bar_lines.is_empty());
        assert_eq!(chart.bar_lines[0].tick, 0);
    }

    #[test]
    fn process_explicit_bar_lines() {
        let mut bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 0)],
        }]);
        bmson.chart_data.lines = Some(vec![
            bmson_def::BarLine { y: 0 },
            bmson_def::BarLine { y: 100 },
        ]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.bar_lines.len(), 2);
        assert_eq!(chart.bar_lines[0].tick, 0);
        assert_eq!(chart.bar_lines[1].tick, 100);
    }

    #[test]
    fn process_bga_events() {
        let mut bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 0)],
        }]);
        bmson.chart_info.bga = bmson_def::BGA {
            bga_header: vec![bmson_def::BGAHeader {
                id: 1,
                name: Path::new("bg.png"),
            }],
            bga_events: vec![BGAEvent { y: 0, id: 1 }],
            layer_events: vec![],
            poor_events: vec![],
        };

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.bga.resources.len(), 1);
        assert_eq!(chart.bga.resources[0].path, Path::new("bg.png"));
        assert_eq!(chart.bga.events.len(), 1);
        assert_eq!(chart.bga.events[0].tick, 0);
        assert_eq!(chart.bga.events[0].resource_id, 1);
    }

    #[test]
    fn process_mine_channel() {
        let mut bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 0)],
        }]);
        bmson.mine_channels = vec![MineChannel {
            name: Path::new("mine.wav"),
            notes: vec![MineNote {
                x: 1,
                y: 480,
                damage: 0.5,
            }],
        }];

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        let mine_note = chart
            .notes
            .iter()
            .find(|n| n.tick == 480)
            .expect("mine note exists");
        assert_eq!(mine_note.data.kind, NoteKind::Mine { damage: 0.5 });
    }

    #[test]
    fn process_scroll_events() {
        let mut bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 0)],
        }]);
        bmson.scroll_events = vec![ScrollEvent { y: 240, rate: 2.0 }];

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.scroll_events.len(), 1);
        assert_eq!(chart.scroll_events[0].tick, 240);
        assert!((chart.scroll_events[0].rate - 2.0).abs() < 1e-9);
    }

    #[test]
    fn process_metadata() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 0, 0)],
        }]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.metadata.title, TITLE);
        assert_eq!(chart.metadata.artist, ARTIST);
        assert_eq!(chart.metadata.genre, GENRE);
        assert_eq!(chart.metadata.chart_name, "NORMAL");
        assert_eq!(chart.metadata.level, 5);
    }

    #[test]
    fn notes_sorted_by_tick() {
        let bmson = make_bmson(vec![SoundChannel {
            name: Path::new("demo.wav"),
            note_events: vec![ne(1, 480, 0), ne(2, 0, 0), ne(3, 240, 0)],
        }]);

        let chart = BmsonProcessor::process(&bmson, &Beat7k).expect("processing succeeds");

        assert_eq!(chart.notes[0].tick, 0);
        assert_eq!(chart.notes[1].tick, 240);
        assert_eq!(chart.notes[2].tick, 480);
    }
}
