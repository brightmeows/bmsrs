//! Timing definitions and global scalars.
//!
//! Corresponds to [`BmsHeaderTiming`] from the tokenizer.
//! Position-based timing events (BPM changes, stops, scrolls, …) are
//! stored in [`Messages`](crate::messages::Messages) alongside channel
//! events so they share the same position model.

use std::collections::BTreeMap;

use bms_tokenizer::{BmsHeaderTiming, BmsIndex, BpmTag, ScrollTag, SpeedTag, StopTag};

/// Timing definitions and global scalars.
///
/// Scalar fields (`bpm`, `base_bpm`) use last-wins semantics. Indexed
/// definitions (`bpm_defs`, `stop_defs`, …) use `BTreeMap`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timing {
    /// Global initial BPM (`#BPM`).
    pub bpm: Option<f64>,
    /// Reference BPM for auto HI-SPEED (`#BASEBPM`).
    pub base_bpm: Option<f64>,
    /// Extended BPM definitions (`#BPMxx`, `#EXBPMxx`).
    pub bpm_defs: BTreeMap<BmsIndex<BpmTag>, f64>,
    /// Stop-sequence definitions (`#STOPxx`).
    pub stop_defs: BTreeMap<BmsIndex<StopTag>, f64>,
    /// Scroll speed multiplier definitions (`#SCROLLxx`).
    pub scroll_defs: BTreeMap<BmsIndex<ScrollTag>, f64>,
    /// Visual note-spacing definitions (`#SPEEDxx`).
    pub speed_defs: BTreeMap<BmsIndex<SpeedTag>, f64>,
}

impl Timing {
    /// Apply a timing header to this struct.
    pub fn apply(&mut self, header: &BmsHeaderTiming) {
        match header {
            BmsHeaderTiming::Bpm(v) => self.bpm = Some(*v),
            BmsHeaderTiming::BpmDef { id, value } | BmsHeaderTiming::ExBpm { id, value } => {
                self.bpm_defs.insert(*id, *value);
            }
            BmsHeaderTiming::BaseBpm(v) => self.base_bpm = Some(*v),
            BmsHeaderTiming::StopDef { id, value } => {
                self.stop_defs.insert(*id, *value);
            }
            BmsHeaderTiming::ScrollDef { id, value } => {
                self.scroll_defs.insert(*id, *value);
            }
            BmsHeaderTiming::SpeedDef { id, value } => {
                self.speed_defs.insert(*id, *value);
            }
            BmsHeaderTiming::Stp { .. } => {
                // `#STP` is a position-based stop; it is stored in
                // `Messages::stp_events` rather than here because it
                // uses the same position model as channel events.
                // The caller is responsible for routing it there.
            }
        }
    }
}
