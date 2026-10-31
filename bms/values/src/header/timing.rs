//! Timing definition headers: `#BPM`, `#BPMxx`/`#EXBPMxx`, `#BASEBPM`,
//! `#STOPxx`, `#SCROLLxx`, `#SPEEDxx`, and `#STP`.

use std::fmt;

use crate::BmsTokenAttr;
use crate::BmsValue;
use crate::index::{BpmIndex, ScrollIndex, SpeedIndex, StopIndex};

/// Parameters for `#STP`, the bemaniaDX-style stop (absolute time in
/// milliseconds).
///
/// Value format: `xxx[.yyy] zzzz`
/// - `xxx` = measure number (0-999, 3-digit zero-padded)
/// - `.yyy` = position within the measure (0-999, optional; interpreted as
///   `yyy/1000` of a measure)
/// - `zzzz` = stop duration (milliseconds)
///
/// Unlike `#STOPxx` (which is in 192nd-note units and therefore depends on
/// BPM), `#STP` always stops for a fixed wall-clock duration, independent of
/// BPM.
///
/// Multiple `#STP` lines at the same position are cumulative.
///
/// **Note**: bemaniaDX ignores `#STP` lines beyond some count (the limit is
/// unspecified). Values with `yyy >= 960` may be ignored or cause a freeze in
/// bemaniaDX. Angolmois and Sonorous have fewer quirks.
#[derive(Debug, Clone, PartialEq)]
pub struct StpParams {
    /// Measure number (0-999, 3-digit zero-padded).
    pub measure: u16,
    /// Position within the measure (0-999, the `yyy` in `xxx.yyy`).
    ///
    /// Interpreted as `yyy/1000` of a measure. Values of 960 or more may be
    /// ignored or cause a freeze in bemaniaDX.
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

impl BmsValue for StpParams {
    #[expect(
        clippy::string_slice,
        reason = "pos_part is ASCII digits and dots from BMS format; byte indexing is safe"
    )]
    fn parse(s: &str) -> Option<Self> {
        let (pos_part, dur_part) = s.split_once(' ')?;
        let dur_ms: f64 = dur_part.trim().parse().ok()?;

        let (measure_str, position_str) = pos_part.find('.').map_or((pos_part, "0"), |dot| {
            (&pos_part[..dot], &pos_part[dot + 1..])
        });

        let measure: u16 = measure_str.parse().ok()?;
        let position: u16 = position_str.parse().ok()?;
        if measure > 999 || position > 999 {
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
/// These commands control *when* events happen: tempo, stops, and scroll
/// gimmicks.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderTiming {
    /// `#BPM`: global initial BPM.
    ///
    /// Default when omitted: `130` (specification), though players differ
    /// (nanasi: `150`; nazo/BMSE: `120`; fgt++: `30`; fgt#/pomu2: `0`). Decimal
    /// values are supported in most players.
    #[bms_token("#BPM {}")]
    Bpm(f64),
    /// `#BPM{id}`: extended BPM definition (bemaniaDX origin).
    ///
    /// Referenced by channel `#xxx08`. Supports decimals and BPM values above
    /// 255, which the basic `#xxx03` channel (hexadecimal integer) cannot
    /// represent.
    ///
    /// Negative values cause reverse scrolling in some players, but this is a
    /// de facto convention; the specification does not define it.
    ///
    /// `#EXBPM{id}` is a functionally equivalent alias (nanasi, to work around
    /// a BMSC parsing bug).
    #[bms_token("#BPM{id} {value}")]
    BpmDef {
        /// 2-character index (for example, `"01"`, `"2A"`).
        id: BpmIndex,
        /// BPM value (may be fractional or negative).
        value: f64,
    },
    /// `#BASEBPM`: reference BPM for automatic HI-SPEED calculation (LR origin).
    ///
    /// Used when a chart has brief extreme BPM spikes. Player auto-speed
    /// normally uses the maximum BPM, but `#BASEBPM` lets the charter specify a
    /// more useful reference value.
    #[bms_token("#BASEBPM {}")]
    BaseBpm(f64),
    /// `#STOP{id}`: DDR-style stop (units of 1/192 note).
    ///
    /// Referenced by channel `#xxx09`. The value is in 192nd notes of a 4/4
    /// measure, so the actual wall-clock stop duration depends on the BPM at
    /// that position: `duration = value * 60 / (BPM * 192)`.
    ///
    /// When a STOP and a BPM change occur at the same position, the BPM change
    /// is applied first and the stop is computed from the new BPM.
    ///
    /// Negative values cause a forward jump in some players (LR2, nanasi, and
    /// others) and are ignored by others. Fractional values are truncated
    /// (floor) by most players; only a few accept them.
    #[bms_token("#STOP{id} {value}")]
    StopDef {
        /// 2-character index.
        id: StopIndex,
        /// Stop duration (192nd-note units; may be fractional).
        value: f64,
    },
    /// `#SCROLL{id}`: scroll speed multiplier (beatoraja extension).
    ///
    /// Referenced by channel `#xxxSC`. Multiplies the visual scroll speed
    /// independently of BPM. Defaults to `1.0`. Negative values cause reverse
    /// scrolling.
    #[bms_token("#SCROLL{id} {value}")]
    ScrollDef {
        /// 2-character index.
        id: ScrollIndex,
        /// Scroll speed multiplier.
        value: f64,
    },
    /// `#SPEED{id}`: speed and spacing multiplier (pomu2 origin).
    ///
    /// Referenced by channel `#xxxSP`. Unlike `#SCROLL` (which scales scroll
    /// speed), `#SPEED` changes the visual spacing between notes without
    /// affecting scroll speed, similar to "sudden+" / "hidden+" adjustments.
    /// Supports interpolation between positions.
    #[bms_token("#SPEED{id} {value}")]
    SpeedDef {
        /// 2-character index.
        id: SpeedIndex,
        /// Speed multiplier value.
        value: f64,
    },
    /// `#EXBPM{id}`: alias for `#BPM{id}` (nanasi origin).
    ///
    /// Functionally identical to `#BPM{id}`. It exists because BMSC has a bug
    /// that confuses `#BPMxx` with the global `#BPM`. Also referenced by
    /// channel `#xxx08`.
    #[bms_token("#EXBPM{id} {value}")]
    ExBpm {
        /// 2-character index.
        id: BpmIndex,
        /// BPM value.
        value: f64,
    },
    /// `#STP`: bemaniaDX-style stop (absolute milliseconds).
    ///
    /// Unlike `#STOPxx` (BPM-dependent 192nd-note units), it defines a stop with
    /// a fixed wall-clock duration at a given measure position.
    ///
    /// A parse failure falls back to a `Fallback` header because the value
    /// format `xxx[.yyy] zzzz` is non-standard.
    #[bms_token("#STP {params}")]
    #[bms_fallback]
    Stp {
        /// Parsed step-timing parameters.
        params: StpParams,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stp_params_with_position() {
        let p = <StpParams as BmsValue>::parse("001.128 500").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 128);
        assert!((p.duration_ms - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stp_params_without_position() {
        let p = <StpParams as BmsValue>::parse("001 500.5").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 0);
        assert!((p.duration_ms - 500.5).abs() < f64::EPSILON);
    }

    #[test]
    fn stp_params_position_999_accepted() {
        let p = <StpParams as BmsValue>::parse("001.999 500").unwrap();
        assert_eq!(p.measure, 1);
        assert_eq!(p.position, 999);
        assert!((p.duration_ms - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stp_params_position_over_999_rejected() {
        assert!(<StpParams as BmsValue>::parse("001.1000 500").is_none());
    }

    #[test]
    fn stp_params_measure_0_accepted() {
        let p = <StpParams as BmsValue>::parse("000 500").unwrap();
        assert_eq!(p.measure, 0);
    }

    #[test]
    fn stp_params_measure_999_accepted() {
        let p = <StpParams as BmsValue>::parse("999 500").unwrap();
        assert_eq!(p.measure, 999);
    }

    #[test]
    fn stp_params_measure_over_999_rejected() {
        assert!(<StpParams as BmsValue>::parse("1000 500").is_none());
    }

    #[test]
    fn stp_params_invalid_format_rejected() {
        assert!(<StpParams as BmsValue>::parse("invalid").is_none());
        assert!(<StpParams as BmsValue>::parse("").is_none());
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
