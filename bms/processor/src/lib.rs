//! BMS → `Chart` conversion processor.
//!
//! [`BmsProcessor`] converts a `bms_parser::Bms` into a format-agnostic
//! `Chart` by applying a [`BmsLayout`] mode family (e.g. [`Bme`], `Pms`,
//! `Nanasi`).
//!
//! Mode families and the [`BmsLayout`] trait live in this crate's `layout`
//! module.
//!
//! # Pipeline
//!
//! ```text
//! bms_parser::Bms → BmsProcessor::process::<L>(bms) → Chart<(), NoCustomEvent>
//! ```
//!
//! # Mode families
//!
//! The layout type selects how BMS `(player, lane)` channel bytes decode
//! into note positions. [`Bme`] covers beat-5k/7k/10k/14k uniformly (the key
//! count of a chart is whatever its notes use). Other families (`Pms`,
//! `PmsBme`, `Nanasi`, `DscOctFp`) cover their eponymous modes.
//! [`BmsProcessor::process_default`] uses [`Bme`].
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

pub mod layout;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use bms_parser::{Bms, BpmValue, KeyType};
use bms_tokenizer::{BmpIndex, WavIndex};
use bmsrs_chart::{
    AudioAsset, BgaResource, BpmChange, Chart, ChartMetadata, Event, NoteKind, StopEvent,
    TimingTrack,
};
use thiserror::Error;

use crate::layout::{Bme, BmsChannel, BmsLayout};

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
    /// decodes into a note position. See the `layout` module for the
    /// available families.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::InvalidBpm`] if the initial BPM is missing
    /// or not positive.
    pub fn process<L>(bms: &Bms) -> Result<Chart<(), bmsrs_chart::NoCustomEvent>, ProcessError>
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

        let bmp_map = build_bmp_map(&bms.visual.bmp_files);
        let bga_resources = bmp_map
            .values()
            .map(|(resource_id, path)| BgaResource {
                id: *resource_id,
                path: path.clone().into(),
            })
            .collect();

        let (paired_lns, consumed) = pair_long_notes(bms, &table);
        let mut events = Vec::new();

        // Bar lines first (priority 0).
        events.extend(build_bar_events(max_measure));

        // Notes (priority 1).
        collect_notes::<L>(bms, &table, &wav_map, &paired_lns, &consumed, &mut events);

        // BGM (priority 1).
        collect_bgm(bms, &table, &wav_map, &mut events);

        // BGA (priority 1).
        collect_bga(bms, &table, &bmp_map, &mut events);

        // BPM changes (priority 2).
        collect_bpm_events(bms, &table, &mut events);

        // Stop events (priority 3) — re-iterate timing stops.
        for se in &timing.stops {
            events.push(Event::Stop {
                tick: se.tick,
                duration: se.duration,
            });
        }

        // Scroll events (priority 4).
        collect_scroll_events(bms, &table, &mut events);

        // Stable sort preserves insertion order at the same tick.
        events.sort_by_key(bmsrs_chart::Event::tick);

        Ok(Chart {
            metadata: build_metadata(bms),
            resolution: RESOLUTION,
            timing,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            events,
            audio_assets,
            bga_resources,
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
    pub fn process_default(
        bms: &Bms,
    ) -> Result<Chart<(), bmsrs_chart::NoCustomEvent>, ProcessError> {
        Self::process::<Bme>(bms)
    }
}

// Builder helpers

/// Determine LN mode and pair LNs. Returns paired LNs and consumed note indices.
fn pair_long_notes(bms: &Bms, table: &MeasureTable) -> (Vec<PairedLn>, BTreeSet<usize>) {
    if let Some(ln_obj) = bms.gameplay.ln_obj {
        return pair_lnobj(&bms.messages.note_events, ln_obj, table);
    }
    if bms.gameplay.ln_type == Some(bms_tokenizer::LnType::Type2) {
        return (
            pair_lntype2(&bms.messages.long_note_events, table),
            BTreeSet::new(),
        );
    }
    (
        pair_lntype1(&bms.messages.long_note_events, table),
        BTreeSet::new(),
    )
}

/// Collect all playable notes into the events vec.
fn collect_notes<L: BmsLayout>(
    bms: &Bms,
    table: &MeasureTable,
    wav_map: &BTreeMap<WavIndex, u32>,
    paired_lns: &[PairedLn],
    consumed: &BTreeSet<usize>,
    events: &mut Vec<Event<(), bmsrs_chart::NoCustomEvent>>,
) {
    let push_note = |tick: u64,
                     side,
                     lane,
                     kind: NoteKind,
                     audio: Option<u32>,
                     ev: &mut Vec<Event<(), bmsrs_chart::NoCustomEvent>>| {
        if let Some((note_side, note_lane)) = BmsChannel::new(side, lane).and_then(L::map_channel) {
            ev.push(Event::Note {
                tick,
                side: note_side,
                lane: note_lane,
                kind,
                audio_index: audio,
                ext: (),
            });
        }
    };

    // Visible notes (skip consumed LNOBJ pairs).
    for (i, ne) in bms.messages.note_events.iter().enumerate() {
        if consumed.contains(&i) {
            continue;
        }
        if ne.key_type == KeyType::Visible {
            push_note(
                table.position_to_tick(ne.position),
                ne.player,
                ne.lane,
                NoteKind::Normal,
                wav_map.get(&ne.wav_id).copied(),
                events,
            );
        }
    }

    // Invisible notes (keysounds).
    for ne in &bms.messages.note_events {
        if ne.key_type == KeyType::Invisible {
            push_note(
                table.position_to_tick(ne.position),
                ne.player,
                ne.lane,
                NoteKind::Invisible,
                wav_map.get(&ne.wav_id).copied(),
                events,
            );
        }
    }

    // Paired long notes.
    for ln in paired_lns {
        push_note(
            ln.tick,
            ln.player,
            ln.lane,
            NoteKind::Long {
                duration: ln.duration,
            },
            wav_map.get(&ln.wav_id).copied(),
            events,
        );
    }

    // Mines.
    for me in &bms.messages.mine_events {
        push_note(
            table.position_to_tick(me.position),
            me.player,
            me.lane,
            NoteKind::Mine { damage: 1.0 },
            None,
            events,
        );
    }
}

/// Collect BGM events into the events vec.
fn collect_bgm(
    bms: &Bms,
    table: &MeasureTable,
    wav_map: &BTreeMap<WavIndex, u32>,
    events: &mut Vec<Event<(), bmsrs_chart::NoCustomEvent>>,
) {
    for be in &bms.messages.bgm_events {
        if let Some(&audio) = wav_map.get(&be.wav_id) {
            events.push(Event::Bgm {
                tick: table.position_to_tick(be.position),
                audio_index: audio,
            });
        }
    }
}

/// Build BPM events (for the unified timeline).
fn collect_bpm_events(
    bms: &Bms,
    table: &MeasureTable,
    events: &mut Vec<Event<(), bmsrs_chart::NoCustomEvent>>,
) {
    for bc in &bms.messages.bpm_changes {
        events.push(Event::Bpm {
            tick: table.position_to_tick(bc.position),
            bpm: resolve_bpm(bc.value, bms),
        });
    }
}

/// Build scroll-speed change events.
fn collect_scroll_events(
    bms: &Bms,
    table: &MeasureTable,
    events: &mut Vec<Event<(), bmsrs_chart::NoCustomEvent>>,
) {
    for se in &bms.messages.scroll_events {
        if let Some(&rate) = bms.timing.scroll_defs.get(&se.scroll_id) {
            events.push(Event::Scroll {
                tick: table.position_to_tick(se.position),
                rate,
            });
        }
    }
}

/// Build BGA events from BGA events and BMP file definitions.
fn collect_bga(
    bms: &Bms,
    table: &MeasureTable,
    bmp_map: &BTreeMap<BmpIndex, (u32, String)>,
    events: &mut Vec<Event<(), bmsrs_chart::NoCustomEvent>>,
) {
    for be in &bms.messages.bga_events {
        if let Some(&(resource_id, _)) = bmp_map.get(&be.bmp_id) {
            events.push(Event::Bga {
                tick: table.position_to_tick(be.position),
                layer: be.layer,
                resource_id,
            });
        }
    }
}

/// Build WAV audio assets and return lookup map + assets vec.
fn build_audio_assets(
    wav_files: &BTreeMap<WavIndex, String>,
) -> (BTreeMap<WavIndex, u32>, Vec<AudioAsset>) {
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

/// Build BMP index to (`resource_id`, file path) map.
fn build_bmp_map(bmp_files: &BTreeMap<BmpIndex, String>) -> BTreeMap<BmpIndex, (u32, String)> {
    let mut map = BTreeMap::new();
    #[expect(clippy::cast_possible_truncation, reason = "BMP count fits in u32")]
    for (i, (id, path)) in bmp_files.iter().enumerate() {
        map.insert(*id, (i as u32, path.clone()));
    }
    map
}

/// Build BPM change events for timing track.
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

/// Resolve a [`BpmValue`] to a concrete BPM, using the BPM definition table
/// for reference values.
fn resolve_bpm(value: BpmValue, bms: &Bms) -> f64 {
    match value {
        BpmValue::Absolute(bpm) => bpm,
        BpmValue::Reference(id) => bms.timing.bpm_defs.get(&id).copied().unwrap_or(120.0),
    }
}

/// Find the BPM active at a given tick (last BPM change at or before `tick`).
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

/// Building stop events from `#STOPxx` definitions (channel `09`).
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

/// Build auto 4/4 bar events (one per measure).
fn build_bar_events(max_measure: u16) -> Vec<Event<(), bmsrs_chart::NoCustomEvent>> {
    let step = RESOLUTION * 4;
    (0..=u64::from(max_measure))
        .map(|i| Event::Bar { tick: i * step })
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
