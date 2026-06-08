//! `#stop`, `#exbpm`, `#exexbpm` timing-related commands.

use crate::BmsTokenAttr;
use crate::id::{BmsChannelId, BpmTag, ScrollTag, SpeedTag, StopTag};

/// Timing definition headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderTiming {
    /// `#BPM` (global BPM)
    #[bms_token("#BPM {value}")]
    Bpm(f64),
    /// `#BPM{id}` with its 2-character index.
    #[bms_token("#BPM{id} {value}")]
    BpmDef {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        id: BmsChannelId<BpmTag>,
        /// The raw value.
        value: f64,
    },
    /// `#BASEBPM`
    #[bms_token("#BASEBPM {value}")]
    BaseBpm(f64),
    /// `#STOP{id}`
    #[bms_token("#STOP{id} {value}")]
    StopDef {
        /// The 2-character index.
        id: BmsChannelId<StopTag>,
        /// The raw value.
        value: f64,
    },
    /// `#SCROLL{id}`
    #[bms_token("#SCROLL{id} {value}")]
    ScrollDef {
        /// The 2-character index.
        id: BmsChannelId<ScrollTag>,
        /// The raw value.
        value: f64,
    },
    /// `#SPEED{id}`
    #[bms_token("#SPEED{id} {value}")]
    SpeedDef {
        /// The 2-character index.
        id: BmsChannelId<SpeedTag>,
        /// The raw value.
        value: f64,
    },
}
