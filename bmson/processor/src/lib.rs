//! BMSON to `Chart` conversion processor.
//!
//! [`BmsonProcessor`] converts a `bmsrs_bmson_def::Bmson` (v2 root schema) into a
//! format-agnostic `Chart<BmsonNoteExt>` by applying a [`BmsonLayout`] mode
//! family ([`Beat`], [`Pms`], or [`GenericLayout`] for n-keys).
//!
//! Mode families and the [`BmsonLayout`] trait live in the [`layout`] module.
//!
//! # Pipeline
//!
//! ```text
//! bmsrs_bmson_def::Bmson -> BmsonProcessor::process(bmson, layout) -> Chart<BmsonNoteExt>
//! ```
//!
//! For v0/v1 files, convert to the root schema through `Bmson::from` first.
//!
//! # Slicing
//!
//! Each `bmsrs_bmson_def::SoundChannel` is sliced into precomputed `AudioAsset`s at
//! every unique note tick (see the internal `slice` module).

mod error;
mod slice;

pub mod layout;

use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bmsrs_bmson_def::{BpmEvent, StopEvent as BmsonStopEvent};
use bmsrs_chart::{
    AudioAsset, AudioIndex, BgaLayer, BgaResource, BgaResourceId, BpmChange, Chart, ChartData,
    ChartInfo, Damage, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint, NoteAudio,
    NoteExt, NoteKind, NoteRole, NoteSide, SongInfo, StopEvent, TimingTrack, TimingTrackError,
};
use thiserror::Error;

pub use crate::error::BmsonProcessWarning;

use crate::layout::{Beat, BmsonLayout, GenericLayout, Pms};

use crate::slice::slice_channel;

/// Per-note extension data for the BMSON format.
///
/// Carries optional fields from `bmsrs_bmson_def::NoteEvent` that are not part of the
/// core chart model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BmsonNoteExt {
    /// Note volume (percentage, DJ.NEXT extension).
    pub vol: Option<i8>,
    /// Note pan (DJ.NEXT extension).
    pub pan: Option<i8>,
    /// Per-note LN type hint; both beatoraja `t` and bmson v2 `ln_type_hint`
    /// merge here, with `t` taking priority.
    pub ln_type_hint: Option<LnTypeHint>,
    /// Per-note LN judgment hint override (bmson v2).
    pub ln_judge_hint: Option<LnJudgeHint>,
    /// Per-note LN life hint override (bmson v2).
    pub ln_life_hint: Option<LnLifeHint>,
}

impl NoteExt for BmsonNoteExt {}

/// Errors that can occur while processing BMSON.
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
    /// The `generic-nkeys` mode key count must be positive and within `u8`
    /// (the index bound of [`Lane::Key`]).
    #[error("generic-nkeys key count must be in 1..=255, got {0}")]
    InvalidKeyCount(u64),
    /// The chart references an unsafe resource path.
    #[error("unsafe resource path \"{path}\": {reason}")]
    UnsafePath {
        /// Original path.
        path: String,
        /// Rejection reason.
        reason: &'static str,
    },
}

/// Zero-sized processor that converts [`bmsrs_bmson_def::Bmson`] into
/// [`Chart<BmsonNoteExt>`].
///
/// Call [`process`](Self::process) with a mode-family layout, or
/// [`process_default`](Self::process_default) to choose automatically from
/// `mode_hint`.
pub struct BmsonProcessor;

/// BMSON processing result.
#[derive(Debug)]
pub struct Processed {
    /// Chart produced by processing.
    pub chart: Chart<BmsonNoteExt>,
    /// Recoverable warnings collected during processing.
    pub warnings: Vec<BmsonProcessWarning>,
}

impl BmsonProcessor {
    /// Process a BMSON chart with a stateless mode-family layout and return
    /// processing warnings.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Timing`] when timing track construction fails
    /// (invalid initial BPM or zero resolution).
    pub fn process<L>(bmson: &bmsrs_bmson_def::Bmson<'_>) -> Result<Processed, ProcessError>
    where
        L: BmsonLayout,
    {
        Self::process_body(bmson, &|x| L::map_x(x))
    }

    /// Process a BMSON chart with the generic-nkeys layout.
    ///
    /// This is the only stateful layout family; when the mode hint is
    /// `generic-nkeys`, call this method directly instead of
    /// [`process`](Self::process).
    ///
    /// # Errors
    ///
    /// Returns [`ProcessError::Timing`] when timing track construction fails
    /// (invalid initial BPM or zero resolution).
    pub fn process_nkeys(
        bmson: &bmsrs_bmson_def::Bmson<'_>,
        keys: u8,
    ) -> Result<Processed, ProcessError> {
        let layout = GenericLayout { keys };
        Self::process_body(bmson, &|x| layout.map_x(x))
    }

    /// Process a BMSON chart, choosing the mode family from `mode_hint`.
    ///
    /// `beat-*` and `dj-*` hints map to [`Beat`]; `popn-*` maps to [`Pms`];
    /// `generic-nkeys` maps to [`GenericLayout`] with the given key count;
    /// non-standard extension hints
    /// ([`ModeHint::Other`](bmsrs_bmson_def::ModeHint::Other)) fall back to [`Beat`]
    /// (the BMSON default), with a fallback warning recorded in
    /// [`Processed::warnings`].
    ///
    /// # Errors
    ///
    /// - Returns [`ProcessError::Timing`] when timing track construction fails
    ///   (invalid initial BPM or zero resolution).
    /// - Returns [`ProcessError::InvalidKeyCount`] when `mode_hint` is
    ///   `generic-nkeys` but the key count is 0 or above 255 (the index bound of
    ///   [`Lane::Key`]); otherwise every note would be dropped silently by the
    ///   empty key mapping.
    pub fn process_default(bmson: &bmsrs_bmson_def::Bmson<'_>) -> Result<Processed, ProcessError> {
        match &bmson.chart_data.mode_hint {
            bmsrs_bmson_def::ModeHint::Popn5k | bmsrs_bmson_def::ModeHint::Popn9k => {
                Self::process::<Pms>(bmson)
            }
            bmsrs_bmson_def::ModeHint::Generic(n) => {
                let keys = u8::try_from(*n).map_err(|_e| ProcessError::InvalidKeyCount(*n))?;
                if keys == 0 {
                    return Err(ProcessError::InvalidKeyCount(*n));
                }
                Self::process_nkeys(bmson, keys)
            }
            bmsrs_bmson_def::ModeHint::Other(hint) => {
                let mut processed = Self::process::<Beat>(bmson)?;
                processed
                    .warnings
                    .push(BmsonProcessWarning::UnknownModeHint { hint: hint.clone() });
                Ok(processed)
            }
            _ => Self::process::<Beat>(bmson),
        }
    }

    /// Public entry point for custom decode logic (internal use).
    fn process_body(
        bmson: &bmsrs_bmson_def::Bmson<'_>,
        decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    ) -> Result<Processed, ProcessError> {
        let bmson_data = &bmson.chart_data;

        let timing = build_timing(bmson_data)?;
        let bga_ids = build_bga_id_map(&bmson.chart_info.bga);
        let mut conv = BmsonConverter {
            bmson,
            decode,
            timing: timing.clone(),
            bga_ids: &bga_ids,
            audio_assets: Vec::new(),
            events: Vec::new(),
            warnings: Vec::new(),
        };

        conv.process_sound_channels()?;
        conv.process_mine_channels()?;
        conv.process_key_channels()?;
        conv.collect_bpm_events();
        conv.collect_stop_events();
        conv.collect_scroll_events();
        conv.collect_bga_events();
        conv.build_bar_lines();

        let song_info = build_song_info(bmson);
        let chart_info = build_chart_info(bmson)?;

        let mut data = ChartData {
            timing,
            judge_multiplier: Some(bmson_data.judge_multiplier),
            life_multiplier: Some(bmson_data.life_multiplier),
            ln_type_hint: ln_type_to_hint(bmson_data.ln_type_hint),
            ln_judge_hint: ln_judge_to_hint(bmson_data.ln_judge_hint),
            ln_life_hint: ln_life_to_hint(bmson_data.ln_life_hint),
            judge_deltas: bmson_data
                .judge_deltas
                .as_ref()
                .map(|d| bmsrs_chart::JudgementDeltas {
                    perfect: d.perfect,
                    great: d.great,
                    good: d.good,
                    miss: d.miss,
                }),
            life_deltas: bmson_data
                .life_deltas
                .as_ref()
                .map(|d| bmsrs_chart::LifeDeltas {
                    perfect: d.perfect,
                    great: d.great,
                    good: d.good,
                    miss: d.miss,
                }),
            events: conv.events,
            audio_assets: conv.audio_assets,
        };
        data.sort_events();

        Ok(Processed {
            chart: Chart {
                song: song_info,
                chart: chart_info,
                data,
            },
            warnings: conv.warnings,
        })
    }
}

/// BMSON to [`Chart`] conversion context.
///
/// Holds the source document [`bmsrs_bmson_def::Bmson`], the decode function, and the
/// precomputed timing cache, along with `audio_assets` and `events` accumulator
/// buffers, so conversion steps are organized as `&mut self` methods rather than
/// free functions with `&mut` parameters. Used only inside
/// [`BmsonProcessor::process_body`].
struct BmsonConverter<'a> {
    /// Source BMSON document.
    bmson: &'a bmsrs_bmson_def::Bmson<'a>,
    /// Channel decode function (`x` to `(side, lane)`).
    decode: &'a dyn Fn(u64) -> Option<(NoteSide, Lane)>,
    /// Precomputed timing cache.
    timing: TimingTrack,
    /// BMSON BGA raw-id to dense resource-index mapping.
    bga_ids: &'a BTreeMap<u64, BgaResourceId>,
    /// Audio asset vector being built.
    audio_assets: Vec<AudioAsset>,
    /// Event vector being built.
    events: Vec<Event<BmsonNoteExt>>,
    /// Recoverable warnings collected during processing.
    warnings: Vec<BmsonProcessWarning>,
}

/// A note waiting to be merged at the same `(side, lane, tick)`.
struct LayeredNote {
    /// Note position.
    lane: Lane,
    /// Note kind; layered long notes use the first length.
    kind: NoteKind,
    /// Release role; any layer with `up: true` makes it `Release`.
    role: NoteRole,
    /// Audio layers.
    audio: Vec<AudioIndex>,
    /// Format extension from the first layer.
    ext: BmsonNoteExt,
}

/// A single judgeable note waiting to be merged.
struct PendingNote {
    /// Player side.
    side: NoteSide,
    /// Lane.
    lane: Lane,
    /// Tick position.
    tick: u64,
    /// Note kind.
    kind: NoteKind,
    /// Play role.
    role: NoteRole,
    /// Audio index.
    audio: Option<AudioIndex>,
    /// Format extension.
    ext: BmsonNoteExt,
}

/// Encode `Lane` into a sortable key without adding `Ord` to the core type.
const fn lane_key(lane: Lane) -> (u8, u8) {
    match lane {
        Lane::Key(key) => (0, key.get()),
        Lane::Scratch(key) => (1, key.get()),
        Lane::FootPedal => (2, 0),
        _ => (3, 0),
    }
}

/// Merge one judgeable note into the layered table.
fn merge_layered_note(
    notes: &mut BTreeMap<(NoteSide, u8, u8, u64), LayeredNote>,
    warnings: &mut Vec<BmsonProcessWarning>,
    note: PendingNote,
) {
    let PendingNote {
        side,
        lane,
        tick,
        kind,
        role,
        audio,
        ext,
    } = note;
    let (lane_kind, lane_index) = lane_key(lane);
    let entry = notes
        .entry((side, lane_kind, lane_index, tick))
        .or_insert_with(|| LayeredNote {
            lane,
            kind,
            role,
            audio: Vec::new(),
            ext,
        });

    if let Some(index) = audio
        && !entry.audio.contains(&index)
    {
        entry.audio.push(index);
    }

    if role == NoteRole::Release {
        entry.role = NoteRole::Release;
    }

    match (entry.kind, kind) {
        (NoteKind::Long { duration: first }, NoteKind::Long { duration: other })
            if first != other =>
        {
            warnings.push(BmsonProcessWarning::LayeredNoteLengthMismatch { tick, first, other });
        }
        (NoteKind::Normal, NoteKind::Long { .. }) => {
            entry.kind = kind;
        }
        _ => {}
    }
}

impl BmsonConverter<'_> {
    /// Collect BPM change events. BPM changes with value 0 are skipped with a
    /// warning.
    fn collect_bpm_events(&mut self) {
        for e in &self.bmson.chart_data.bpm_events {
            if e.bpm == 0.0 {
                self.warnings
                    .push(BmsonProcessWarning::ZeroBpmChangeIgnored { tick: e.y });
            } else {
                self.events.push(Event::bpm(e.y, e.bpm));
            }
        }
    }

    /// Collect stop events.
    fn collect_stop_events(&mut self) {
        self.events.extend(
            self.bmson
                .chart_data
                .stop_events
                .iter()
                .map(|e| Event::stop(e.y, e.duration)),
        );
    }

    /// Collect SCROLL events.
    fn collect_scroll_events(&mut self) {
        self.events.extend(
            self.bmson
                .scroll_events
                .iter()
                .map(|e| Event::scroll(e.y, e.rate)),
        );
    }

    /// Collect BGA events (three layers: Base / Layer / Poor).
    ///
    /// `resource_id` is renumbered to a dense index through
    /// [`BmsonConverter::bga_ids`]; events referencing an id absent from
    /// `bga_header` are skipped (matching the BMS-side lookup-miss behavior).
    fn collect_bga_events(&mut self) {
        let bga = &self.bmson.chart_info.bga;
        for (layer, events) in [
            (BgaLayer::Base, &bga.bga_events),
            (BgaLayer::Layer, &bga.layer_events),
            (BgaLayer::Poor, &bga.poor_events),
        ] {
            for e in events {
                let Some(&resource_id) = self.bga_ids.get(&e.id) else {
                    continue;
                };
                self.events
                    .push(Event::new(e.y, EventKind::Bga { layer, resource_id }));
            }
        }
    }

    /// Build barline events from the barline definitions or the 4/4 default.
    fn build_bar_lines(&mut self) {
        let last_tick = self.events.iter().map(Event::tick).max().unwrap_or(0);
        self.events.extend(build_bar_lines(
            self.bmson.chart_data.lines.as_deref(),
            self.timing.resolution(),
            last_tick,
        ));
    }
    /// Process sound channels: slice and create note/BGM events.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio asset count fits in u32 for practical charts"
    )]
    fn process_sound_channels(&mut self) -> Result<(), ProcessError> {
        let mut layered: BTreeMap<(NoteSide, u8, u8, u64), LayeredNote> = BTreeMap::new();

        for channel in &self.bmson.chart_data.sound_channels {
            let path = normalize_path(channel.name)?;
            let sliced = slice_channel(channel, &self.timing, &path);

            let channel_playable: HashSet<u64> = channel
                .note_events
                .iter()
                .filter_map(|ne| if ne.is_bgm() { None } else { Some(ne.y) })
                .collect();

            for ne in &channel.note_events {
                let audio_idx = sliced
                    .pulse_to_index
                    .get(&ne.y)
                    .copied()
                    .map(|idx| AudioIndex::new((idx + self.audio_assets.len()) as u32));

                if ne.is_bgm() {
                    if channel_playable.contains(&ne.y) {
                        continue;
                    }
                    if let Some(idx) = audio_idx {
                        self.events.push(Event::bgm(ne.y, idx));
                    }
                } else {
                    let Some((side, lane)) = (self.decode)(ne.x) else {
                        continue;
                    };
                    let kind = if ne.l > 0 {
                        NoteKind::Long { duration: ne.l }
                    } else {
                        NoteKind::Normal
                    };
                    let role = if ne.up == Some(true) {
                        NoteRole::Release
                    } else {
                        NoteRole::Normal
                    };
                    merge_layered_note(
                        &mut layered,
                        &mut self.warnings,
                        PendingNote {
                            side,
                            lane,
                            tick: ne.y,
                            kind,
                            role,
                            audio: audio_idx,
                            ext: build_note_ext(ne),
                        },
                    );
                }
            }

            self.audio_assets.extend(sliced.assets);
        }

        for ((side, _, _, tick), note) in layered {
            self.events.push(Event::new(
                tick,
                EventKind::Note {
                    side,
                    lane: note.lane,
                    kind: note.kind,
                    role: note.role,
                    audio: NoteAudio::from_indices(note.audio),
                    ext: note.ext,
                },
            ));
        }

        Ok(())
    }

    /// Process mine channels: each channel contributes one whole-file
    /// `AudioAsset` and its mine notes.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio asset count fits in u32 for practical charts"
    )]
    fn process_mine_channels(&mut self) -> Result<(), ProcessError> {
        for mc in &self.bmson.mine_channels {
            let path = normalize_path(mc.name)?;
            let mine_audio_idx = AudioIndex::new(self.audio_assets.len() as u32);
            self.audio_assets.push(AudioAsset {
                path: Arc::from(path),
                start: Duration::ZERO,
                duration: None,
            });

            for mn in &mc.notes {
                let Some((side, lane)) = (self.decode)(mn.x) else {
                    continue;
                };
                self.events.push(Event::new(
                    mn.y,
                    EventKind::Note {
                        side,
                        lane,
                        kind: NoteKind::Mine {
                            damage: Damage::new(mn.damage),
                        },
                        role: NoteRole::Normal,
                        audio: NoteAudio::one(mine_audio_idx),
                        ext: BmsonNoteExt::default(),
                    },
                ));
            }
        }
        Ok(())
    }

    /// Process key (invisible) channels: same structure as mine channels.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio asset count fits in u32 for practical charts"
    )]
    fn process_key_channels(&mut self) -> Result<(), ProcessError> {
        for kc in &self.bmson.key_channels {
            let path = normalize_path(kc.name)?;
            let key_audio_idx = AudioIndex::new(self.audio_assets.len() as u32);
            self.audio_assets.push(AudioAsset {
                path: Arc::from(path),
                start: Duration::ZERO,
                duration: None,
            });

            for kn in &kc.notes {
                let Some((side, lane)) = (self.decode)(kn.x) else {
                    continue;
                };
                self.events.push(Event::new(
                    kn.y,
                    EventKind::Note {
                        side,
                        lane,
                        kind: NoteKind::Invisible,
                        role: NoteRole::Normal,
                        audio: NoteAudio::one(key_audio_idx),
                        ext: BmsonNoteExt::default(),
                    },
                ));
            }
        }
        Ok(())
    }
}

/// Normalize a BMSON resource path into a safe relative path.
///
/// Rejects absolute paths, Windows drive/UNC prefixes, `..` parent references,
/// NUL bytes, and empty paths; normalizes backslashes to forward slashes so
/// Windows-style paths also resolve on Unix.
fn normalize_path(path: &Path) -> Result<PathBuf, ProcessError> {
    let raw = path.to_string_lossy();
    if raw.contains('\0') {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "NUL byte",
        });
    }
    if raw.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && raw.as_bytes().get(1) == Some(&b':')
    {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "drive prefix",
        });
    }

    let normalized = raw.replace('\\', "/");
    let normalized_path = Path::new(&normalized);
    if normalized_path.is_absolute() {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "absolute path",
        });
    }

    let mut safe = PathBuf::new();
    for component in normalized_path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(ProcessError::UnsafePath {
                    path: raw.into_owned(),
                    reason: "parent directory traversal",
                });
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ProcessError::UnsafePath {
                    path: raw.into_owned(),
                    reason: "absolute path",
                });
            }
        }
    }

    if safe.as_os_str().is_empty() {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "empty path",
        });
    }

    Ok(safe)
}

/// Normalize an optional path and convert it to `String`.
fn normalize_optional_path(option: Option<&Path>) -> Result<Option<String>, ProcessError> {
    option
        .map(|raw| normalize_path(raw).map(|safe| safe.to_string_lossy().into_owned()))
        .transpose()
}

/// Build a [`TimingTrack`] from BMSON chart data.
///
/// # Errors
///
/// Returns [`TimingTrackError::InvalidBpm`] when `data.init_bpm` is invalid
/// (zero or non-finite).
fn build_timing(data: &bmsrs_bmson_def::ChartData<'_>) -> Result<TimingTrack, TimingTrackError> {
    TimingTrack::new(
        data.init_bpm,
        data.bpm_events.iter().map(build_bpm_change).collect(),
        data.stop_events.iter().map(build_stop_event).collect(),
        data.resolution,
    )
}

/// Build [`BmsonNoteExt`] from a BMSON [`NoteEvent`](bmsrs_bmson_def::NoteEvent).
fn build_note_ext(ne: &bmsrs_bmson_def::NoteEvent) -> BmsonNoteExt {
    BmsonNoteExt {
        vol: ne.vol,
        pan: ne.pan,
        ln_type_hint: ne
            .t
            .map(ln_mode_to_hint)
            .or_else(|| ne.ln_type_hint.map(ln_type_to_hint)),
        ln_judge_hint: ne.ln_judge_hint.map(ln_judge_to_hint),
        ln_life_hint: ne.ln_life_hint.map(ln_life_to_hint),
    }
}

/// Convert beatoraja `t` LN mode into a format-agnostic hint.
const fn ln_mode_to_hint(m: bmsrs_bmson_def::LnMode) -> LnTypeHint {
    match m {
        bmsrs_bmson_def::LnMode::Cn => LnTypeHint::Cn,
        bmsrs_bmson_def::LnMode::Hcn => LnTypeHint::Hcn,
        _ => LnTypeHint::Ln, // Ln plus #[non_exhaustive] fallback.
    }
}

/// Convert `bmsrs_bmson_def::LnType` into the format-agnostic [`LnTypeHint`].
const fn ln_type_to_hint(lt: bmsrs_bmson_def::LnType) -> LnTypeHint {
    match lt {
        bmsrs_bmson_def::LnType::Cn => LnTypeHint::Cn,
        bmsrs_bmson_def::LnType::Hcn => LnTypeHint::Hcn,
        _ => LnTypeHint::Ln, // Ln plus #[non_exhaustive] fallback.
    }
}

/// Convert `bmsrs_bmson_def::LnJudge` into the format-agnostic [`LnJudgeHint`].
const fn ln_judge_to_hint(lj: bmsrs_bmson_def::LnJudge) -> LnJudgeHint {
    match lj {
        bmsrs_bmson_def::LnJudge::Ticks => LnJudgeHint::Ticks,
        _ => LnJudgeHint::Normal,
    }
}

/// Convert `bmsrs_bmson_def::LnLife` into the format-agnostic [`LnLifeHint`].
const fn ln_life_to_hint(ll: bmsrs_bmson_def::LnLife) -> LnLifeHint {
    match ll {
        bmsrs_bmson_def::LnLife::Ticks => LnLifeHint::Ticks,
        _ => LnLifeHint::Normal,
    }
}

/// Build the mapping from BMSON BGA raw ids to dense indices.
///
/// This mapping guarantees the invariant that `resource_id` equals the index
/// into `bga_resources`: `bga_header` ids may be sparse or huge, and renumbering
/// lets renderers safely index the resource table. Duplicate raw ids use the
/// last occurrence (consistent with the schema note, since insert overwrites).
fn build_bga_id_map(bga: &bmsrs_bmson_def::BGA<'_>) -> BTreeMap<u64, BgaResourceId> {
    bga.bga_header
        .iter()
        .enumerate()
        .map(|(i, h)| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "BGA header count fits in u32"
            )]
            (h.id, BgaResourceId::new(i as u32))
        })
        .collect()
}

/// Build [`SongInfo`] from BMSON song information.
fn build_song_info(bmson: &bmsrs_bmson_def::Bmson<'_>) -> SongInfo {
    SongInfo {
        title: bmson.song_info.title.to_owned(),
        artist: bmson.song_info.artist.to_owned(),
        genre: bmson.song_info.genre.to_owned(),
    }
}

/// Build [`ChartInfo`] from BMSON chart information.
fn build_chart_info(bmson: &bmsrs_bmson_def::Bmson<'_>) -> Result<ChartInfo, ProcessError> {
    let bga = &bmson.chart_info.bga;
    let bga_resources: Vec<BgaResource> = bga
        .bga_header
        .iter()
        .map(|h| {
            Ok(BgaResource {
                path: normalize_path(h.name)?,
                // BMSON has no crop/placement concept (images and videos are
                // just paths).
                crop: None,
            })
        })
        .collect::<Result<_, ProcessError>>()?;

    Ok(ChartInfo {
        subtitle: bmson.chart_info.subtitle.to_owned(),
        chart_name: bmson.chart_info.chart_name.to_owned(),
        level: bmson.chart_info.level,
        subartists: bmson
            .chart_info
            .subartists
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        title_image: normalize_optional_path(bmson.chart_info.title_image)?,
        back_image: normalize_optional_path(bmson.chart_info.back_image)?,
        eyecatch_image: normalize_optional_path(bmson.chart_info.eyecatch_image)?,
        banner_image: normalize_optional_path(bmson.chart_info.banner_image)?,
        preview_music: normalize_optional_path(bmson.chart_info.preview_music)?,
        bga_resources,
        // BMSON has no #VIDEOFILE/#MOVIE concept.
        video: None,
        key_count: mode_key_count(&bmson.chart_data.mode_hint),
    })
}

/// Infer the total playable key count from `mode_hint`.
fn mode_key_count(mode: &bmsrs_bmson_def::ModeHint) -> Option<u8> {
    use bmsrs_bmson_def::ModeHint;

    match mode {
        ModeHint::Beat5k
        | ModeHint::Dj5kOnly
        | ModeHint::DjRuby
        | ModeHint::Dj5k
        | ModeHint::Popn5k => Some(5),
        ModeHint::Beat7k | ModeHint::Dj7k => Some(7),
        ModeHint::Beat10k | ModeHint::Dj10k => Some(10),
        ModeHint::Beat14k | ModeHint::Dj14k | ModeHint::DjAndromeda => Some(14),
        ModeHint::Popn9k => Some(9),
        ModeHint::Generic(count) => u8::try_from(*count).ok(),
        _ => None,
    }
}

/// Build [`BpmChange`] from a BMSON [`BpmEvent`].
const fn build_bpm_change(e: &BpmEvent) -> BpmChange {
    BpmChange {
        tick: e.y,
        bpm: e.bpm,
    }
}

/// Build [`StopEvent`] from a BMSON stop event.
const fn build_stop_event(e: &BmsonStopEvent) -> StopEvent {
    StopEvent {
        tick: e.y,
        duration: e.duration,
    }
}

/// Build barline events from the BMSON `lines` field.
///
/// `None` auto-generates 4/4 barlines (one every `resolution * 4` ticks) from 0
/// to `last_tick`. The caller derives `last_tick` from the complete event set
/// (including BGA) so every event type is covered.
fn build_bar_lines(
    lines: Option<&[bmsrs_bmson_def::BarLine]>,
    resolution: u64,
    last_tick: u64,
) -> Vec<Event<BmsonNoteExt>> {
    if let Some(vec) = lines {
        return vec.iter().map(|bl| Event::bar(bl.y)).collect();
    }

    let step = resolution * 4;
    let count = last_tick / step + 1;
    (0..=count).map(|i| Event::bar(i * step)).collect()
}
