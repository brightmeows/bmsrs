//! `KaleidBeat` sidecar extraction for bmson.

use std::path::PathBuf;

use bmsrs_bmson_model::{
    Bmson, ConditionalBlock, Difficulty, DummyMineNote, DummyNote, DummyPotionNote,
    FreeZoneChannel, JudgeRankEvent, PotionNote, SoundFilter, SpeedEvent, TextEvent,
};

/// `KaleidBeat` extensions carried by a bmson file.
///
/// `KaleidBeat` extension: none of these fields are part of the bmson schema or
/// beatoraja. Ordinary charts leave [`BmsonProcessed::kaleid`](crate::BmsonProcessed::kaleid)
/// as `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsonKaleid {
    /// Explicit difficulty value or name.
    pub difficulty: Option<Difficulty>,
    /// Song-select comment.
    pub comment: String,
    /// Displayed BPM override.
    pub display_bpm: Option<String>,
    /// MIDI accompaniment file.
    pub midi_file: Option<PathBuf>,
    /// pomu2 character definition file.
    pub chara_file: Option<PathBuf>,
    /// Spacing-factor keyframes.
    pub speed_events: Vec<SpeedEvent>,
    /// Timed text events.
    pub text_events: Vec<TextEvent>,
    /// Judge-rank keyframes.
    pub judge_rank_events: Vec<JudgeRankEvent>,
    /// Free-zone sound channels.
    pub free_zone_channels: Vec<FreeZoneChannel>,
    /// Potion notes.
    pub potion_notes: Vec<PotionNote>,
    /// Display-only dummy notes.
    pub dummy_notes: Vec<DummyNote>,
    /// Display-only dummy mines.
    pub dummy_mine_notes: Vec<DummyMineNote>,
    /// Display-only dummy potions.
    pub dummy_potion_notes: Vec<DummyPotionNote>,
    /// Runtime conditional blocks.
    pub conditional_blocks: Vec<ConditionalBlock>,
    /// Filter objects keyed by the sound-channel index.
    pub filters: Vec<(usize, SoundFilter)>,
}

impl BmsonKaleid {
    /// Build the sidecar, or `None` when the file carries no extension.
    #[must_use]
    pub fn from_bmson(bmson: &Bmson) -> Option<Self> {
        let filters: Vec<_> = bmson
            .chart_data
            .sound_channels
            .iter()
            .enumerate()
            .filter_map(|(index, channel)| channel.filter().map(|filter| (index, filter)))
            .collect();
        let kaleid = Self {
            difficulty: bmson.chart_info.difficulty.clone(),
            comment: bmson.chart_info.comment.clone(),
            display_bpm: bmson.chart_info.display_bpm.clone(),
            midi_file: bmson.chart_info.midi_file.clone(),
            chara_file: bmson.chart_info.chara_file.clone(),
            speed_events: bmson.speed_events.clone(),
            text_events: bmson.text_events.clone(),
            judge_rank_events: bmson.judge_rank_events.clone(),
            free_zone_channels: bmson.free_zone_channels.clone(),
            potion_notes: bmson.potion_notes.clone(),
            dummy_notes: bmson.dummy_notes.clone(),
            dummy_mine_notes: bmson.dummy_mine_notes.clone(),
            dummy_potion_notes: bmson.dummy_potion_notes.clone(),
            conditional_blocks: bmson.conditional_blocks.clone(),
            filters,
        };
        if kaleid.is_empty() {
            None
        } else {
            Some(kaleid)
        }
    }

    /// Return whether every extension field is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.difficulty.is_none()
            && self.comment.is_empty()
            && self.display_bpm.is_none()
            && self.midi_file.is_none()
            && self.chara_file.is_none()
            && self.speed_events.is_empty()
            && self.text_events.is_empty()
            && self.judge_rank_events.is_empty()
            && self.free_zone_channels.is_empty()
            && self.potion_notes.is_empty()
            && self.dummy_notes.is_empty()
            && self.dummy_mine_notes.is_empty()
            && self.dummy_potion_notes.is_empty()
            && self.conditional_blocks.is_empty()
            && self.filters.is_empty()
    }
}
