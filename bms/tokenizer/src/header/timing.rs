//! `#stop`, `#exbpm`, `#exexbpm` timing-related commands.

use crate::id::{BmsChannelId, BpmTag, ScrollTag, SpeedTag, StopTag};

/// Timing definition headers.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsHeaderTiming {
    /// `#BPM` (global BPM)
    Bpm(f64),
    /// `#BPMxx` with its 2-character index.
    BpmDef {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        index: BmsChannelId<BpmTag>,
        /// The raw value string.
        value: f64,
    },
    /// `#BASEBPM`
    BaseBpm(f64),
    /// `#STOPxx`
    StopDef {
        /// The 2-character index.
        index: BmsChannelId<StopTag>,
        /// The raw value string.
        value: f64,
    },
    /// `#SCROLLxx`
    ScrollDef {
        /// The 2-character index.
        index: BmsChannelId<ScrollTag>,
        /// The raw value string.
        value: f64,
    },
    /// `#SPEEDxx`
    SpeedDef {
        /// The 2-character index.
        index: BmsChannelId<SpeedTag>,
        /// The raw value string.
        value: f64,
    },
}
