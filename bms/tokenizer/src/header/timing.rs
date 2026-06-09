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
    /// `#EXBPM{id}` — extended BPM definition (alias of `#BPM{id}`).
    #[bms_token("#EXBPM{id} {value}")]
    ExBpm {
        /// The 2-character index.
        id: BmsChannelId<BpmTag>,
        /// The BPM value.
        value: f64,
    },
    /// `#STP` — step timing adjustment (non-standard format).
    ///
    /// Hand-parsed in `parse_header_line` because the value format
    /// `xxx.yyy zzzz` does not follow standard header patterns.
    ///
    /// **Note:** `format_header` returns `("Stp".to_owned(), String::new())`
    /// — not a valid round-trip representation.  The hand-parsed `Stp`
    /// variant is not expected to be serialised back to BMS text.
    Stp {
        /// Measure number.
        measure: u16,
        /// Position within the measure (0–255).
        position: u16,
        /// Duration in milliseconds.
        duration_ms: f64,
    },
}
