//! BMS → `Chart` conversion processor.
//!
//! [`BmsProcessor`] converts a `bms_parser::Bms` into a format-agnostic
//! `Chart` by applying a `BmsLayout` mode family (e.g. `Bme`, `Pms`,
//! `Nanasi`).
//!
//! # Pipeline
//!
//! ```text
//! bms_parser::Bms → BmsProcessor::process(bms, layout) → Chart<NoteData>
//! ```
//!
//! # Mode families
//!
//! The layout argument selects how BMS `(player, lane)` channel bytes decode
//! into flat chart lanes. `Bme` covers beat-5k/7k/10k/14k uniformly (the key
//! count of a chart is whatever its notes use). Other families (`Pms`,
//! `PmsBme`, `Nanasi`, `DscOctFp`) cover their eponymous modes.
//! [`BmsProcessor::process_default`] uses `Bme`.
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
use bms_tokenizer::{BmpTag, BmsIndex, WavTag};
use bmsrs_chart::{
    AudioAsset, BarLine, Bga, BgaResource, BgaTimelineEvent, BgmEvent, Bme, BmsChannel, BmsLayout,
    BpmChange, Chart, ChartMetadata, Note, NoteData, NoteKind, ScrollChangeEvent, StopEvent,
    TimingTrack,
};
use thiserror::Error;

use crate::long_note::{PairedLn, pair_lnobj, pair_lntype1, pair_lntype2};
use crate::position::MeasureTable;

/// Errors that can occur during BMS processing.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// Initial BPM is missing or invalid.
    #[error("init_bpm must be positive, got {0}")]
    InvalidBpm(f64),
}

/// Zero-sized processor that converts [`Bms`] into [`Chart`].
pub struct BmsProcessor;

/// Default resolution (ticks per quarter note) for BMS charts.
const RESOLUTION: u64 = 240;

impl BmsProcessor {
    /// Process a BMS chart with an explicit mode-family layout.
    ///
    /// The layout type determines how each BMS `(player, lane)` channel byte
    /// decodes into a note position. See [`bmsrs_chart::layout`] for the
    /// available families.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if the initial BPM is missing
    /// or not positive.
    pub fn process<L>(bms: &Bms) -> Result<Chart<NoteData>, ProcessError>
    where
        L: BmsLayout,
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
        let notes = collect_notes::<L>(bms, &table, &wav_map, &paired_lns, &consumed);
        let bgm = collect_bgm(bms, &table, &wav_map);

        Ok(Chart {
            metadata: build_metadata(bms),
            resolution: RESOLUTION,
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

    /// Process a BMS chart with the default [`Bme`] layout.
    ///
    /// `Bme` covers beat-5k/7k/10k/14k uniformly — both player sides are
    /// mapped, so single-player charts simply leave the 2P lanes unused and
    /// dual-player charts populate them. The `#PLAYER` header does not affect
    /// the mapping (consistent with modern engines, which ignore it).
    ///
    /// For PMS, nanasi, or other families, call
    /// [`process::<Nanasi>`](Self::process) with the relevant layout type.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if the initial BPM is missing
    /// or not positive.
    pub fn process_default(bms: &Bms) -> Result<Chart<NoteData>, ProcessError> {
        Self::process::<Bme>(bms)
    }
}

// Builder helpers

/// Determine LN mode and pair LNs. Returns paired LNs and consumed note indices.
///
/// The LN notation is selected in this priority:
/// 1. `#LNOBJ` — LNOBJ notation (channels 11-49 with marker WAV).
/// 2. `#LNTYPE 2` — MGQ notation (channels 51-69, 00 = release).
/// 3. `#LNTYPE 1` / default — RDM notation (channels 51-69, consecutive pairs).
fn pair_long_notes(bms: &Bms, table: &MeasureTable) -> (Vec<PairedLn>, BTreeSet<usize>) {
    // LNOBJ takes highest priority.
    if let Some(ln_obj) = bms.gameplay.ln_obj {
        return pair_lnobj(&bms.messages.note_events, ln_obj, table);
    }

    // Check for MGQ notation.
    if bms.gameplay.ln_type == Some(bms_tokenizer::LnType::Type2) {
        return (
            pair_lntype2(&bms.messages.long_note_events, table),
            BTreeSet::new(),
        );
    }

    // Default: RDM / LNTYPE 1.
    (
        pair_lntype1(&bms.messages.long_note_events, table),
        BTreeSet::new(),
    )
}

/// Collect all playable notes (visible, invisible, LN, mines) into a sorted vector.
fn collect_notes<L: BmsLayout>(
    bms: &Bms,
    table: &MeasureTable,
    wav_map: &BTreeMap<BmsIndex<WavTag>, u32>,
    paired_lns: &[PairedLn],
    consumed: &BTreeSet<usize>,
) -> Vec<Note<NoteData>> {
    let mut notes = Vec::new();

    // Visible notes (skip consumed LNOBJ pairs).
    for (i, ne) in bms.messages.note_events.iter().enumerate() {
        if consumed.contains(&i) {
            continue;
        }
        if ne.key_type == KeyType::Visible {
            if let Some(nd) = BmsChannel::new(ne.player, ne.lane).and_then(L::map_channel) {
                notes.push(Note {
                    tick: table.position_to_tick(ne.position),
                    audio: wav_map.get(&ne.wav_id).copied(),
                    data: NoteData {
                        kind: NoteKind::Normal,
                        ..nd
                    },
                });
            }
        }
    }

    // Invisible notes (keysounds).
    for ne in &bms.messages.note_events {
        if ne.key_type == KeyType::Invisible {
            if let Some(nd) = BmsChannel::new(ne.player, ne.lane).and_then(L::map_channel) {
                notes.push(Note {
                    tick: table.position_to_tick(ne.position),
                    audio: wav_map.get(&ne.wav_id).copied(),
                    data: NoteData {
                        kind: NoteKind::Invisible,
                        ..nd
                    },
                });
            }
        }
    }

    // Paired long notes.
    for ln in paired_lns {
        if let Some(nd) = BmsChannel::new(ln.player, ln.lane).and_then(L::map_channel) {
            notes.push(Note {
                tick: ln.tick,
                audio: wav_map.get(&ln.wav_id).copied(),
                data: NoteData {
                    kind: NoteKind::Long {
                        duration: ln.duration,
                    },
                    ..nd
                },
            });
        }
    }

    // Mines.
    for me in &bms.messages.mine_events {
        if let Some(nd) = BmsChannel::new(me.player, me.lane).and_then(L::map_channel) {
            notes.push(Note {
                tick: table.position_to_tick(me.position),
                audio: None,
                data: NoteData {
                    kind: NoteKind::Mine { damage: 1.0 },
                    ..nd
                },
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

/// Build scroll-speed change events, sorted by tick.
fn build_scroll_events(bms: &Bms, table: &MeasureTable) -> Vec<ScrollChangeEvent> {
    let mut events: Vec<ScrollChangeEvent> = bms
        .messages
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
        .collect();
    events.sort_by_key(|e| e.tick);
    events
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
