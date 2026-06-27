//! Timing definitions and global scalars.
//!
//! Corresponds to [`BmsHeaderTiming`] from the tokenizer.
//! Position-based timing events (BPM changes, stops, scrolls, …) are
//! stored in [`Messages`](crate::messages::Messages) alongside channel
//! events so they share the same position model.

use std::collections::BTreeMap;

use bms_tokenizer::{BmsBase, BmsHeaderTiming, BpmIndex, ScrollIndex, SpeedIndex, StopIndex};

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
    pub bpm_defs: BTreeMap<BpmIndex, f64>,
    /// Stop-sequence definitions (`#STOPxx`).
    pub stop_defs: BTreeMap<StopIndex, f64>,
    /// Scroll speed multiplier definitions (`#SCROLLxx`).
    pub scroll_defs: BTreeMap<ScrollIndex, f64>,
    /// Visual note-spacing definitions (`#SPEEDxx`).
    pub speed_defs: BTreeMap<SpeedIndex, f64>,
}

impl Timing {
    /// Apply a timing header to this struct.
    ///
    /// Indexed keys (`BpmIndex`, `StopIndex`, etc.) are normalized using
    /// `base` for case-insensitive comparison in standard BMS.
    pub fn apply(&mut self, header: &BmsHeaderTiming, base: BmsBase) {
        match header {
            BmsHeaderTiming::Bpm(v) => self.bpm = Some(*v),
            BmsHeaderTiming::BpmDef { id, value } | BmsHeaderTiming::ExBpm { id, value } => {
                let nid = BpmIndex::from(id.normalize(base));
                self.bpm_defs.insert(nid, *value);
            }
            BmsHeaderTiming::BaseBpm(v) => self.base_bpm = Some(*v),
            BmsHeaderTiming::StopDef { id, value } => {
                let nid = StopIndex::from(id.normalize(base));
                self.stop_defs.insert(nid, *value);
            }
            BmsHeaderTiming::ScrollDef { id, value } => {
                let nid = ScrollIndex::from(id.normalize(base));
                self.scroll_defs.insert(nid, *value);
            }
            BmsHeaderTiming::SpeedDef { id, value } => {
                let nid = SpeedIndex::from(id.normalize(base));
                self.speed_defs.insert(nid, *value);
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
