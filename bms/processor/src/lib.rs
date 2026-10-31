//! BMS to `Chart` conversion processor.
//!
//! [`BmsProcessor`] converts `bmsrs_bms_parser::Bms` into a format-agnostic `Chart`
//! by applying a [`BmsLayout`] mode family (such as [`Bme`], `Pms`, or
//! `Nanasi`).
//!
//! Mode families and the [`BmsLayout`] trait live in this crate's `layout`
//! module.
//!
//! # Pipeline
//!
//! ```text
//! bmsrs_bms_parser::Bms -> BmsProcessor::process::<L>(bms) -> Chart<(), NoCustomEvent>
//! ```
//!
//! # Mode families
//!
//! The layout type determines how BMS `(player, lane)` channel bytes decode to
//! note positions. [`Bme`] covers beat-5k/7k/10k/14k uniformly (a chart's key
//! count is determined by the lanes its notes actually use). Other families
//! (`Pms`, `PmsBme`, `Nanasi`, `DscOctFp`) cover their namesake modes.
//! [`BmsProcessor::process_default`] uses [`Bme`].
//!
//! # Long-note modes
//!
//! BMS supports two LN (long-note) notations, selected automatically:
//!
//! - **LNOBJ**: when `#LNOBJ` is defined, ordinary notes paired by the
//!   designated WAV index form long notes.
//! - **LNTYPE 1 (RDM)**: events on channels 51-69 are paired consecutively per
//!   `(player, lane)`.

pub mod custom_event;
mod error;
mod long_note;
mod position;

pub mod layout;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bmsrs_bms_parser::{
    Audio, BmpIndex, BmsBgaLayer, BmsChannel as RawChannel, ChangeOptionIndex, ExRankIndex, LnMode,
    OwnedExWavParams, PlayerMode, Rank, SeekIndex, WavCmdParams, WavIndex,
};
use bmsrs_bms_parser::{Bms, BpmValue, Gameplay, KeyType};
use bmsrs_chart::{
    AudioAsset, AudioIndex, BgaResource, BgaResourceId, BpmChange, BpmLookup, Chart, ChartData,
    ChartInfo, CropRect, Damage, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint,
    NoteAudio, NoteKind, NoteRole, NoteSide, SongInfo, StopEvent, TimingTrack, TimingTrackError,
    VideoAsset,
};
use thiserror::Error;

pub use crate::error::ProcessWarning;

use crate::custom_event::BmsCustomEvent;
use crate::layout::{Bme, BmsLayout, ChannelPos, DscOctFp, Pms, PmsBme, PmsLayout};

use crate::long_note::{LnPairingResult, PairedLn, pair_lnobj, pair_lntype1, pair_lntype2};
use crate::position::MeasureTable;

/// Errors that can occur while processing BMS.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// Timing track construction failed (invalid initial BPM or zero
    /// resolution).
    ///
    /// Forwards [`TimingTrackError`] instead of translating it at the boundary:
    /// `ZeroResolution` used to be translated into `InvalidBpm(0.0)`, producing
    /// a misleading error.
    #[error(transparent)]
    Timing(#[from] TimingTrackError),
    /// Initial BPM is zero.
    ///
    /// In some implementations BMS `#BPM 0` means the chart does not scroll, but
    /// the current time model cannot represent zero BPM. It is rejected here
    /// instead of silently falling back to 130.
    #[error("initial BPM must not be zero; BMS #BPM 0 is not supported")]
    ZeroBpm,
}

/// Unified event type produced by the BMS processor.
///
/// The generic parameters are fixed to `((), BmsCustomEvent)`, where
/// `BmsCustomEvent` carries engine-specific events (BGA opacity, ARGB, TEXT,
/// and so on).
type BmsEvent = Event<(), BmsCustomEvent>;

/// Zero-sized processor that converts [`Bms`] into [`Chart`].
pub struct BmsProcessor;

/// BMS processing result.
#[derive(Debug)]
pub struct Processed {
    /// Chart produced by processing.
    pub chart: Chart<(), BmsCustomEvent>,
    /// Recoverable warnings collected during processing.
    pub warnings: Vec<ProcessWarning>,
    /// Raw BMS gameplay parameters that do not enter the core `Chart`.
    pub gameplay: BmsGameplay,
    /// Raw BMS audio parameters that do not enter the core `AudioAsset`.
    pub audio: BmsAudio,
}

/// Sidecar for BMS gameplay parameters.
///
/// The conversion rules for these fields depend on the specific engine, so they
/// do not enter the format-agnostic `Chart`; consumers that need the raw values
/// read them from [`Processed::gameplay`].
#[derive(Debug, Clone, PartialEq)]
pub struct BmsGameplay {
    /// Game mode (`#PLAYER`).
    pub player: Option<PlayerMode>,
    /// Judgment difficulty (`#RANK`).
    pub rank: Option<Rank>,
    /// Fine-grained judgment difficulty (`#DEFEXRANK`).
    pub def_ex_rank: Option<f64>,
    /// Maximum life gauge increase (`#TOTAL`).
    pub total: Option<f64>,
    /// Master volume percentage (`#VOLWAV`).
    pub vol_wav: Option<f64>,
    /// Per-position judgment width overrides (`#EXRANKxx`).
    pub ex_rank_defs: BTreeMap<ExRankIndex, f64>,
    /// Engine-specific option string (`#OPTION`).
    pub option: Option<String>,
    /// Dynamic option change definitions (`#CHANGEOPTIONxx`).
    pub change_option_defs: BTreeMap<ChangeOptionIndex, String>,
}

impl From<&Gameplay> for BmsGameplay {
    fn from(gameplay: &Gameplay) -> Self {
        Self {
            player: gameplay.player,
            rank: gameplay.rank,
            def_ex_rank: gameplay.def_ex_rank,
            total: gameplay.total,
            vol_wav: gameplay.vol_wav,
            ex_rank_defs: gameplay.ex_rank_defs.clone(),
            option: gameplay.option.clone(),
            change_option_defs: gameplay.change_option_defs.clone(),
        }
    }
}

/// Sidecar for BMS audio parameters.
///
/// `#PATH_WAV` has already been applied to
/// [`AudioAsset::path`](bmsrs_chart::AudioAsset::path); this sidecar keeps the
/// original prefix and the other parameters that do not enter the core model.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "sidecar keeps raw BMS parameters; some fields cannot safely implement Eq"
)]
#[derive(Debug, Clone, PartialEq)]
pub struct BmsAudio {
    /// Per-WAV effect parameters from `#EXWAV`.
    pub ex_wav_params: BTreeMap<WavIndex, OwnedExWavParams>,
    /// `#WAVCMD` global playback command.
    pub wav_cmd: Option<WavCmdParams>,
    /// `#CDDA` track reference.
    pub cdda: Option<String>,
    /// `#MIDIFILE` reference.
    pub midifile: Option<String>,
    /// Original `#PATH_WAV` directory prefix.
    pub path_wav: Option<String>,
}

impl From<&Audio> for BmsAudio {
    fn from(audio: &Audio) -> Self {
        Self {
            ex_wav_params: audio.ex_wav_params.clone(),
            wav_cmd: audio.wav_cmd.clone(),
            cdda: audio.cdda.clone(),
            midifile: audio.midifile.clone(),
            path_wav: audio.path_wav.clone(),
        }
    }
}

/// Default beat resolution for BMS charts (ticks per quarter note).
const RESOLUTION: u64 = 240;

impl BmsProcessor {
    /// Process a BMS chart with an explicit mode-family layout and return
    /// processing warnings.
    ///
    /// The layout type determines how each BMS `(player, lane)` channel byte
    /// decodes to a note position. Available families are listed in the
    /// `layout` module.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Timing`] when timing track construction fails
    /// (initial BPM missing, zero, or non-finite).
    pub fn process<L>(bms: &Bms) -> Result<Processed, ProcessError>
    where
        L: BmsLayout,
    {
        let init_bpm = bms.timing.bpm.unwrap_or(130.0);
        if init_bpm == 0.0 {
            return Err(ProcessError::ZeroBpm);
        }

        let max_measure = find_max_measure(bms);
        let table = MeasureTable::new(max_measure, &bms.messages.measure_lengths, RESOLUTION);
        let mut conv = BmsConverter {
            bms,
            table: &table,
            events: Vec::with_capacity(1024),
            warnings: Vec::new(),
        };

        let (wav_map, audio_assets) = build_audio_assets(&bms.audio);
        let bpm_changes = conv.build_bpm_changes();
        let bpm_lookup = BpmLookup::new(init_bpm, &bpm_changes);

        let mut stops = conv.build_stops_from_defs();
        stops.extend(conv.build_stops_from_stp(&bpm_lookup));
        stops.sort_by_key(|s| s.tick);

        let timing = TimingTrack::new(init_bpm, bpm_changes, stops, RESOLUTION)?;

        let bmp_map = conv.build_bmp_map();

        let (ln_result, consumed) = conv.pair_long_notes();
        let paired_lns = ln_result.paired;
        let unpaired_starts = ln_result.unpaired_starts;

        // Collect unpaired long-note warnings.
        for &(player, lane, tick) in &unpaired_starts {
            conv.warnings
                .push(ProcessWarning::UnterminatedLongNote { player, lane, tick });
        }

        // Barlines first (priority 0).
        conv.events.extend(build_bar_events(&table));

        // Notes (priority 1).
        conv.collect_notes::<L>(&wav_map, &paired_lns, &consumed, &unpaired_starts);

        // BGM (priority 1).
        conv.collect_bgm(&wav_map);

        // LNOBJ end markers play as BGM (per the BMS specification).
        conv.collect_lnobj_bgm(&wav_map, &consumed);

        // BGA (priority 1).
        conv.collect_bga(&bmp_map);

        // BPM changes (priority 2).
        conv.collect_bpm_events();

        // Stop events (priority 3): iterate the timing stop events again.
        conv.collect_stop_events(timing.stops());

        // SCROLL events (priority 4).
        conv.collect_scroll_events();

        // SPEED events (priority 5).
        conv.collect_speed_events();

        // BMS engine-specific custom events (priority 6).
        conv.collect_custom_events(&bmp_map);

        let (song, chart_info) = conv.build_metadata(&bmp_map);
        let ln_type_hint = conv.ln_type_hint();

        let mut data = ChartData {
            timing,
            judge_multiplier: None,
            life_multiplier: None,
            ln_type_hint,
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: conv.events,
            audio_assets,
        };
        data.sort_events();

        // Warning aggregation: parser-layer value parse warnings come first (by
        // file position), followed by processor-layer missing-reference and
        // clamping warnings. Taken after the build_metadata borrow ends.
        let mut warnings: Vec<ProcessWarning> = bms
            .messages
            .warnings
            .iter()
            .cloned()
            .map(ProcessWarning::UnparsableValue)
            .collect();
        warnings.extend(conv.warnings);
        let gameplay = BmsGameplay::from(&bms.gameplay);
        let audio = BmsAudio::from(&bms.audio);

        Ok(Processed {
            chart: Chart {
                song,
                chart: chart_info,
                data,
            },
            warnings,
            gameplay,
            audio,
        })
    }

    /// Process a BMS chart with an automatically detected PMS variant layout.
    ///
    /// Automatically selects [`Pms`] (Standard) or [`PmsBme`] (BME-type) based
    /// on the channels used in the chart.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Timing`] when timing track construction fails
    /// (initial BPM missing, zero, or non-finite).
    pub fn process_pms(bms: &Bms) -> Result<Processed, ProcessError> {
        match PmsLayout::detect(bms) {
            PmsLayout::Standard => Self::process::<Pms>(bms),
            PmsLayout::BmeType => Self::process::<PmsBme>(bms),
        }
    }

    /// Process a BMS chart with the default layout.
    ///
    /// Uses [`DscOctFp`] when the chart declares `#OCT/FP`; otherwise uses
    /// [`Bme`]. `Bme` covers beat-5k/7k/10k/14k uniformly: both player sides are
    /// mapped, so a single-player chart simply leaves the 2P lanes empty while a
    /// two-player chart fills them. The `#PLAYER` header command does not affect
    /// the mapping (matching modern engines, which ignore it).
    ///
    /// For PMS, nanasi, or other families, call
    /// [`process::<Nanasi>`](Self::process) with the appropriate layout type.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Timing`] when timing track construction fails
    /// (initial BPM missing, zero, or non-finite).
    pub fn process_default(bms: &Bms) -> Result<Processed, ProcessError> {
        if bms.gameplay.oct_fp == Some(true) {
            Self::process::<DscOctFp>(bms)
        } else {
            Self::process::<Bme>(bms)
        }
    }
}

/// BMS to [`Chart`] conversion context.
///
/// Holds the source document [`Bms`], the measure tick table [`MeasureTable`],
/// and the event buffer being built, so each `collect_*` step writes directly to
/// the internal `events` without the caller passing `&mut Vec`. Used only inside
/// [`BmsProcessor::process`].
struct BmsConverter<'a> {
    /// Source BMS document.
    bms: &'a Bms,
    /// Measure to cumulative tick lookup table.
    table: &'a MeasureTable,
    /// Event vector being built, eventually taken by
    /// [`BmsProcessor::process`].
    events: Vec<BmsEvent>,
    /// Recoverable warnings collected during processing.
    warnings: Vec<ProcessWarning>,
}

impl BmsConverter<'_> {
    /// Determine the LN mode and pair long notes. Returns the pairing result and
    /// the consumed note indices.
    fn pair_long_notes(&self) -> (LnPairingResult, HashSet<usize>) {
        if let Some(ln_obj) = self.bms.gameplay.ln_obj {
            let (paired, consumed) = pair_lnobj(&self.bms.messages.note_events, ln_obj, self.table);
            return (
                LnPairingResult {
                    paired,
                    unpaired_starts: Vec::new(),
                },
                consumed,
            );
        }
        if self.bms.gameplay.ln_type == Some(bmsrs_bms_parser::LnType::Type2) {
            return (
                pair_lntype2(&self.bms.messages.long_note_events, self.table),
                HashSet::new(),
            );
        }
        (
            pair_lntype1(&self.bms.messages.long_note_events, self.table),
            HashSet::new(),
        )
    }

    /// Derive the chart-level long-note type hint from the `#LNMODE` header.
    ///
    /// `LnMode{Ln,Cn,Hcn}` and `LnTypeHint{Ln,Cn,Hcn}` are 1:1 isomorphic enums.
    /// `None` (no `#LNMODE` declared) maps to [`LnTypeHint::Ln`], consistent with
    /// the former `LnTypeHint::default()` but now explicit.
    const fn ln_type_hint(&self) -> LnTypeHint {
        match self.bms.gameplay.ln_mode {
            Some(LnMode::Ln) | None => LnTypeHint::Ln,
            Some(LnMode::Cn) => LnTypeHint::Cn,
            Some(LnMode::Hcn) => LnTypeHint::Hcn,
        }
    }
    /// Collect all playable notes into the event vector.
    #[expect(
        clippy::too_many_lines,
        reason = "normal/invisible/LNOBJ/long-note/mine sources share one matching logic; splitting would fragment it"
    )]
    fn collect_notes<L: BmsLayout>(
        &mut self,
        wav_map: &BTreeMap<WavIndex, AudioIndex>,
        paired_lns: &[PairedLn],
        consumed: &HashSet<usize>,
        unpaired_starts: &[(u8, u8, u64)],
    ) {
        let push_note = |tick: u64,
                         side,
                         lane,
                         kind: NoteKind,
                         audio: Option<AudioIndex>,
                         ev: &mut Vec<BmsEvent>| {
            if let Some((note_side, note_lane)) =
                ChannelPos::new(side, lane).and_then(L::map_channel)
            {
                ev.push(Event::new(
                    tick,
                    EventKind::Note {
                        side: note_side,
                        lane: note_lane,
                        kind,
                        role: NoteRole::Normal,
                        audio: NoteAudio::from(audio),
                        ext: (),
                    },
                ));
            }
        };

        let mut check_wav =
            |wav_id: &WavIndex, player: u8, lane: u8, tick: u64| -> Option<AudioIndex> {
                let audio = wav_map.get(wav_id).copied();
                // When `to_index()` returns `None`, skip the warning silently:
                // a non-decomposable WAV key (such as the "ZZ" LNOBJ marker) is
                // outside the WAV table's definition range and does not produce
                // a missing-definition warning.
                if audio.is_none()
                    && let Some(key) = wav_id.0.to_index()
                {
                    self.warnings.push(ProcessWarning::MissingWavDefinition {
                        key,
                        player,
                        lane,
                        tick,
                    });
                }
                audio
            };

        // Build the LN range table:
        // (player, lane) to Vec<(start_tick, end_tick)>.
        let ln_ranges = build_ln_ranges(paired_lns);

        // Playable notes (skip consumed LNOBJ pairs; visible notes are also
        // suppressed by LN ranges).
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i) {
                continue;
            }
            let tick = self.table.position_to_tick(ne.position);
            match ne.key_type {
                KeyType::Visible => {
                    if is_note_suppressed(ne.player, ne.lane, tick, &ln_ranges, unpaired_starts) {
                        continue;
                    }
                    push_note(
                        tick,
                        ne.player,
                        ne.lane,
                        NoteKind::Normal,
                        check_wav(&ne.wav_id, ne.player, ne.lane, tick),
                        &mut self.events,
                    );
                }
                KeyType::Invisible => push_note(
                    tick,
                    ne.player,
                    ne.lane,
                    NoteKind::Invisible,
                    check_wav(&ne.wav_id, ne.player, ne.lane, tick),
                    &mut self.events,
                ),
            }
        }

        // In LNOBJ mode, ch51-69 long-note channels are mutually exclusive with
        // LNOBJ (memo/10 leaves this undefined). Do not discard silently; keep
        // the data as ordinary visible notes.
        if self.bms.gameplay.ln_obj.is_some() {
            for lne in &self.bms.messages.long_note_events {
                let tick = self.table.position_to_tick(lne.position);
                push_note(
                    tick,
                    lne.player,
                    lne.lane,
                    NoteKind::Normal,
                    check_wav(&lne.wav_id, lne.player, lne.lane, tick),
                    &mut self.events,
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
                check_wav(&ln.wav_id, ln.player, ln.lane, ln.tick),
                &mut self.events,
            );
        }

        // Mines.
        for me in &self.bms.messages.mine_events {
            push_note(
                self.table.position_to_tick(me.position),
                me.player,
                me.lane,
                NoteKind::Mine {
                    damage: Damage::new(mine_damage(me.raw_value)),
                },
                None,
                &mut self.events,
            );
        }
    }

    /// Collect BGM events into the event vector.
    fn collect_bgm(&mut self, wav_map: &BTreeMap<WavIndex, AudioIndex>) {
        for be in &self.bms.messages.bgm_events {
            if let Some(&audio) = wav_map.get(&be.wav_id) {
                self.events
                    .push(Event::bgm(self.table.position_to_tick(be.position), audio));
            }
        }
    }

    /// Collect BGM events for LNOBJ end markers.
    ///
    /// Per the BMS specification, when an
    /// [`#LNOBJ`](bmsrs_bms_parser::BmsHeaderGameplay::LnObj) end marker crosses the
    /// judgment line, its WAV file plays as BGM. This function walks the
    /// `consumed` note indices returned by [`pair_lnobj`] and emits one BGM event
    /// for each end marker.
    fn collect_lnobj_bgm(
        &mut self,
        wav_map: &BTreeMap<WavIndex, AudioIndex>,
        consumed: &HashSet<usize>,
    ) {
        let Some(ln_obj) = self.bms.gameplay.ln_obj else {
            return;
        };
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i)
                && *ne.wav_id == *ln_obj
                && let Some(&audio) = wav_map.get(&ne.wav_id)
            {
                self.events
                    .push(Event::bgm(self.table.position_to_tick(ne.position), audio));
            }
        }
    }

    /// Build BPM events (for the unified timeline).
    ///
    /// Invalid values (zero, NaN, infinity; predicate
    /// [`BpmChange::is_valid`]) are skipped with a warning, matching the filter
    /// semantics of timing track construction.
    fn collect_bpm_events(&mut self) {
        for bc in &self.bms.messages.bpm_changes {
            let tick = self.table.position_to_tick(bc.position);
            let bpm = self.resolve_bpm(bc.value, tick);
            if BpmChange::is_valid(bpm) {
                self.events.push(Event::bpm(tick, bpm));
            } else {
                self.warnings
                    .push(ProcessWarning::InvalidBpmChangeIgnored { tick, bpm });
            }
        }
    }

    /// Build SCROLL change events.
    fn collect_scroll_events(&mut self) {
        for se in &self.bms.messages.scroll_events {
            if let Some(&rate) = self.bms.timing.scroll_defs.get(&se.scroll_id) {
                self.events.push(Event::scroll(
                    self.table.position_to_tick(se.position),
                    rate,
                ));
            }
        }
    }

    /// Build SPEED (visual note spacing) keyframe events.
    fn collect_speed_events(&mut self) {
        for se in &self.bms.messages.speed_events {
            if let Some(&rate) = self.bms.timing.speed_defs.get(&se.speed_id) {
                self.events
                    .push(Event::speed(self.table.position_to_tick(se.position), rate));
            }
        }
    }

    /// Emit stop events from the already-built stop event slice.
    fn collect_stop_events(&mut self, stops: &[StopEvent]) {
        for se in stops {
            self.events.push(Event::stop(se.tick, se.duration));
        }
    }

    /// Collect BMS engine-specific custom events.
    ///
    /// Reads the merged, already-normalized channel data from
    /// [`Messages::non_event_data`] and converts it to [`EventKind::Custom`]
    /// variants. Every non-`"00"` value produces one event.
    ///
    /// **Normalization guarantee**: the parser layer has already normalized every
    /// index according to the pre-scanned `#BASE`; lookups here do not need to
    /// normalize again (removing the duplicate normalization previously marked
    /// F1/F2).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "channel values are single-byte bounded (0-255) or u32 resource indices"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "single structured match on channel variant with short branches; extraction would lose clarity"
    )]
    fn collect_custom_events(&mut self, bmp_map: &BTreeMap<BmpIndex, BmpEntry>) {
        let non_event = &self.bms.messages.non_event_data;

        // BGA layer mapping (channel to layer).
        let layer_of = |ch: RawChannel| -> Option<bmsrs_chart::BgaLayer> {
            match ch {
                RawChannel::BgaBaseOpacity | RawChannel::BgaArgbBase => {
                    Some(bmsrs_chart::BgaLayer::Base)
                }
                RawChannel::BgaLayerOpacity | RawChannel::BgaArgbLayer => {
                    Some(bmsrs_chart::BgaLayer::Layer)
                }
                RawChannel::BgaLayer2Opacity | RawChannel::BgaArgbLayer2 => {
                    Some(bmsrs_chart::BgaLayer::Layer2)
                }
                RawChannel::BgaPoorOpacity | RawChannel::BgaArgbPoor => {
                    Some(bmsrs_chart::BgaLayer::Poor)
                }
                _ => None,
            }
        };

        for item in non_event {
            let total = item.values.len() as u32;
            if total == 0 {
                continue;
            }

            for (i, obj) in item.values.iter().enumerate() {
                if obj.as_str() == "00" {
                    continue;
                }
                // Value channels (0B-0E, 97, 98) use hexadecimal (01-FF); index
                // channels (99, A0, A1-A5, A6, 05) use Base36 indices. Every
                // value has already been normalized by base in the parser layer,
                // so lookups need no further normalization.
                let tick = self.table.position_to_tick(bmsrs_bms_parser::Position::new(
                    item.measure,
                    i as u32,
                    total,
                ));

                let payload = match item.channel {
                    RawChannel::BgaBaseOpacity
                    | RawChannel::BgaLayerOpacity
                    | RawChannel::BgaLayer2Opacity
                    | RawChannel::BgaPoorOpacity => {
                        let Some(layer) = layer_of(item.channel) else {
                            continue;
                        };
                        let Some(opacity) = obj.as_u8_hex() else {
                            continue;
                        };
                        BmsCustomEvent::BgaOpacity { layer, opacity }
                    }
                    RawChannel::BgaArgbBase
                    | RawChannel::BgaArgbLayer
                    | RawChannel::BgaArgbLayer2
                    | RawChannel::BgaArgbPoor => {
                        let Some(layer) = layer_of(item.channel) else {
                            continue;
                        };
                        // The value is normalized; look it up directly.
                        let Some((a, r, g, b)) = self
                            .bms
                            .visual
                            .argb_defs
                            .get(&BmpIndex::from(*obj))
                            .cloned()
                            .map(|p| (p.a, p.r, p.g, p.b))
                        else {
                            continue;
                        };
                        BmsCustomEvent::BgaArgb { layer, a, r, g, b }
                    }
                    RawChannel::BgaKeyBound => {
                        // The value is normalized; look it up in bmp_map
                        // directly.
                        let Some(resource_id) =
                            bmp_map.get(&BmpIndex::from(*obj)).map(|e| e.resource_id)
                        else {
                            continue;
                        };
                        BmsCustomEvent::BgaKeyBound { resource_id }
                    }
                    RawChannel::Text => {
                        let index = u64::from(obj.to_index().unwrap_or(0));
                        BmsCustomEvent::TextDisplay {
                            text_index: index as u32,
                        }
                    }
                    RawChannel::Judge => {
                        let index = u64::from(obj.to_index().unwrap_or(0));
                        BmsCustomEvent::JudgeOverride { rank: index }
                    }
                    RawChannel::Option => {
                        let option_id = u64::from(obj.to_index().unwrap_or(0));
                        // The value is normalized; look it up in
                        // change_option_defs directly (F1+F2 are already
                        // resolved in the parser).
                        let opt_idx = ChangeOptionIndex::from(*obj);
                        let value = self
                            .bms
                            .gameplay
                            .change_option_defs
                            .get(&opt_idx)
                            .cloned()
                            .unwrap_or_default();
                        BmsCustomEvent::OptionChange { option_id, value }
                    }
                    RawChannel::BgmVolume => {
                        let Some(volume) = obj.as_u8_hex() else {
                            continue;
                        };
                        BmsCustomEvent::BgmVolume { volume }
                    }
                    RawChannel::KeyVolume => {
                        let Some(volume) = obj.as_u8_hex() else {
                            continue;
                        };
                        BmsCustomEvent::KeyVolume { volume }
                    }
                    RawChannel::Seek => {
                        // The value is normalized; look it up in seek_defs
                        // directly.
                        let Some(ms) = self
                            .bms
                            .visual
                            .seek_defs
                            .get(&SeekIndex::from(*obj))
                            .copied()
                        else {
                            continue;
                        };
                        // VideoSeek.position is u64 (BmsCustomEvent derives Eq);
                        // milliseconds are non-negative by semantics, so clamp
                        // before converting.
                        #[expect(
                            clippy::cast_sign_loss,
                            reason = "clamped to non-negative before cast"
                        )]
                        #[expect(clippy::cast_possible_truncation, reason = "seek ms fits in u64")]
                        let position = ms.max(0.0).round() as u64;
                        BmsCustomEvent::VideoSeek { position }
                    }
                    _ => continue,
                };
                self.events
                    .push(Event::new(tick, EventKind::Custom(payload)));
            }
        }
    }

    /// Build BGA events from BGA events and BMP file definitions.
    fn collect_bga(&mut self, bmp_map: &BTreeMap<BmpIndex, BmpEntry>) {
        for be in &self.bms.messages.bga_events {
            if let Some(entry) = bmp_map.get(&be.bmp_id) {
                self.events.push(Event::new(
                    self.table.position_to_tick(be.position),
                    EventKind::Bga {
                        layer: map_bga_layer(be.layer),
                        resource_id: entry.resource_id,
                    },
                ));
            }
        }
    }

    /// Build BPM change events for the timing track.
    ///
    /// Sort by `tick` first, then deduplicate the ordered sequence: adjacent
    /// changes with the same value keep the first. The order cannot be reversed:
    /// `messages.bpm_changes` accumulates by (measure, channel) rather than by
    /// tick, so deduplicating before sorting would merge equal-value changes that
    /// are adjacent in the vector but not adjacent in time.
    fn build_bpm_changes(&mut self) -> Vec<BpmChange> {
        let mut changes: Vec<BpmChange> = self
            .bms
            .messages
            .bpm_changes
            .iter()
            .filter_map(|bc| {
                let tick = self.table.position_to_tick(bc.position);
                let bpm = self.resolve_bpm(bc.value, tick);
                BpmChange::is_valid(bpm).then_some(BpmChange { tick, bpm })
            })
            .collect();
        // Deduplication must happen after sorting by tick: two equal-value
        // changes in the same channel are adjacent in the vector (chunked by
        // channel). If a change from another channel sits between them,
        // deduplicating before sorting would merge them incorrectly.
        changes.sort_by_key(|c| c.tick);
        changes.dedup_by(|a, b| (a.bpm - b.bpm).abs() < f64::EPSILON);
        changes
    }

    /// Build a mapping from BMP indices to [`BmpEntry`] (dense id, path, and
    /// optional crop).
    ///
    /// Merges keys from the `#BMP`, `#BGA`, and `#@BGA` namespaces:
    ///
    /// - When an id is defined in `#BGA` / `#@BGA`, `#BGA` wins per the BMS
    ///   specification: the path comes from the crop definition's source BMP
    ///   (resolved through the decimal `bmp_index`), with a [`CropRect`]
    ///   attached.
    /// - Otherwise the path comes from `#BMP` itself and the crop is `None`.
    /// - Crop ids whose source path cannot be resolved are skipped (they enter
    ///   neither the resource table nor the event stream).
    ///
    /// `#@BGA` (width/height form) is normalized to `#BGA` (bottom-right-corner
    /// form): `x2 = sx + w` and `y2 = sy + h`.
    #[expect(clippy::cast_possible_truncation, reason = "BMP count fits in u32")]
    fn build_bmp_map(&self) -> BTreeMap<BmpIndex, BmpEntry> {
        let visual = &self.bms.visual;
        let mut map: BTreeMap<BmpIndex, BmpEntry> = BTreeMap::new();

        // Collect every involved id (bmp_files, crop_defs, alt_crop_defs).
        let mut ids: BTreeSet<BmpIndex> = visual.bmp_files.keys().copied().collect();
        ids.extend(visual.crop_defs.keys().copied());
        ids.extend(visual.alt_crop_defs.keys().copied());

        for (i, id) in ids.into_iter().enumerate() {
            // Crop definition: #BGA takes priority over #@BGA (the same id is
            // unlikely to appear in both). Both give (CropRect, source
            // bmp_index); #@BGA's w/h is normalized to the bottom-right corner.
            let crop_def = visual
                .crop_defs
                .get(&id)
                .map(|p| {
                    (
                        CropRect {
                            x1: p.x1,
                            y1: p.y1,
                            x2: p.x2,
                            y2: p.y2,
                            dx: p.dx,
                            dy: p.dy,
                        },
                        p.bmp_index,
                    )
                })
                .or_else(|| {
                    visual.alt_crop_defs.get(&id).map(|p| {
                        (
                            CropRect {
                                x1: p.sx,
                                y1: p.sy,
                                x2: p.sx + p.w,
                                y2: p.sy + p.h,
                                dx: p.dx,
                                dy: p.dy,
                            },
                            p.bmp_index,
                        )
                    })
                });

            // Path: with a crop definition, use the source BMP (#BGA first);
            // otherwise use #BMP itself.
            let (crop, path_opt) = match crop_def {
                Some((crop, source_index)) => (
                    Some(crop),
                    resolve_bmp_path(&visual.bmp_files, source_index).cloned(),
                ),
                None => (None, visual.bmp_files.get(&id).cloned()),
            };

            let Some(path) = path_opt else {
                continue;
            };

            map.insert(
                id,
                BmpEntry {
                    resource_id: BgaResourceId::new(i as u32),
                    path,
                    crop,
                },
            );
        }

        map
    }

    /// Resolve a [`BpmValue`] into a concrete BPM, using the BPM definition table
    /// for references.
    fn resolve_bpm(&mut self, value: BpmValue, tick: u64) -> f64 {
        match value {
            BpmValue::Absolute(bpm) => bpm,
            BpmValue::Reference(id) => {
                if let Some(bpm) = self.bms.timing.bpm_defs.get(&id).copied() {
                    bpm
                } else {
                    if let Some(key) = id.to_index() {
                        self.warnings
                            .push(ProcessWarning::MissingBpmDefinition { key, tick });
                    }
                    120.0
                }
            }
        }
    }

    /// Build stop events from `#STOPxx` definitions (channel `09`).
    ///
    /// Negative STOP values are clamped to `0` (skipped/ignored), matching
    /// beatoraja and Angolmois behavior.
    #[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    fn build_stops_from_defs(&mut self) -> Vec<StopEvent> {
        self.bms
            .messages
            .stop_events
            .iter()
            .filter_map(|se| {
                let tick = self.table.position_to_tick(se.position);
                if let Some(&raw) = self.bms.timing.stop_defs.get(&se.stop_id) {
                    let duration = (raw / 192.0 * RESOLUTION as f64 * 4.0).round();
                    if duration < 0.0 {
                        self.warnings.push(ProcessWarning::StopDurationClipped {
                            tick,
                            original: duration,
                        });
                    }
                    Some(StopEvent {
                        tick,
                        duration: duration.max(0.0) as u64,
                    })
                } else {
                    if let Some(key) = se.stop_id.to_index() {
                        self.warnings
                            .push(ProcessWarning::MissingStopDefinition { key, tick });
                    }
                    None
                }
            })
            .collect()
    }

    /// Build stop events from the `#STP` header command (duration in
    /// milliseconds).
    ///
    /// Negative STOP values are clamped to `0` (skipped/ignored), matching
    /// beatoraja and Angolmois behavior.
    #[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    fn build_stops_from_stp(&self, bpm_lookup: &BpmLookup<'_>) -> Vec<StopEvent> {
        self.bms
            .messages
            .stp_events
            .iter()
            .map(|stp| {
                let tick = self.table.position_to_tick(stp.position);
                let bpm = bpm_lookup.bpm_at_tick(tick);
                let tick_duration = stp.duration_ms / 1000.0 * bpm / 60.0 * RESOLUTION as f64;
                StopEvent {
                    tick,
                    duration: tick_duration.round().max(0.0) as u64,
                }
            })
            .collect()
    }

    /// Build [`SongInfo`] and [`ChartInfo`] from BMS metadata and the BMP
    /// mapping.
    #[expect(clippy::cast_possible_truncation, reason = "play level fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "play level is non-negative")]
    fn build_metadata(&self, bmp_map: &BTreeMap<BmpIndex, BmpEntry>) -> (SongInfo, ChartInfo) {
        let bga_resources: Vec<BgaResource> = bmp_map
            .values()
            .map(|e| BgaResource {
                path: e.path.clone().into(),
                crop: e.crop,
            })
            .collect();
        let video = self.build_video_asset();
        (
            SongInfo {
                title: self.bms.metadata.title.clone().unwrap_or_default(),
                artist: self.bms.metadata.artist.clone().unwrap_or_default(),
                genre: self.bms.metadata.genre.clone().unwrap_or_default(),
            },
            ChartInfo {
                subtitle: self.bms.metadata.subtitle.clone().unwrap_or_default(),
                chart_name: self
                    .bms
                    .display
                    .play_level
                    .map(|l| format!("{l:.0}"))
                    .unwrap_or_default(),
                level: self.bms.display.play_level.map_or(0, |l| l as u64),
                subartists: self
                    .bms
                    .metadata
                    .sub_artist
                    .as_deref()
                    .map(|s| vec![s.to_owned()])
                    .unwrap_or_default(),
                title_image: None,
                back_image: self.bms.display.back_bmp.clone(),
                eyecatch_image: self.bms.display.stage_file.clone(),
                banner_image: self.bms.display.banner.clone(),
                preview_music: self.bms.display.preview.clone(),
                bga_resources,
                video,
                key_count: key_count(&self.events),
            },
        )
    }

    /// Build a [`VideoAsset`] from `#VIDEOFILE` / `#MOVIE` and video parameters.
    ///
    /// `#VIDEOFILE` (looping) takes priority over `#MOVIE` (single play). Returns
    /// `None` when neither is present. `#VIDEOf/s`, `#VIDEOCOLORS`, and
    /// `#VIDEODLY` are attached as optional parameters; `#VIDEOCOLORS` and
    /// `#VIDEODLY` are semantically integers and are converted from `f64` to
    /// `u32`.
    #[expect(clippy::cast_possible_truncation, reason = "video params fit in u32")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    fn build_video_asset(&self) -> Option<VideoAsset> {
        let visual = &self.bms.visual;
        let (path, loop_playback) = if let Some(p) = &visual.video_file {
            (p.clone(), true)
        } else {
            (visual.movie.clone()?, false)
        };
        let mut asset = VideoAsset::new(PathBuf::from(path), loop_playback);
        if let Some(fps) = visual.video_fps {
            asset = asset.with_fps(fps);
        }
        if let Some(colors) = visual.video_colors {
            asset = asset.with_colors(colors.max(0.0).round() as u32);
        }
        if let Some(delay) = visual.video_dly {
            asset = asset.with_delay_frames(delay.max(0.0).round() as u32);
        }
        Some(asset)
    }
}

/// Map the BMS semantic layer's layer enum to the format-agnostic
/// [`bmsrs_chart::BgaLayer`].
const fn map_bga_layer(layer: BmsBgaLayer) -> bmsrs_chart::BgaLayer {
    match layer {
        BmsBgaLayer::Base => bmsrs_chart::BgaLayer::Base,
        BmsBgaLayer::Poor => bmsrs_chart::BgaLayer::Poor,
        BmsBgaLayer::Layer => bmsrs_chart::BgaLayer::Layer,
        BmsBgaLayer::Layer2 => bmsrs_chart::BgaLayer::Layer2,
    }
}

// Pure conversion helpers (no `&Bms` + `&MeasureTable` dependency)

/// Find the largest measure number referenced by any event.
///
/// Called before constructing [`BmsConverter`]; the result is the input to
/// [`MeasureTable::new`].
fn find_max_measure(bms: &Bms) -> u16 {
    let mut max_m = 0u16;

    for ml in &bms.messages.measure_lengths {
        max_m = max_m.max(ml.measure);
    }

    // Walk every event type and extract position.measure() uniformly.
    macro_rules! track_positions {
        ($($events:expr),+ $(,)?) => {
            $(
                for ev in $events {
                    max_m = max_m.max(ev.position.measure());
                }
            )+
        };
    }

    track_positions!(
        &bms.messages.note_events,
        &bms.messages.long_note_events,
        &bms.messages.bgm_events,
        &bms.messages.mine_events,
        &bms.messages.bpm_changes,
        &bms.messages.stop_events,
        &bms.messages.scroll_events,
        &bms.messages.speed_events,
        &bms.messages.bga_events,
        &bms.messages.stp_events,
    );

    // non_event_data stores NonEventData; walk its measure field.
    for item in &bms.messages.non_event_data {
        max_m = max_m.max(item.measure);
    }

    max_m.max(1) + 1
}

/// Build WAV audio assets and return the lookup table and asset vector.
fn build_audio_assets(audio: &Audio) -> (BTreeMap<WavIndex, AudioIndex>, Vec<AudioAsset>) {
    let mut wav_map = BTreeMap::new();
    let mut audio_assets = Vec::new();
    for (&wav_id, path) in &audio.wav_files {
        #[expect(clippy::cast_possible_truncation, reason = "WAV count fits in u32")]
        let idx = AudioIndex::new(audio_assets.len() as u32);
        let resolved_path = audio.path_wav.as_ref().map_or_else(
            || PathBuf::from(path),
            |prefix| Path::new(prefix).join(path),
        );
        audio_assets.push(AudioAsset {
            path: Arc::from(resolved_path),
            start: Duration::ZERO,
            duration: None,
        });
        wav_map.insert(wav_id, idx);
    }
    (wav_map, audio_assets)
}

/// Entry produced by [`build_bmp_map`](BmsConverter::build_bmp_map): dense
/// resource id, source file path, and optional crop definition.
struct BmpEntry {
    /// Zero-based resource id (the vector index into `bga_resources`).
    resource_id: BgaResourceId,
    /// Source image file path (the underlying `#BMP` that a crop definition
    /// points to).
    path: String,
    /// Crop and placement definition (`#BGA` / `#@BGA`); `None` when absent.
    crop: Option<CropRect>,
}

/// Resolve a decimal BMP number into a file path from the `#BMP` table.
///
/// The `bmp_index` field of `#BGA` / `#@BGA` is the **decimal** number of the
/// source `#BMP`: per the BMS command memo the value is the hexadecimal index
/// converted to decimal (so `16` references `#BMP10`), and only indices
/// `00`-`FF` (0-255) can be referenced. Values above 255 are invalid and
/// resolve to nothing.
fn resolve_bmp_path(bmp_files: &BTreeMap<BmpIndex, String>, bmp_index: u16) -> Option<&String> {
    let hex = u8::try_from(bmp_index).ok()?;
    bmp_files
        .iter()
        .find_map(|(k, v)| (k.as_u8_hex() == Some(hex)).then_some(v))
}

/// Build the LN range lookup table.
fn build_ln_ranges(paired_lns: &[PairedLn]) -> HashMap<(u8, u8), Vec<(u64, u64)>> {
    let mut ln_ranges: HashMap<(u8, u8), Vec<(u64, u64)>> = HashMap::new();
    for ln in paired_lns {
        ln_ranges
            .entry((ln.player, ln.lane))
            .or_default()
            .push((ln.tick, ln.end_tick));
    }
    ln_ranges
}

/// Infer the total number of playable keys from the event stream.
///
/// Takes the maximum `Lane::Key` number per side and adds the two sides; returns
/// `None` when there are no key notes. Scratches, foot pedals, and mines do not
/// count toward the key count.
fn key_count(events: &[BmsEvent]) -> Option<u8> {
    let mut p1 = 0u8;
    let mut p2 = 0u8;

    for event in events {
        if let EventKind::Note {
            side,
            lane: Lane::Key(key),
            ..
        } = &event.kind
        {
            let slot = if side.as_u8() == NoteSide::P1.as_u8() {
                &mut p1
            } else {
                &mut p2
            };
            *slot = (*slot).max(key.get());
        }
    }

    let total = p1.saturating_add(p2);
    (total > 0).then_some(total)
}

/// Return whether a visible note should be suppressed by an LN range.
fn is_note_suppressed(
    player: u8,
    lane: u8,
    tick: u64,
    ln_ranges: &HashMap<(u8, u8), Vec<(u64, u64)>>,
    unpaired_starts: &[(u8, u8, u64)],
) -> bool {
    if let Some(ranges) = ln_ranges.get(&(player, lane)) {
        for &(start, end) in ranges {
            if tick >= start && tick <= end {
                return true;
            }
        }
    }
    unpaired_starts.contains(&(player, lane, tick))
}

/// Build aligned bar events from the precomputed measure tick table.
///
/// Supports variable time signatures (`#xxx02`): each measure's barline sits at
/// its actual start tick instead of a forced equal-step 4/4 spacing.
fn build_bar_events(table: &MeasureTable) -> Vec<BmsEvent> {
    table
        .bar_ticks()
        .iter()
        .map(|&tick| Event::bar(tick))
        .collect()
}

/// Convert a mine's raw Base36 value into a damage value.
///
/// - `Some(1295)` (`"ZZ"`) means instant death (`f64::INFINITY`).
/// - `Some(n)` gives `n / 2.0`.
/// - `None` (decoding failed) gives the default `1.0`.
fn mine_damage(raw: Option<u16>) -> f64 {
    match raw {
        None => 1.0,
        Some(1295) => f64::INFINITY,
        Some(n) => f64::from(n) / 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_kill_value_produces_infinite_damage() {
        assert!(mine_damage(Some(1295)).is_infinite());
    }

    #[test]
    fn normal_value_produces_half_damage() {
        assert!((mine_damage(Some(1)) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn undecodable_value_produces_default_damage() {
        assert!((mine_damage(None) - 1.0).abs() < f64::EPSILON);
    }
}
