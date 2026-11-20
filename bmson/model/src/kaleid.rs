//! `KaleidBeat` extensions to bmson.
//!
//! Every type in this module is a `KaleidBeat` extension: none of them are part
//! of the bmson v0/v1/v2 schemas or beatoraja.

use serde::{Deserialize, Serialize};

/// Chart difficulty declared directly instead of inferred from `chart_name`.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja. The JSON
/// value is either a number (1-5) or a difficulty name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Difficulty {
    /// Numeric difficulty (1 = BEGINNER through 5 = INSANE).
    Number(u8),
    /// Difficulty name such as `HYPER` or `ANOTHER`.
    Name(String),
}

/// A speed (spacing factor) keyframe.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeedEvent {
    /// Pulse position.
    pub y: u64,
    /// Spacing multiplier; `0` stops and negatives reverse.
    pub rate: f64,
}

/// A timed text event.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEvent {
    /// Pulse position.
    pub y: u64,
    /// Text body; an empty string clears the text.
    pub text: String,
}

/// A judge-rank keyframe.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JudgeRankEvent {
    /// Pulse position.
    pub y: u64,
    /// Judge rank percentage (`100` is normal).
    pub judge_rank: f64,
}

/// A free-zone note inside a [`FreeZoneChannel`].
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeZoneNote {
    /// Lane (`8` or `16` for the scratch lanes).
    pub x: u8,
    /// Pulse position.
    pub y: u64,
    /// Zone length in pulses; omitted means one quarter note.
    #[serde(default)]
    pub l: Option<u64>,
}

/// A free-zone sound channel.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeZoneChannel {
    /// Optional sound name for the zone.
    #[serde(default)]
    pub name: String,
    /// Zones in this channel.
    #[serde(default)]
    pub notes: Vec<FreeZoneNote>,
}

/// A potion note.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PotionNote {
    /// Lane.
    pub x: u8,
    /// Pulse position.
    pub y: u64,
    /// Recovery percentage.
    pub recovery: f64,
}

/// A display-only dummy note.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DummyNote {
    /// Lane.
    pub x: u8,
    /// Pulse position.
    pub y: u64,
    /// Length in pulses; `0` means a tap.
    #[serde(default)]
    pub l: u64,
    /// Long-note type (1 = LN, 2 = CN, 3 = HCN, 4 = HLN).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<u8>,
}

/// A display-only dummy mine.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DummyMineNote {
    /// Lane.
    pub x: u8,
    /// Pulse position.
    pub y: u64,
}

/// A display-only dummy potion.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DummyPotionNote {
    /// Lane.
    pub x: u8,
    /// Pulse position.
    pub y: u64,
}

/// A `#WAV` / `#BMD` filter object carried in `sound_channels[].name`.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja. The raw
/// text is kept because the manual only publishes an example grammar.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundFilter {
    /// Filter kind.
    pub kind: SoundFilterKind,
    /// Numeric parameters after the kind, in source order.
    pub params: Vec<f64>,
    /// Optional `Q` value.
    pub q: Option<f64>,
    /// Original filter text.
    pub raw: String,
}

/// Filter kind for [`SoundFilter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundFilterKind {
    /// Low-pass filter (`LPF`).
    Lpf,
    /// High-pass filter (`HPF`).
    Hpf,
}

impl SoundFilter {
    /// Parse `LPF ...` or `HPF ...` from a sound-channel name.
    ///
    /// Returns `None` when the text is an ordinary file path. Unknown tokens
    /// are ignored, but the raw text stays available for round-tripping.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split_whitespace();
        let kind = match parts.next()? {
            "LPF" | "lpf" => SoundFilterKind::Lpf,
            "HPF" | "hpf" => SoundFilterKind::Hpf,
            _ => return None,
        };
        let mut params: Vec<f64> = Vec::new();
        let mut q = None;
        for part in parts {
            if let Some(value) = part.strip_prefix('Q').or_else(|| part.strip_prefix('q')) {
                q = value.parse().ok();
            } else if let Ok(value) = part.parse() {
                params.push(value);
            }
        }
        Some(Self {
            kind,
            params,
            q,
            raw: text.to_owned(),
        })
    }
}

/// A conditional block at a pulse position.
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditionalBlock {
    /// Evaluation pulse.
    pub y: u64,
    /// Branches in source order; one branch must be `default`.
    #[serde(default)]
    pub branches: Vec<ConditionalBranch>,
}

/// One branch of a [`ConditionalBlock`].
///
/// `KaleidBeat` extension: not part of the bmson schema or beatoraja.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConditionalBranch {
    /// Condition expression, using the same grammar as BMS `#WHEN`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Whether this is the required fallback branch.
    #[serde(default)]
    pub default: bool,
    /// Sound channels active in this branch.
    #[serde(default)]
    pub sound_channels: Vec<crate::SoundChannel>,
    /// Invisible key channels active in this branch.
    #[serde(default)]
    pub key_channels: Vec<crate::KeyChannel>,
    /// Mine channels active in this branch.
    #[serde(default)]
    pub mine_channels: Vec<crate::MineChannel>,
    /// Free-zone channels active in this branch.
    #[serde(default)]
    pub free_zone_channels: Vec<FreeZoneChannel>,
    /// Potion notes active in this branch.
    #[serde(default)]
    pub potion_notes: Vec<PotionNote>,
    /// Dummy notes active in this branch.
    #[serde(default)]
    pub dummy_notes: Vec<DummyNote>,
    /// Dummy mines active in this branch.
    #[serde(default)]
    pub dummy_mine_notes: Vec<DummyMineNote>,
    /// Dummy potions active in this branch.
    #[serde(default)]
    pub dummy_potion_notes: Vec<DummyPotionNote>,
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
    pub speed_events: Vec<SpeedEvent>,
    /// Bar lines active in this branch.
    #[serde(default)]
    pub lines: Vec<crate::BarLine>,
    /// BGA data active in this branch.
    #[serde(default)]
    pub bga: crate::BGA,
}
