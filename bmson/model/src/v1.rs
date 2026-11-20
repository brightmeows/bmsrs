//! **bmson v1.0.0**: flat schema.
//!
//! This module corresponds to the v1.0.0 specification, where `Bmson` is a flat
//! object containing both metadata and chart data.
//!
//! # Conversion
//!
//! [`Bmson`] and [`crate::Bmson`] convert through [`From`].
//!
//! v1 to root (v2) splits [`BmsonInfo`] into [`crate::SongInfo`],
//! [`crate::ChartInfo`], and [`crate::ChartData`]. Root to v1 merges them back.
//!
//! # Serde
//!
//! Top-level types use v1-specific field names (`info`, `sound_channels`, and
//! `notes` as root keys). Leaf types are reused from the root module.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    BGA, BarLine, BpmEvent, KeyChannel, LnMode, MineChannel, ModeHint, ScrollEvent, StopEvent,
};

/// Top-level bmson object in the v1.0.0 schema.
///
/// ```json
/// {
///   "version": "1.0.0",
///   "info": { "title": "...", ... },
///   "lines": [ ... ],
///   "bpm_events": [ ... ],
///   "stop_events": [ ... ],
///   "sound_channels": [ ... ],
///   "bga": { ... }
/// }
/// ```
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bmson {
    /// bmson format version (should be `"1.0.0"`).
    pub version: String,

    /// Metadata object.
    pub info: BmsonInfo,

    /// Barline positions. `None` auto-generates 4/4; `Some([])` means no
    /// barlines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<BarLine>>,

    /// BPM change events (the specification allows null; null is treated as
    /// empty).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub bpm_events: Vec<BpmEvent>,

    /// Stop (pause) events (the specification allows null; null is treated as
    /// empty).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub stop_events: Vec<StopEvent>,

    /// Sound channels (v1 uses `notes` inside each channel).
    #[serde(default)]
    pub sound_channels: Vec<SoundChannel>,

    /// Background animation data.
    pub bga: BGA,

    /// Scroll speed events.
    #[serde(default)]
    pub scroll_events: Vec<ScrollEvent>,
    /// Mine channels.
    #[serde(default)]
    pub mine_channels: Vec<MineChannel>,
    /// Invisible key channels.
    #[serde(default)]
    pub key_channels: Vec<KeyChannel>,

    // KaleidBeat extensions (not part of the bmson schema or beatoraja).
    /// `KaleidBeat` extension: spacing-factor keyframes.
    #[serde(default)]
    pub speed_events: Vec<crate::SpeedEvent>,
    /// `KaleidBeat` extension: timed text events.
    #[serde(default)]
    pub text_events: Vec<crate::TextEvent>,
    /// `KaleidBeat` extension: judge-rank keyframes.
    #[serde(default)]
    pub judge_rank_events: Vec<crate::JudgeRankEvent>,
    /// `KaleidBeat` extension: free-zone sound channels.
    #[serde(default)]
    pub free_zone_channels: Vec<crate::FreeZoneChannel>,
    /// `KaleidBeat` extension: potion notes.
    #[serde(default)]
    pub potion_notes: Vec<crate::PotionNote>,
    /// `KaleidBeat` extension: display-only dummy notes.
    #[serde(default)]
    pub dummy_notes: Vec<crate::DummyNote>,
    /// `KaleidBeat` extension: display-only dummy mines.
    #[serde(default)]
    pub dummy_mine_notes: Vec<crate::DummyMineNote>,
    /// `KaleidBeat` extension: display-only dummy potions.
    #[serde(default)]
    pub dummy_potion_notes: Vec<crate::DummyPotionNote>,
    /// `KaleidBeat` extension: runtime conditional blocks.
    #[serde(default)]
    pub conditional_blocks: Vec<ConditionalBlock>,
}

/// Metadata object in the v1 schema.
///
/// Contains all information about the song and this specific chart.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BmsonInfo {
    /// Song title.
    pub title: String,

    /// Subtitle (default `""`).
    #[serde(default)]
    pub subtitle: String,

    /// Main artist.
    pub artist: String,

    /// Other contributors (`["key:value", ...]`).
    #[serde(default, deserialize_with = "crate::null_to_default")]
    pub subartists: Vec<String>,

    /// Genre.
    pub genre: String,

    /// Game mode hint (default `"beat-7k"`).
    #[serde(default)]
    pub mode_hint: ModeHint,

    /// Chart name / difficulty label (default `""`).
    #[serde(default)]
    pub chart_name: String,

    /// Numeric difficulty level.
    pub level: u64,

    /// Initial BPM.
    pub init_bpm: f64,

    /// Judgment window (0-100 scale, default 100).
    #[serde(default = "default_100")]
    pub judge_rank: f64,

    /// Life gauge charge amount (0-100 scale, default 100).
    #[serde(default = "default_100")]
    pub total: f64,

    /// Background image (during gameplay).
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_image: Option<PathBuf>,

    /// Eyecatch image (loading screen).
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub eyecatch_image: Option<PathBuf>,

    /// Banner image (song selection and result screens).
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub banner_image: Option<PathBuf>,

    /// Preview audio path.
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub preview_music: Option<PathBuf>,

    /// Title image shown before gameplay starts. Equivalent to `#BACKBMP` in the
    /// OADX+ skin system.
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub title_image: Option<PathBuf>,

    /// Beat resolution (default 240).
    #[serde(
        default = "crate::default_resolution",
        deserialize_with = "crate::deserialize_resolution"
    )]
    pub resolution: u64,

    /// Long-note type, a beatoraja extension (JSON key `lnType`).
    #[serde(alias = "lnType", default, skip_serializing_if = "Option::is_none")]
    pub ln_type: Option<LnMode>,

    // KaleidBeat extensions (not part of the bmson schema or beatoraja).
    /// `KaleidBeat` extension: explicit difficulty value or name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<crate::Difficulty>,
    /// `KaleidBeat` extension: song-select comment.
    #[serde(default)]
    pub comment: String,
    /// `KaleidBeat` extension: override for the displayed BPM string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_bpm: Option<String>,
    /// `KaleidBeat` extension: MIDI accompaniment file.
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub midi_file: Option<PathBuf>,
    /// `KaleidBeat` extension: pomu2 character definition file.
    #[serde(
        default,
        deserialize_with = "crate::de_opt_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub chara_file: Option<PathBuf>,
}

/// Default value 100.0 used by `#[serde(default)]` on fields such as `judge_rank`.
const fn default_100() -> f64 {
    100.0
}

/// Sound channel in the v1 schema.
///
/// Same as [`crate::SoundChannel`], except the note field is `notes` (v1
/// convention) rather than `note_events` (v2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundChannel {
    /// Audio file name.
    #[serde(deserialize_with = "crate::de_path")]
    pub name: PathBuf,
    /// Notes referencing this audio file.
    #[serde(rename = "notes")]
    pub notes: Vec<crate::NoteEvent>,
}

impl SoundChannel {
    /// Parse the channel name as a `KaleidBeat` filter object, if present.
    ///
    /// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
    #[must_use]
    pub fn filter(&self) -> Option<crate::SoundFilter> {
        self.name.to_str().and_then(crate::SoundFilter::parse)
    }
}

/// A conditional block in the v1 schema.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditionalBlock {
    /// Evaluation pulse.
    pub y: u64,
    /// Branches in source order.
    #[serde(default)]
    pub branches: Vec<ConditionalBranch>,
}

/// One branch of a v1 [`ConditionalBlock`].
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConditionalBranch {
    /// Condition expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Whether this is the fallback branch.
    #[serde(default)]
    pub default: bool,
    /// Sound channels active in this branch.
    #[serde(default)]
    pub sound_channels: Vec<SoundChannel>,
    /// Invisible key channels active in this branch.
    #[serde(default)]
    pub key_channels: Vec<KeyChannel>,
    /// Mine channels active in this branch.
    #[serde(default)]
    pub mine_channels: Vec<MineChannel>,
    /// Free-zone channels active in this branch.
    #[serde(default)]
    pub free_zone_channels: Vec<crate::FreeZoneChannel>,
    /// Potion notes active in this branch.
    #[serde(default)]
    pub potion_notes: Vec<crate::PotionNote>,
    /// Dummy notes active in this branch.
    #[serde(default)]
    pub dummy_notes: Vec<crate::DummyNote>,
    /// Dummy mines active in this branch.
    #[serde(default)]
    pub dummy_mine_notes: Vec<crate::DummyMineNote>,
    /// Dummy potions active in this branch.
    #[serde(default)]
    pub dummy_potion_notes: Vec<crate::DummyPotionNote>,
    /// BPM changes active in this branch.
    #[serde(default)]
    pub bpm_events: Vec<crate::BpmEvent>,
    /// Stop events active in this branch.
    #[serde(default)]
    pub stop_events: Vec<crate::StopEvent>,
    /// Scroll events active in this branch.
    #[serde(default)]
    pub scroll_events: Vec<crate::ScrollEvent>,
    /// Speed keyframes active in this branch.
    #[serde(default)]
    pub speed_events: Vec<crate::SpeedEvent>,
    /// Bar lines active in this branch.
    #[serde(default)]
    pub lines: Vec<crate::BarLine>,
    /// BGA data active in this branch.
    #[serde(default)]
    pub bga: crate::BGA,
}

use crate::{Bmson as RootBmson, ChartData, ChartInfo, SongInfo};

impl From<Bmson> for RootBmson {
    fn from(v1: Bmson) -> Self {
        let info = v1.info;

        let song_info = SongInfo {
            title: info.title,
            artist: info.artist,
            genre: info.genre,
        };

        let chart_info = ChartInfo {
            subtitle: info.subtitle,
            subartists: info.subartists,
            chart_name: info.chart_name,
            level: info.level,
            back_image: info.back_image,
            eyecatch_image: info.eyecatch_image,
            banner_image: info.banner_image,
            preview_music: info.preview_music,
            title_image: info.title_image,
            bga: v1.bga,
            difficulty: info.difficulty,
            comment: info.comment,
            display_bpm: info.display_bpm,
            midi_file: info.midi_file,
            chara_file: info.chara_file,
        };

        let chart_data = ChartData {
            mode_hint: info.mode_hint,
            ln_type_hint: info.ln_type.map(crate::ln_mode_to_type_hint),
            ln_judge_hint: crate::LnJudge::Normal,
            ln_life_hint: crate::LnLife::Normal,
            init_bpm: info.init_bpm,
            judge_multiplier: info.judge_rank / 100.0,
            life_multiplier: info.total / 100.0,
            resolution: info.resolution,
            lines: v1.lines,
            bpm_events: v1.bpm_events,
            stop_events: v1.stop_events,
            sound_channels: v1
                .sound_channels
                .into_iter()
                .map(|ch| crate::SoundChannel {
                    name: ch.name,
                    note_events: ch
                        .notes
                        .into_iter()
                        .map(|mut note| {
                            crate::common::backfill_ln_type_hint(&mut note);
                            note
                        })
                        .collect(),
                })
                .collect(),
            judge_deltas: None,
            life_deltas: None,
        };

        Self {
            version: v1.version,
            song_info,
            chart_info,
            chart_data,
            scroll_events: v1.scroll_events,
            mine_channels: v1.mine_channels,
            key_channels: v1.key_channels,
            speed_events: v1.speed_events,
            text_events: v1.text_events,
            judge_rank_events: v1.judge_rank_events,
            free_zone_channels: v1.free_zone_channels,
            potion_notes: v1.potion_notes,
            dummy_notes: v1.dummy_notes,
            dummy_mine_notes: v1.dummy_mine_notes,
            dummy_potion_notes: v1.dummy_potion_notes,
            conditional_blocks: v1
                .conditional_blocks
                .into_iter()
                .map(v1_block_to_root)
                .collect(),
        }
    }
}

impl From<RootBmson> for Bmson {
    fn from(root: RootBmson) -> Self {
        let info = BmsonInfo {
            title: root.song_info.title,
            subtitle: root.chart_info.subtitle,
            artist: root.song_info.artist,
            subartists: root.chart_info.subartists,
            genre: root.song_info.genre,
            mode_hint: root.chart_data.mode_hint,
            chart_name: root.chart_info.chart_name,
            level: root.chart_info.level,
            init_bpm: root.chart_data.init_bpm,
            judge_rank: root.chart_data.judge_multiplier * 100.0,
            total: root.chart_data.life_multiplier * 100.0,
            back_image: root.chart_info.back_image,
            eyecatch_image: root.chart_info.eyecatch_image,
            banner_image: root.chart_info.banner_image,
            preview_music: root.chart_info.preview_music,
            title_image: root.chart_info.title_image,
            resolution: root.chart_data.resolution,
            ln_type: root.chart_data.ln_type_hint.map(|hint| match hint {
                crate::LnType::Cn => crate::LnMode::Cn,
                crate::LnType::Hcn => crate::LnMode::Hcn,
                crate::LnType::Hln => crate::LnMode::Hln,
                // An explicit `ln` stays explicit so the undeclared state
                // survives v2 -> v1 -> v2 roundtrips.
                crate::LnType::Ln => crate::LnMode::Ln,
            }),
            difficulty: root.chart_info.difficulty,
            comment: root.chart_info.comment,
            display_bpm: root.chart_info.display_bpm,
            midi_file: root.chart_info.midi_file,
            chara_file: root.chart_info.chara_file,
        };

        Self {
            version: root.version,
            info,
            lines: root.chart_data.lines,
            bpm_events: root.chart_data.bpm_events,
            stop_events: root.chart_data.stop_events,
            sound_channels: root
                .chart_data
                .sound_channels
                .into_iter()
                .map(|ch| SoundChannel {
                    name: ch.name,
                    notes: ch.note_events,
                })
                .collect(),
            bga: root.chart_info.bga,
            scroll_events: root.scroll_events,
            mine_channels: root.mine_channels,
            key_channels: root.key_channels,
            speed_events: root.speed_events,
            text_events: root.text_events,
            judge_rank_events: root.judge_rank_events,
            free_zone_channels: root.free_zone_channels,
            potion_notes: root.potion_notes,
            dummy_notes: root.dummy_notes,
            dummy_mine_notes: root.dummy_mine_notes,
            dummy_potion_notes: root.dummy_potion_notes,
            conditional_blocks: root
                .conditional_blocks
                .into_iter()
                .map(root_block_to_v1)
                .collect(),
        }
    }
}

/// Convert a v1 conditional block to the v2 root shape.
fn v1_block_to_root(block: ConditionalBlock) -> crate::ConditionalBlock {
    crate::ConditionalBlock {
        y: block.y,
        branches: block
            .branches
            .into_iter()
            .map(|branch| crate::ConditionalBranch {
                when: branch.when,
                default: branch.default,
                sound_channels: branch
                    .sound_channels
                    .into_iter()
                    .map(|channel| crate::SoundChannel {
                        name: channel.name,
                        note_events: channel.notes,
                    })
                    .collect(),
                key_channels: branch.key_channels,
                mine_channels: branch.mine_channels,
                free_zone_channels: branch.free_zone_channels,
                potion_notes: branch.potion_notes,
                dummy_notes: branch.dummy_notes,
                dummy_mine_notes: branch.dummy_mine_notes,
                dummy_potion_notes: branch.dummy_potion_notes,
                bpm_events: branch.bpm_events,
                stop_events: branch.stop_events,
                scroll_events: branch.scroll_events,
                speed_events: branch.speed_events,
                lines: branch.lines,
                bga: branch.bga,
            })
            .collect(),
    }
}

/// Convert a v2 root conditional block to the v1 shape.
fn root_block_to_v1(block: crate::ConditionalBlock) -> ConditionalBlock {
    ConditionalBlock {
        y: block.y,
        branches: block
            .branches
            .into_iter()
            .map(|branch| ConditionalBranch {
                when: branch.when,
                default: branch.default,
                sound_channels: branch
                    .sound_channels
                    .into_iter()
                    .map(|channel| SoundChannel {
                        name: channel.name,
                        notes: channel.note_events,
                    })
                    .collect(),
                key_channels: branch.key_channels,
                mine_channels: branch.mine_channels,
                free_zone_channels: branch.free_zone_channels,
                potion_notes: branch.potion_notes,
                dummy_notes: branch.dummy_notes,
                dummy_mine_notes: branch.dummy_mine_notes,
                dummy_potion_notes: branch.dummy_potion_notes,
                bpm_events: branch.bpm_events,
                stop_events: branch.stop_events,
                scroll_events: branch.scroll_events,
                speed_events: branch.speed_events,
                lines: branch.lines,
                bga: branch.bga,
            })
            .collect(),
    }
}
