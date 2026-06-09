//! Timing definition headers: `#BPM`, `#BPMxx`/`#EXBPMxx`, `#BASEBPM`,
//! `#STOPxx`, `#SCROLLxx`, `#SPEEDxx`, `#STP`.

use std::fmt;

use crate::BmsTokenAttr;
use crate::BmsValue;
use crate::id::{BmsChannelId, BpmTag, ScrollTag, SpeedTag, StopTag};

/// Parameters for `#STP` — bemaniaDX-style stop (absolute time, in ms).
///
/// Value format: `xxx[.yyy] zzzz`
/// - `xxx` = measure number (0–999, 3-digit zero-padded)
/// - `.yyy` = position within measure (0–999, optional; interpreted as
///   `yyy/1000` of a measure)
/// - `zzzz` = stop duration in milliseconds
///
/// Unlike `#STOPxx` (which is in 192nd-note units and thus BPM-dependent),
/// `#STP` always stops for a fixed wall-clock duration regardless of BPM.
///
/// Multiple `#STP` lines at the same position are additive.
///
/// **Caveats**: bemaniaDX ignores `#STP` lines beyond a certain count
/// (limit unspecified).  Values of `yyy ≥ 960` may be ignored or cause
/// freezes in bemaniaDX.  Angolmois and Sonorous have fewer quirks.
#[derive(Debug, Clone, PartialEq)]
pub struct StpParams {
    /// Measure number.
    pub measure: u16,
    /// Position within the measure (0–999, as `yyy` in `xxx.yyy`).
    ///
    /// Interpreted as `yyy/1000` of a measure.  Values ≥ 960 may be
    /// ignored or cause freezes in bemaniaDX.
    pub position: u16,
    /// Duration in milliseconds.
    pub duration_ms: f64,
}

impl fmt::Display for StpParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.position == 0 {
            write!(f, "{:03} {}", self.measure, self.duration_ms)
        } else {
            write!(
                f,
                "{:03}.{} {}",
                self.measure, self.position, self.duration_ms
            )
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
        if position > 999 {
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
///
/// These commands control *when* events happen — tempo, stops, and scroll
/// gimmicks.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderTiming {
    /// `#BPM` — global initial BPM.
    ///
    /// Default when omitted: `130` (spec), but players vary (nanasi:
    /// `150`; nazo/BMSE: `120`; fgt++: `30`; fgt#/pomu2: `0`).
    /// Supports fractional values in most players.
    #[bms_token("#BPM {value}")]
    Bpm(f64),
    /// `#BPM{id}` — extended BPM definition (bemaniaDX origin).
    ///
    /// Referenced by channel `#xxx08`.  Supports fractional and
    /// out-of-255 BPM values that the basic `#xxx03` channel (hex integer)
    /// cannot represent.
    ///
    /// Negative values cause reverse scrolling in some players, but
    /// this is a de-facto convention — the spec does not define them.
    ///
    /// `#EXBPM{id}` is a functional alias (nanasi, to work around a
    /// BMSC parsing bug).
    #[bms_token("#BPM{id} {value}")]
    BpmDef {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        id: BmsChannelId<BpmTag>,
        /// The BPM value (may be fractional or negative).
        value: f64,
    },
    /// `#BASEBPM` — reference BPM for auto HI-SPEED calculation (LR origin).
    ///
    /// Used when the chart has short extreme BPM spikes.  Normally the
    /// player's auto-speed uses the max BPM, but `#BASEBPM` lets the
    /// charter specify a more practical reference value.
    #[bms_token("#BASEBPM {value}")]
    BaseBpm(f64),
    /// `#STOP{id}` — DDR-type stop (192nd-note units).
    ///
    /// Referenced by channel `#xxx09`.  The value is in 192nd-note
    /// units of a 4/4 measure, so the actual wall-clock stop duration
    /// depends on the BPM at that position: `duration = value * 60 /
    /// (BPM * 192)`.
    ///
    /// When a STOP and a BPM change occur at the same position, the BPM
    /// change is applied first, then the stop is evaluated against the
    /// new BPM.
    ///
    /// Negative values cause forward skipping in some players (LR2,
    /// nanasi, etc.) and are ignored by others.  Fractional values are
    /// truncated (floor) by most players; only a few accept them.
    #[bms_token("#STOP{id} {value}")]
    StopDef {
        /// The 2-character index.
        id: BmsChannelId<StopTag>,
        /// Stop duration in 192nd-note units (may be fractional).
        value: f64,
    },
    /// `#SCROLL{id}` — scroll speed multiplier (beatoraja extension).
    ///
    /// Referenced by channel `#xxxSC`.  Multiplies the visual scroll
    /// speed independently of BPM.  Default is `1.0`.  Negative values
    /// cause reverse scrolling.
    #[bms_token("#SCROLL{id} {value}")]
    ScrollDef {
        /// The 2-character index.
        id: BmsChannelId<ScrollTag>,
        /// Scroll speed multiplier.
        value: f64,
    },
    /// `#SPEED{id}` — speed/spacing multiplier (pomu2 origin).
    ///
    /// Referenced by channel `#xxxSP`.  Unlike `#SCROLL` (which scales
    /// scroll speed), `#SPEED` changes the visual spacing between notes
    /// without affecting scroll speed — similar to "sudden+" / "hidden+"
    /// adjustments.  Supports interpolation between positions.
    #[bms_token("#SPEED{id} {value}")]
    SpeedDef {
        /// The 2-character index.
        id: BmsChannelId<SpeedTag>,
        /// Speed multiplier value.
        value: f64,
    },
    /// `#EXBPM{id}` — alias of `#BPM{id}` (nanasi origin).
    ///
    /// Identical in function to `#BPM{id}`.  Exists because BMSC had a
    /// bug where it confused `#BPMxx` with the global `#BPM`.  Also
    /// referenced by channel `#xxx08`.
    #[bms_token("#EXBPM{id} {value}")]
    ExBpm {
        /// The 2-character index.
        id: BmsChannelId<BpmTag>,
        /// The BPM value.
        value: f64,
    },
    /// `#STP` — bemaniaDX-style stop (absolute milliseconds).
    ///
    /// Unlike `#STOPxx` (BPM-dependent 192nd-note units), this defines
    /// a stop with a fixed wall-clock duration at a specific measure
    /// position.
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
    fn stp_params_position_999_accepted() {
        let p = StpParams::parse("001.999 500").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 999);
        assert_eq!(p.duration_ms, 500.0);
    }

    #[test]
    fn stp_params_position_over_999_rejected() {
        assert!(StpParams::parse("001.1000 500").is_none());
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
