//! BMS → [`Chart`] conversion processor.
//!
//! [`BmsProcessor`] converts a [`bms_parser::Bms`] into a format-agnostic
//! [`Chart<T>`] by applying a [`BmsMapping`] layout.
//!
//! # Pipeline
//!
//! ```text
//! bms_parser::Bms → BmsProcessor::process(bms, layout) → Chart<L::NoteData>
//! ```
//!
//! # Long-note modes
//!
//! BMS supports two LN notations, selected automatically:
//!
//! - **LNOBJ**: when `#LNOBJ` is defined, regular notes paired by the
//!   designated WAV index form LNs.
//! - **LNTYPE 1 (RDM)**: events on channels 51–69 are paired consecutively
//!   per `(player, lane)`.

mod long_note;
mod position;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use bms_parser::{BgaLayer, Bms, BpmValue, KeyType};
use bms_tokenizer::{BmpTag, BmsIndex, PlayerMode, WavTag};
use bmsrs_chart::{
    AudioAsset, BarLine, Beat5k, Beat7k, Beat10k, Beat14k, Bga, BgaResource, BgaTimelineEvent,
    BgmEvent, BpmChange, Chart, ChartMetadata, DefaultNoteData, Layout, Note, NoteKind,
    ScrollChangeEvent, StopEvent, TimingTrack,
};
use thiserror::Error;

use crate::long_note::{PairedLn, pair_lnobj, pair_lntype1};
use crate::position::MeasureTable;

/// Errors that can occur during BMS processing.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// Initial BPM is missing or invalid.
    #[error("init_bpm must be positive, got {0}")]
    InvalidBpm(f64),
}

/// Layout extension that maps BMS player/lane numbers to chart lanes and
/// constructs [`NoteData`](bmsrs_chart::NoteData).
///
/// Default implementations are provided for [`Beat7k`], [`Beat5k`],
/// [`Beat14k`], and [`Beat10k`].
pub trait BmsMapping: Layout {
    /// Map a BMS `(player, lane)` pair to a zero-based chart lane index.
    ///
    /// Returns `None` if unmapped (note is discarded).
    fn map_lane(&self, player: u8, lane: u8) -> Option<u16>;

    /// Construct note data for a note on `player` at `chart_lane`.
    fn make_note_data(&self, player: u8, chart_lane: u16, kind: NoteKind) -> Self::NoteData;
}

/// Zero-sized processor that converts [`Bms`] into [`Chart`].
pub struct BmsProcessor;

/// Default resolution (ticks per quarter note) for BMS charts.
const RESOLUTION: u64 = 240;

impl BmsProcessor {
    /// Process a BMS chart with an explicit layout.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if the initial BPM is missing
    /// or not positive.
    pub fn process<L>(bms: &Bms, layout: &L) -> Result<Chart<L::NoteData>, ProcessError>
    where
        L: BmsMapping,
    {
        let init_bpm = bms.timing.bpm.unwrap_or(130.0);
        if init_bpm <= 0.0 {
            return Err(ProcessError::InvalidBpm(init_bpm));
        }

        let max_measure = find_max_measure(bms);
        let table = MeasureTable::new(max_measure, &bms.messages.measure_lengths, RESOLUTION);
        let (wav_map, audio_assets) = build_audio_assets(&bms.audio.wav_files);
        let bpm_changes = build_bpm_changes(bms, &table);

        let mut stops = build_stops_from_defs(bms, &table);
        stops.extend(build_stops_from_stp(bms, &table, &bpm_changes, init_bpm));
        stops.sort_by_key(|s| s.tick);

        let timing = TimingTrack {
            init_bpm,
            bpm_changes,
            stops,
        };

        let (paired_lns, consumed) = pair_long_notes(bms, &table);
        let notes = collect_notes(bms, layout, &table, &wav_map, &paired_lns, &consumed);
        let bgm = collect_bgm(bms, &table, &wav_map);

        Ok(Chart {
            metadata: build_metadata(bms),
            resolution: RESOLUTION,
            lane_count: layout.lane_count(),
            timing,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            notes,
            bgm,
            audio_assets,
            bar_lines: build_bar_lines(max_measure),
            scroll_events: build_scroll_events(bms, &table),
            bga: build_bga(bms, &table),
        })
    }

    /// Process a BMS chart, inferring the layout from `#PLAYER`.
    ///
    /// Double Play → [`Beat14k`]; all others → [`Beat7k`].
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if the initial BPM is missing
    /// or not positive.
    pub fn process_default(bms: &Bms) -> Result<Chart<DefaultNoteData>, ProcessError> {
        match bms.gameplay.player {
            Some(PlayerMode::Double | PlayerMode::Battle) => Self::process(bms, &Beat14k),
            _ => Self::process(bms, &Beat7k),
        }
    }
}

// Builder helpers

/// Determine LN mode and pair LNs. Returns paired LNs and consumed note indices.
fn pair_long_notes(bms: &Bms, table: &MeasureTable) -> (Vec<PairedLn>, BTreeSet<usize>) {
    if let Some(ln_obj) = bms.gameplay.ln_obj {
        pair_lnobj(&bms.messages.note_events, ln_obj, table)
    } else {
        (
            pair_lntype1(&bms.messages.long_note_events, table),
            BTreeSet::new(),
        )
    }
}

/// Collect all playable notes (visible, invisible, LN, mines) into a sorted vector.
fn collect_notes<L: BmsMapping>(
    bms: &Bms,
    layout: &L,
    table: &MeasureTable,
    wav_map: &BTreeMap<BmsIndex<WavTag>, u32>,
    paired_lns: &[PairedLn],
    consumed: &BTreeSet<usize>,
) -> Vec<Note<L::NoteData>> {
    let mut notes = Vec::new();

    // Visible notes (skip consumed LNOBJ pairs).
    for (i, ne) in bms.messages.note_events.iter().enumerate() {
        if consumed.contains(&i) {
            continue;
        }
        if ne.key_type == KeyType::Visible {
            if let Some(lane) = layout.map_lane(ne.player, ne.lane) {
                notes.push(Note {
                    tick: table.position_to_tick(ne.position),
                    audio: wav_map.get(&ne.wav_id).copied(),
                    data: layout.make_note_data(ne.player, lane, NoteKind::Normal),
                });
            }
        }
    }

    // Invisible notes (keysounds).
    for ne in &bms.messages.note_events {
        if ne.key_type == KeyType::Invisible {
            if let Some(lane) = layout.map_lane(ne.player, ne.lane) {
                notes.push(Note {
                    tick: table.position_to_tick(ne.position),
                    audio: wav_map.get(&ne.wav_id).copied(),
                    data: layout.make_note_data(ne.player, lane, NoteKind::Invisible),
                });
            }
        }
    }

    // Paired long notes.
    for ln in paired_lns {
        if let Some(lane) = layout.map_lane(ln.player, ln.lane) {
            notes.push(Note {
                tick: ln.tick,
                audio: wav_map.get(&ln.wav_id).copied(),
                data: layout.make_note_data(
                    ln.player,
                    lane,
                    NoteKind::Long {
                        duration: ln.duration,
                    },
                ),
            });
        }
    }

    // Mines.
    for me in &bms.messages.mine_events {
        if let Some(lane) = layout.map_lane(me.player, me.lane) {
            notes.push(Note {
                tick: table.position_to_tick(me.position),
                audio: None,
                data: layout.make_note_data(me.player, lane, NoteKind::Mine { damage: 1.0 }),
            });
        }
    }

    notes.sort_by_key(|n| n.tick);
    notes
}

/// Collect BGM events into a vector.
fn collect_bgm(
    bms: &Bms,
    table: &MeasureTable,
    wav_map: &BTreeMap<BmsIndex<WavTag>, u32>,
) -> Vec<BgmEvent> {
    bms.messages
        .bgm_events
        .iter()
        .filter_map(|be| {
            wav_map.get(&be.wav_id).copied().map(|audio| BgmEvent {
                tick: table.position_to_tick(be.position),
                audio,
            })
        })
        .collect()
}

/// Build WAV audio assets and a `BmsIndex<WavTag>` → `u32` lookup map.
fn build_audio_assets(
    wav_files: &BTreeMap<BmsIndex<WavTag>, String>,
) -> (BTreeMap<BmsIndex<WavTag>, u32>, Vec<AudioAsset>) {
    let mut wav_map = BTreeMap::new();
    let mut audio_assets = Vec::new();
    for (&wav_id, path) in wav_files {
        #[expect(clippy::cast_possible_truncation, reason = "WAV count fits in u32")]
        let idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: path.clone().into(),
            start: Duration::ZERO,
            duration: None,
        });
        wav_map.insert(wav_id, idx);
    }
    (wav_map, audio_assets)
}

/// Build BPM change events.
fn build_bpm_changes(bms: &Bms, table: &MeasureTable) -> Vec<BpmChange> {
    bms.messages
        .bpm_changes
        .iter()
        .map(|bc| BpmChange {
            tick: table.position_to_tick(bc.position),
            bpm: resolve_bpm(bc.value, bms),
        })
        .collect()
}

/// Resolve a [`BpmValue`] to a concrete BPM.
fn resolve_bpm(value: BpmValue, bms: &Bms) -> f64 {
    match value {
        BpmValue::Absolute(bpm) => bpm,
        BpmValue::Reference(id) => bms.timing.bpm_defs.get(&id).copied().unwrap_or(120.0),
    }
}

/// Find the BPM active at `tick` (last BPM change at or before `tick`).
fn bpm_at_tick(bpm_changes: &[BpmChange], init_bpm: f64, tick: u64) -> f64 {
    let mut bpm = init_bpm;
    for bc in bpm_changes {
        if bc.tick <= tick {
            bpm = bc.bpm;
        } else {
            break;
        }
    }
    bpm
}

/// Build stop events from `#STOPxx` definitions (channel `09`).
///
/// BMS STOP unit: 1/192 of a 4/4 measure → ticks = `raw / 192 * res * 4`.
#[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
#[expect(clippy::cast_sign_loss, reason = "raw is non-negative")]
#[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
fn build_stops_from_defs(bms: &Bms, table: &MeasureTable) -> Vec<StopEvent> {
    bms.messages
        .stop_events
        .iter()
        .filter_map(|se| {
            bms.timing.stop_defs.get(&se.stop_id).map(|&raw| StopEvent {
                tick: table.position_to_tick(se.position),
                duration: (raw / 192.0 * RESOLUTION as f64 * 4.0).round() as u64,
            })
        })
        .collect()
}

/// Build stop events from `#STP` headers (duration in milliseconds).
#[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
#[expect(clippy::cast_sign_loss, reason = "duration_ms is non-negative")]
#[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
fn build_stops_from_stp(
    bms: &Bms,
    table: &MeasureTable,
    bpm_changes: &[BpmChange],
    init_bpm: f64,
) -> Vec<StopEvent> {
    bms.messages
        .stp_events
        .iter()
        .map(|stp| {
            let tick = table.position_to_tick(stp.position);
            let bpm = bpm_at_tick(bpm_changes, init_bpm, tick);
            let tick_duration = stp.duration_ms / 1000.0 * bpm / 60.0 * RESOLUTION as f64;
            StopEvent {
                tick,
                duration: tick_duration.round() as u64,
            }
        })
        .collect()
}

/// Build scroll-speed change events.
fn build_scroll_events(bms: &Bms, table: &MeasureTable) -> Vec<ScrollChangeEvent> {
    bms.messages
        .scroll_events
        .iter()
        .filter_map(|se| {
            bms.timing
                .scroll_defs
                .get(&se.scroll_id)
                .map(|&rate| ScrollChangeEvent {
                    tick: table.position_to_tick(se.position),
                    rate,
                })
        })
        .collect()
}

/// Build BGA data from BGA events and BMP file definitions.
fn build_bga(bms: &Bms, table: &MeasureTable) -> Bga {
    let mut bmp_map: BTreeMap<BmsIndex<BmpTag>, u32> = BTreeMap::new();
    let mut resources = Vec::new();
    for (&bmp_id, path) in &bms.visual.bmp_files {
        #[expect(clippy::cast_possible_truncation, reason = "BMP count fits in u32")]
        let idx = resources.len() as u32;
        resources.push(BgaResource {
            id: idx,
            path: path.clone().into(),
        });
        bmp_map.insert(bmp_id, idx);
    }

    let map_bga_events = |layer: BgaLayer| -> Vec<BgaTimelineEvent> {
        bms.messages
            .bga_events
            .iter()
            .filter(|be| be.layer == layer)
            .filter_map(|be| {
                bmp_map
                    .get(&be.bmp_id)
                    .map(|&resource_id| BgaTimelineEvent {
                        tick: table.position_to_tick(be.position),
                        resource_id,
                    })
            })
            .collect()
    };

    Bga {
        resources,
        events: map_bga_events(BgaLayer::Base),
        layer_events: map_bga_events(BgaLayer::Layer),
        poor_events: map_bga_events(BgaLayer::Poor),
    }
}

/// Build auto 4/4 bar lines (one per measure).
fn build_bar_lines(max_measure: u16) -> Vec<BarLine> {
    let step = RESOLUTION * 4;
    (0..=u64::from(max_measure))
        .map(|i| BarLine { tick: i * step })
        .collect()
}

/// Build [`ChartMetadata`] from BMS metadata.
#[expect(clippy::cast_possible_truncation, reason = "play level fits in u64")]
#[expect(clippy::cast_sign_loss, reason = "play level is non-negative")]
fn build_metadata(bms: &Bms) -> ChartMetadata {
    ChartMetadata {
        title: bms.metadata.title.clone().unwrap_or_default(),
        subtitle: bms.metadata.subtitle.clone().unwrap_or_default(),
        artist: bms.metadata.artist.clone().unwrap_or_default(),
        subartists: bms
            .metadata
            .sub_artist
            .as_deref()
            .map(|s| vec![s.to_owned()])
            .unwrap_or_default(),
        genre: bms.metadata.genre.clone().unwrap_or_default(),
        chart_name: bms
            .display
            .play_level
            .map(|l| format!("{l:.0}"))
            .unwrap_or_default(),
        level: bms.display.play_level.map_or(0, |l| l as u64),
    }
}

/// Find the maximum measure number referenced by any event.
fn find_max_measure(bms: &Bms) -> u16 {
    let mut max_m = 0u16;

    for ml in &bms.messages.measure_lengths {
        max_m = max_m.max(ml.measure);
    }
    for ne in &bms.messages.note_events {
        max_m = max_m.max(ne.position.measure);
    }
    for le in &bms.messages.long_note_events {
        max_m = max_m.max(le.position.measure);
    }
    for be in &bms.messages.bgm_events {
        max_m = max_m.max(be.position.measure);
    }
    for me in &bms.messages.mine_events {
        max_m = max_m.max(me.position.measure);
    }
    for bc in &bms.messages.bpm_changes {
        max_m = max_m.max(bc.position.measure);
    }
    for se in &bms.messages.stop_events {
        max_m = max_m.max(se.position.measure);
    }
    for se in &bms.messages.scroll_events {
        max_m = max_m.max(se.position.measure);
    }
    for be in &bms.messages.bga_events {
        max_m = max_m.max(be.position.measure);
    }
    for stp in &bms.messages.stp_events {
        max_m = max_m.max(stp.position.measure);
    }

    max_m.max(1) + 1
}

// BmsMapping implementations

/// Implements [`BmsMapping`] for a zero-sized layout type.
macro_rules! impl_bms_mapping {
    ($ty:ty, |$player:ident, $lane:ident| $body:expr) => {
        impl BmsMapping for $ty {
            #[inline]
            fn map_lane(&self, $player: u8, $lane: u8) -> Option<u16> {
                $body
            }

            #[inline]
            fn make_note_data(
                &self,
                _player: u8,
                chart_lane: u16,
                kind: NoteKind,
            ) -> Self::NoteData {
                DefaultNoteData {
                    lane: chart_lane,
                    kind,
                }
            }
        }
    };
}

impl_bms_mapping!(Beat7k, |player, lane| match (player, lane) {
    (1, 1..=8) => Some(u16::from(lane - 1)),
    _ => None,
});

impl_bms_mapping!(Beat5k, |player, lane| match (player, lane) {
    (1, 1..=5) => Some(u16::from(lane - 1)),
    (1, 8) => Some(5),
    _ => None,
});

impl_bms_mapping!(Beat14k, |player, lane| match (player, lane) {
    (1, 1..=8) => Some(u16::from(lane - 1)),
    (2, 1..=8) => Some(u16::from(lane - 1 + 8)),
    _ => None,
});

impl_bms_mapping!(Beat10k, |player, lane| match (player, lane) {
    (1, 1..=5) => Some(u16::from(lane - 1)),
    (1, 8) => Some(5),
    (2, 1..=5) => Some(u16::from(lane - 1 + 6)),
    (2, 8) => Some(11),
    _ => None,
});

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bms_parser::{KeyType, NoteEvent, Position};
    use bms_tokenizer::LnObjTag;
    use bmsrs_chart::NoteData;

    #[test]
    fn beat7k_maps_keys_and_scratch() {
        assert_eq!(Beat7k.map_lane(1, 1), Some(0));
        assert_eq!(Beat7k.map_lane(1, 8), Some(7));
        assert_eq!(Beat7k.map_lane(2, 1), None);
    }

    #[test]
    fn beat5k_maps_five_keys_plus_scratch() {
        assert_eq!(Beat5k.map_lane(1, 1), Some(0));
        assert_eq!(Beat5k.map_lane(1, 5), Some(4));
        assert_eq!(Beat5k.map_lane(1, 8), Some(5));
    }

    #[test]
    fn beat14k_maps_both_players() {
        assert_eq!(Beat14k.map_lane(1, 1), Some(0));
        assert_eq!(Beat14k.map_lane(1, 8), Some(7));
        assert_eq!(Beat14k.map_lane(2, 1), Some(8));
        assert_eq!(Beat14k.map_lane(2, 8), Some(15));
    }

    #[test]
    fn beat10k_maps_split_layout() {
        assert_eq!(Beat10k.map_lane(1, 1), Some(0));
        assert_eq!(Beat10k.map_lane(1, 5), Some(4));
        assert_eq!(Beat10k.map_lane(1, 8), Some(5));
        assert_eq!(Beat10k.map_lane(2, 1), Some(6));
        assert_eq!(Beat10k.map_lane(2, 8), Some(11));
    }

    #[test]
    fn process_basic_note() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        bms.audio
            .wav_files
            .insert("01".parse().unwrap(), "kick.wav".to_owned());
        bms.messages.note_events.push(NoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            key_type: KeyType::Visible,
            wav_id: "01".parse().unwrap(),
        });

        let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

        assert_eq!(chart.notes.len(), 1);
        assert_eq!(chart.notes[0].tick, 0);
        assert_eq!(chart.notes[0].data.lane(), 0);
        assert_eq!(chart.notes[0].data.kind(), NoteKind::Normal);
        assert_eq!(chart.audio_assets.len(), 1);
    }

    #[test]
    fn process_zero_bpm_returns_error() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(0.0);
        let result = BmsProcessor::process(&bms, &Beat7k);
        assert!(result.is_err());
    }

    #[test]
    fn process_negative_bpm_returns_error() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(-10.0);
        let result = BmsProcessor::process(&bms, &Beat7k);
        assert!(result.is_err());
    }

    #[test]
    fn process_bgm_events_mapped() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        bms.audio
            .wav_files
            .insert("01".parse().unwrap(), "bgm.wav".to_owned());
        bms.messages.bgm_events.push(bms_parser::BgmEvent {
            position: Position::new(0, 0, 8),
            wav_id: "01".parse().unwrap(),
        });

        let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

        assert_eq!(chart.bgm.len(), 1);
        assert_eq!(chart.bgm[0].tick, 0);
    }

    #[test]
    fn process_metadata_from_headers() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        bms.metadata.title = Some("Test Song".to_owned());
        bms.metadata.artist = Some("Test Artist".to_owned());
        bms.metadata.genre = Some("Test Genre".to_owned());

        let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

        assert_eq!(chart.metadata.title, "Test Song");
        assert_eq!(chart.metadata.artist, "Test Artist");
        assert_eq!(chart.metadata.genre, "Test Genre");
    }

    #[test]
    fn process_default_single_player_uses_beat7k() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        bms.gameplay.player = Some(PlayerMode::Single);

        let chart = BmsProcessor::process_default(&bms).unwrap();

        assert_eq!(chart.lane_count, 8);
    }

    #[test]
    fn process_default_double_player_uses_beat14k() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        bms.gameplay.player = Some(PlayerMode::Double);

        let chart = BmsProcessor::process_default(&bms).unwrap();

        assert_eq!(chart.lane_count, 16);
    }

    #[test]
    fn process_lnobj_produces_long_note() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        bms.audio
            .wav_files
            .insert("01".parse().unwrap(), "a.wav".to_owned());
        bms.audio
            .wav_files
            .insert("02".parse().unwrap(), "b.wav".to_owned());
        let ln_obj: BmsIndex<LnObjTag> = "02".parse().unwrap();
        bms.gameplay.ln_obj = Some(ln_obj);
        bms.messages.note_events.push(NoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            key_type: KeyType::Visible,
            wav_id: "01".parse().unwrap(),
        });
        bms.messages.note_events.push(NoteEvent {
            position: Position::new(1, 0, 8),
            player: 1,
            lane: 1,
            key_type: KeyType::Visible,
            wav_id: "02".parse().unwrap(),
        });

        let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

        let lns: Vec<&Note> = chart
            .notes
            .iter()
            .filter(|n| matches!(n.data.kind(), NoteKind::Long { .. }))
            .collect();
        assert_eq!(lns.len(), 1);
        assert_eq!(lns[0].tick, 0);
    }

    #[test]
    fn process_bpm_change_reference_resolved() {
        use bms_parser::BpmChange;
        use bms_tokenizer::BpmTag;

        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        let bpm_id: BmsIndex<BpmTag> = "01".parse().unwrap();
        bms.timing.bpm_defs.insert(bpm_id, 200.0);
        bms.messages.bpm_changes.push(BpmChange {
            position: Position::new(1, 0, 8),
            value: BpmValue::Reference(bpm_id),
        });

        let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

        assert_eq!(chart.timing.bpm_changes.len(), 1);
        assert!((chart.timing.bpm_changes[0].bpm - 200.0).abs() < 1e-9);
    }

    #[test]
    fn process_bar_lines_generated() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);

        let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

        assert!(!chart.bar_lines.is_empty());
        assert_eq!(chart.bar_lines[0].tick, 0);
        assert_eq!(chart.bar_lines[1].tick, 960);
    }

    #[test]
    fn process_invisible_note_mapped() {
        let mut bms = Bms::default();
        bms.timing.bpm = Some(120.0);
        bms.audio
            .wav_files
            .insert("01".parse().unwrap(), "se.wav".to_owned());
        bms.messages.note_events.push(NoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            key_type: KeyType::Invisible,
            wav_id: "01".parse().unwrap(),
        });

        let chart = BmsProcessor::process(&bms, &Beat7k).unwrap();

        assert_eq!(chart.notes.len(), 1);
        assert_eq!(chart.notes[0].data.kind(), NoteKind::Invisible);
    }
}
