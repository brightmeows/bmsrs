//! BMS to `Chart` conversion processor.
//!
//! [`BmsProcessor`] converts `bmsrs_bms_model::Bms` into a format-agnostic `Chart`
//! by applying a [`BmsLayout`] mode family (such as [`Bme`], `Pms`, or
//! `Nanasi`).
//!
//! Mode families and the [`BmsLayout`] trait live in this crate's `layout`
//! module.
//!
//! # Pipeline
//!
//! ```text
//! bmsrs_bms_model::Bms -> BmsProcessor::process::<L>(bms) -> Chart<(), NoCustomEvent>
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

mod audio;
mod bga;
pub mod custom_event;
mod error;
mod long_note;
mod notes;
mod position;
mod timing;

pub mod layout;

use std::collections::BTreeMap;
use std::sync::Arc;

use bmsrs_bms_model::{Bms, Gameplay};
use bmsrs_bms_values::{
    BmpIndex, BmsChannel as RawChannel, ChangeOptionIndex, ExRankIndex, PlayerMode, Rank,
    SeekIndex, TextIndex,
};
use bmsrs_chart::{
    BgaResource, BpmLookup, Chart, ChartData, ChartDataError, ChartInfo, Event, EventKind,
    JudgeRankKind, JudgeRankSpec, Lane, Note, NoteSide, SongInfo, SwBgaResource, SwBgaResourceId,
    TimingTrack, TimingTrackError,
};
use bmsrs_pipeline::Outcome;
use thiserror::Error;

pub use crate::audio::BmsAudio;
pub use crate::error::{ProcessWarning, ProcessWarningKind};

use crate::bga::BmpEntry;
use crate::custom_event::BmsCustomEvent;
use crate::layout::{Bme, BmsLayout, DscOctFp, Pms, PmsBme, PmsLayout};
use crate::position::MeasureTable;
use crate::timing::build_bar_events;

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
    /// Chart data construction or validation failed.
    #[error(transparent)]
    ChartData(#[from] ChartDataError),
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
            ex_rank_defs: gameplay.ex_rank_defs.clone(),
            option: gameplay.option.clone(),
            change_option_defs: gameplay.change_option_defs.clone(),
        }
    }
}

/// Default beat resolution for BMS charts (ticks per quarter note).
const RESOLUTION: u64 = 240;

impl BmsProcessor {
    /// Process a BMS chart with an explicit mode-family layout.
    ///
    /// The layout type determines how each BMS `(player, lane)` channel byte
    /// decodes to a note position. Available families are listed in the
    /// `layout` module.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Timing`] when timing track construction fails
    /// (initial BPM missing, zero, or non-finite).
    pub fn process<L>(bms: &Bms) -> Result<Outcome<Processed, ProcessWarning>, ProcessError>
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

        let (wav_map, audio_assets) = BmsConverter::build_audio_assets(&bms.audio);
        let bpm_changes = conv.build_bpm_changes();
        let bpm_lookup = BpmLookup::new(init_bpm, &bpm_changes);

        let mut stops = conv.build_stops_from_defs(&bpm_lookup);
        stops.extend(conv.build_stops_from_stp());
        stops.sort_by_key(|s| s.tick);

        let timing = TimingTrack::new(init_bpm, bpm_changes, stops, RESOLUTION)?;

        let bmp_map = conv.build_bmp_map();

        let (ln_result, consumed) = conv.pair_long_notes();
        let paired_lns = ln_result.paired;
        let unpaired_starts = ln_result.unpaired_starts;

        // Collect unpaired long-note warnings.
        for &(player, lane, tick) in &unpaired_starts {
            conv.warnings.push(ProcessWarning::new(
                None,
                ProcessWarningKind::UnterminatedLongNote { player, lane, tick },
            ));
        }

        // Barlines first (priority 0).
        conv.events.extend(build_bar_events(&table));

        // Notes (priority 1).
        conv.collect_notes::<L>(&wav_map, &paired_lns, &consumed, &unpaired_starts);

        // BGM (priority 1).
        conv.collect_bgm(&wav_map);

        // BGA (priority 1).
        conv.collect_bga(&bmp_map);

        // BPM changes (priority 2).
        conv.collect_bpm_events(timing.bpm_changes());

        // Judgment rank changes (priority 2, alongside BPM).
        conv.collect_judge_rank();

        // Stop events (priority 3): iterate the timing stop events again.
        conv.collect_stop_events(timing.stops());

        // SCROLL events (priority 4).
        conv.collect_scroll_events();

        // SPEED events (priority 5).
        conv.collect_speed_events();

        // BMS engine-specific custom events (priority 6).
        let swbga_map = conv.build_swbga_map();

        conv.collect_custom_events(&swbga_map);

        let swbga_resources = conv.build_swbga_resources(&bmp_map);
        let (song, chart_info) = conv.build_metadata(&bmp_map, swbga_resources);
        let ln_type_hint = conv.ln_type_hint();

        let data = ChartData::new(init_bpm, RESOLUTION, conv.events, audio_assets)?
            .with_ln_type_hint(ln_type_hint);

        // Warning aggregation: parser-layer value parse warnings come first (by
        // file position), followed by processor-layer missing-reference and
        // clamping warnings. Taken after the build_metadata borrow ends.
        let mut warnings: Vec<ProcessWarning> = bms
            .messages
            .warnings
            .iter()
            .cloned()
            .map(ProcessWarning::from)
            .collect();
        warnings.extend(conv.warnings);
        let gameplay = BmsGameplay::from(&bms.gameplay);
        let audio = BmsAudio::from(&bms.audio);

        Ok(Outcome::new(
            Processed {
                chart: Chart::new(song, chart_info, data)?,
                gameplay,
                audio,
            },
            warnings,
        ))
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
    pub fn process_pms(bms: &Bms) -> Result<Outcome<Processed, ProcessWarning>, ProcessError> {
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
    pub fn process_default(bms: &Bms) -> Result<Outcome<Processed, ProcessWarning>, ProcessError> {
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
///
/// The conversion methods live in domain modules: [`timing`] (BPM, STOP,
/// SCROLL, SPEED, bar lines), [`notes`] (playable notes, BGM, long notes),
/// [`bga`] (BMP/BGA/SWBGA/video), and [`audio`] (the WAV asset table) in this
/// file's sibling modules; cross-domain steps (`collect_custom_events`,
/// `collect_judge_rank`, `build_metadata`) stay here.
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
    /// Build judgment rank events from channel `A0` references into
    /// `#EXRANKxx`.
    ///
    /// The definition value is the judgment width percentage (`100` is the
    /// standard width; fractional values are allowed by the specification).
    /// References to undefined definitions warn and are skipped.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "channel values are single-byte bounded (0-255) or u32 resource indices"
    )]
    fn collect_judge_rank(&mut self) {
        let base = self.bms.gameplay.base.unwrap_or_default();
        let non_event = &self.bms.messages.non_event_data;
        for item in non_event
            .iter()
            .filter(|item| item.channel == RawChannel::Judge)
        {
            let total = item.values.len() as u32;
            for (i, obj) in item.values.iter().enumerate() {
                if obj.as_str() == "00" {
                    continue;
                }
                let tick = self.table.position_to_tick(bmsrs_bms_model::Position::new(
                    item.measure,
                    i as u32,
                    total,
                ));
                let Some(percent) = self
                    .bms
                    .gameplay
                    .ex_rank_defs
                    .get(&ExRankIndex::from(*obj))
                    .copied()
                else {
                    let span = self.message_span(item.measure, &[RawChannel::Judge]);
                    if let Some(key) = obj.to_index_in(base) {
                        self.warnings.push(ProcessWarning::new(
                            span,
                            ProcessWarningKind::MissingExRankDefinition { key, tick },
                        ));
                    }
                    continue;
                };
                self.events
                    .push(Event::new(tick, EventKind::JudgeRank { percent }));
            }
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
    fn collect_custom_events(&mut self, swbga_map: &BTreeMap<BmpIndex, SwBgaResourceId>) {
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
                let tick = self.table.position_to_tick(bmsrs_bms_model::Position::new(
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
                        // The value is normalized; look it up in the SWBGA
                        // definition table directly.
                        let Some(swbga_id) = swbga_map.get(&BmpIndex::from(*obj)).copied() else {
                            let span = self.message_span(item.measure, &[RawChannel::BgaKeyBound]);
                            self.warnings.push(ProcessWarning::new(
                                span,
                                ProcessWarningKind::MissingSwBgaDefinition {
                                    key: obj.to_index().unwrap_or(0),
                                    tick,
                                },
                            ));
                            continue;
                        };
                        BmsCustomEvent::BgaKeyBound { swbga_id }
                    }
                    RawChannel::Text => {
                        // The value is normalized; look it up in text_defs
                        // directly. The text body travels with the event; an
                        // empty definition clears the display.
                        let Some(text) = self
                            .bms
                            .metadata
                            .text_defs
                            .get(&TextIndex::from(*obj))
                            .cloned()
                        else {
                            let span = self.message_span(item.measure, &[RawChannel::Text]);
                            self.warnings.push(ProcessWarning::new(
                                span,
                                ProcessWarningKind::MissingTextDefinition {
                                    key: obj.to_index().unwrap_or(0),
                                    tick,
                                },
                            ));
                            continue;
                        };
                        BmsCustomEvent::TextDisplay {
                            text: Arc::from(text.as_str()),
                        }
                    }
                    // Judgment rank changes are collected separately at
                    // priority 2.
                    RawChannel::Judge => {
                        continue;
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

    /// Build [`SongInfo`] and [`ChartInfo`] from BMS metadata and the BMP
    /// mapping.
    #[expect(clippy::cast_possible_truncation, reason = "play level fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "play level is non-negative")]
    fn build_metadata(
        &self,
        bmp_map: &BTreeMap<BmpIndex, BmpEntry>,
        swbga_resources: Vec<SwBgaResource>,
    ) -> (SongInfo, ChartInfo) {
        let bga_resources: Vec<BgaResource> = bmp_map
            .values()
            .map(|e| BgaResource {
                path: e.path.clone().into(),
                crop: e.crop,
            })
            .collect();
        let video = self.build_video_asset();
        // `#TEXT00` is the nanasi miss-time text; it belongs to the miss
        // system, not to channel 99 data.
        let miss_text = self
            .bms
            .metadata
            .text_defs
            .iter()
            .find(|(id, _)| id.to_index_in(self.bms.gameplay.base.unwrap_or_default()) == Some(0))
            .map(|(_, value)| value.clone());
        // Judgment difficulty: #DEFEXRANK (percentage) refines #RANK (engine
        // specific index), so it wins when both are present.
        let judge_rank = match (self.bms.gameplay.def_ex_rank, self.bms.gameplay.rank) {
            (Some(value), _) => Some(JudgeRankSpec {
                value,
                kind: JudgeRankKind::DefExRank,
            }),
            (None, Some(rank)) => Some(JudgeRankSpec {
                value: rank_value(rank),
                kind: JudgeRankKind::BmsRank,
            }),
            (None, None) => None,
        };
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
                miss_text,
                bga_resources,
                swbga_resources,
                video,
                key_count: key_count(&self.events),
                judge_rank,
                total: self.bms.gameplay.total,
                volwav_percent: self.bms.gameplay.vol_wav.map(|v| v.round() as u8),
            },
        )
    }

    /// Return the source line span for a measure and candidate channels.
    fn message_span(
        &self,
        measure: u16,
        channels: &[RawChannel],
    ) -> Option<bmsrs_pipeline::SourceSpan> {
        channels
            .iter()
            .find_map(|channel| self.bms.messages.line_span(measure, *channel))
    }
}

/// Return the numeric `#RANK` index of a [`Rank`] variant.
const fn rank_value(rank: Rank) -> f64 {
    match rank {
        Rank::VeryHard => 0.0,
        Rank::Hard => 1.0,
        Rank::Normal => 2.0,
        Rank::Easy => 3.0,
        Rank::VeryEasy => 4.0,
        Rank::Other(n) => n as f64,
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

/// Infer the total number of playable keys from the event stream.
///
/// Takes the maximum `Lane::Key` number per side and adds the two sides; returns
/// `None` when there are no key notes. Scratches, foot pedals, and mines do not
/// count toward the key count.
fn key_count(events: &[BmsEvent]) -> Option<u8> {
    let mut p1 = 0u8;
    let mut p2 = 0u8;

    for event in events {
        if let EventKind::Note(Note {
            side,
            lane: Lane::Key(key),
            ..
        }) = &event.kind
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
