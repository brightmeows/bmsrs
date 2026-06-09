//! `#stop`, `#exbpm`, `#stp` timing-related commands.

use std::fmt;

use crate::BmsTokenAttr;
use crate::BmsValue;
use crate::id::{BmsChannelId, BpmTag, ScrollTag, SpeedTag, StopTag};

/// Parameters for `#STP` — step timing adjustment.
///
/// Value format: `xxx[.yyy] zzzz`
/// - `xxx` = measure number (decimal, 1–3 digits)
/// - `.yyy` = position within measure (optional, 0–255)
/// - `zzzz` = stop duration in milliseconds (decimal)
#[derive(Debug, Clone, PartialEq)]
pub struct StpParams {
    /// Measure number.
    pub measure: u16,
    /// Position within the measure (0–255).
    pub position: u16,
    /// Duration in milliseconds.
    pub duration_ms: f64,
}

impl fmt::Display for StpParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.position == 0 {
            write!(f, "{:03} {}", self.measure, self.duration_ms)
        } else {
            write!(f, "{:03}.{} {}", self.measure, self.position, self.duration_ms)
        }
    }
}

impl<'a> BmsValue<'a> for StpParams {
    fn parse(s: &'a str) -> Option<Self> {
        let (pos_part, dur_part) = s.split_once(' ')?;
        let dur_ms: f64 = dur_part.trim().parse().ok()?;

        let (measure_str, position_str) = if let Some(dot) = pos_part.find('.') {
            (&pos_part[..dot], &pos_part[dot + 1..])
        } else {
            (pos_part, "0")
        };

        let measure: u16 = measure_str.parse().ok()?;
        let position: u16 = position_str.parse().ok()?;
        if position > 255 {
            return None;
        }

        Some(Self {
            measure,
            position,
            duration_ms: dur_ms,
        })
    }
}

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
    /// `#STP` — step timing adjustment.
    ///
    /// Parse failures fall through to the `Fallback` header because the
    /// value format `xxx[.yyy] zzzz` is non-standard.
    #[bms_token("#STP {params}")]
    #[bms_fallback]
    Stp {
        /// Parsed step timing parameters.
        params: StpParams,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stp_params_with_position() {
        let p = StpParams::parse("001.128 500").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 128);
        assert_eq!(p.duration_ms, 500.0);
    }

    #[test]
    fn stp_params_without_position() {
        let p = StpParams::parse("001 500.5").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 0);
        assert!((p.duration_ms - 500.5).abs() < f64::EPSILON);
    }

    #[test]
    fn stp_params_position_over_255_rejected() {
        assert!(StpParams::parse("001.256 500").is_none());
    }

    #[test]
    fn stp_params_invalid_format_rejected() {
        assert!(StpParams::parse("invalid").is_none());
        assert!(StpParams::parse("").is_none());
    }

    #[test]
    fn stp_params_display_with_position() {
        let p = StpParams {
            measure: 1,
            position: 128,
            duration_ms: 500.0,
        };
        assert_eq!(p.to_string(), "001.128 500");
    }

    #[test]
    fn stp_params_display_without_position() {
        let p = StpParams {
            measure: 1,
            position: 0,
            duration_ms: 500.5,
        };
        assert_eq!(p.to_string(), "001 500.5");
    }
}
