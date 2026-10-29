//! Gameplay behavior headers: `#PLAYER`, `#RANK`, `#TOTAL`, `#VOLWAV`,
//! `#LNTYPE`, `#LNOBJ`, `#LNMODE`, `#OCT/FP`, `#OPTION`, `#CHANGEOPTION`, and
//! `#BASE`.
//!
//! This module also defines the domain types used by [`BmsHeaderGameplay`]:
//! [`PlayerMode`], [`Rank`], [`LnType`], [`LnMode`], and [`BmsBaseMode`].

use std::fmt;
use std::str::FromStr;

use crate::BmsTokenAttr;
use crate::index::{BmsBase, ChangeOptionIndex, ExRankIndex, LnObjIndex};

/// Gameplay mode specified by `#PLAYER`.
///
/// Modern players (LR2, nanasi, ruvit, beatoraja) usually **ignore** `#PLAYER`
/// and infer the actual mode from the channels present in the chart. The
/// command is kept only for backward compatibility.
///
/// | Value | Mode | Life gauge | Notes |
/// |-------|------|---------------|-------|
/// | `1` / `SP` | Single Play | 1 | default; 1P side only |
/// | `2` / `CP` | Couple Play | 2 | two-player co-op; rarely supported today |
/// | `3` / `DP` | Double Play | 1 | one player uses both sides |
/// | `4` / `BP` | Battle Play | 2 | two players play the same chart; BM98 only |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum PlayerMode {
    /// Single Play. Values: `"1"` or `"SP"`.
    #[bms_token("1")]
    #[bms_token("SP")]
    #[bms_token("sp")]
    #[bms_token("Sp")]
    Single,
    /// Couple Play (two-player co-op). Values: `"2"` or `"CP"`.
    #[bms_token("2")]
    #[bms_token("CP")]
    #[bms_token("cp")]
    #[bms_token("Cp")]
    Couple,
    /// Double Play (one player on both sides). Values: `"3"` or `"DP"`.
    #[bms_token("3")]
    #[bms_token("DP")]
    #[bms_token("dp")]
    #[bms_token("Dp")]
    Double,
    /// Battle Play (two players on the same chart). Values: `"4"` or `"BP"`.
    #[bms_token("4")]
    #[bms_token("BP")]
    #[bms_token("bp")]
    #[bms_token("Bp")]
    Battle,
}

/// Long-note notation specified by `#LNTYPE`.
///
/// - **RDM** (`Type1`, `#LNTYPE 1`): a long note starts at the first non-`00`
///   note and ends at the next non-`00` note. This is the modern default;
///   omitting `#LNTYPE` means RDM.
/// - **MGQ** (`Type2`, `#LNTYPE 2`): a long note continues while non-`00`
///   notes are consecutive and closes at a `00`. **Deprecated**: no modern
///   player uses MGQ notation.
///
/// Both types use channels `#xxx51-69`. The alternative is
/// [`LnObj`](BmsHeaderGameplay::LnObj) (RDM type #2), which consumes a `#WAV`
/// index as a long-note terminator marker, letting authors edit long notes on
/// ordinary `#xxx11-29` channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum LnType {
    /// RDM-type long notes (`#LNTYPE 1`).
    #[bms_token("1")]
    #[bms_token("01")]
    Type1,
    /// MGQ-type long notes (`#LNTYPE 2`).
    #[bms_token("2")]
    #[bms_token("02")]
    Type2,
}

/// Long-note mode specified by `#LNMODE` (beatoraja extension).
///
/// Determines how long notes behave during play in beatoraja. When present, the
/// chart's long-note type is **forced**, regardless of the player's LN MODE
/// option.
///
/// | Value | Mode | Behavior |
/// |-------|------|-----------|
/// | `1` | LN | standard long note: press at the start, release at the end |
/// | `2` | CN | charge note: hold through the note; no release required at the end |
/// | `3` | HCN | hell charge note: like CN but with stricter judgment |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum LnMode {
    /// Standard long note (`#LNMODE 1`).
    #[bms_token("1")]
    #[bms_token("01")]
    Ln,
    /// Charge note (`#LNMODE 2`).
    #[bms_token("2")]
    #[bms_token("02")]
    Cn,
    /// Hell charge note (`#LNMODE 3`).
    #[bms_token("3")]
    #[bms_token("03")]
    Hcn,
}

/// Judgment difficulty specified by `#RANK`.
///
/// Controls how strict the timing windows are. Standard values 0-4 map to named
/// variants; non-standard values (such as fgt++ relative rank) are preserved as
/// [`Rank::Other`].
///
/// Default when `#RANK` is omitted: **`Normal` (2)** in most players. Notable
/// exceptions: BMSE and iBMSC default to `Easy` (3).
///
/// | Value | Label | Approximate window (LR2) | Notes |
/// |-------|-------|----------------------|-------|
/// | `0` | VERY HARD | ±8 ms | |
/// | `1` | HARD | ±15 ms | |
/// | `2` | NORMAL | ±18 ms | default |
/// | `3` | EASY | ±21 ms | |
/// | `4` | VERY EASY | n/a | nanasi/beatoraja extension |
///
/// Some players (fgt++, Angolmois, `TechnicalGroove`) accept values outside
/// 0-4 and treat them as relative multipliers. Those are captured by
/// [`Rank::Other`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    /// `#RANK 0`: VERY HARD (±8 ms in LR2).
    VeryHard,
    /// `#RANK 1`: HARD (±15 ms in LR2).
    Hard,
    /// `#RANK 2`: NORMAL (±18 ms in LR2). The default when `#RANK` is omitted.
    Normal,
    /// `#RANK 3`: EASY (±21 ms in LR2).
    Easy,
    /// `#RANK 4`: VERY EASY (nanasi/beatoraja extension).
    VeryEasy,
    /// Non-standard rank value preserved for forward compatibility.
    Other(u8),
}

impl FromStr for Rank {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: u8 = s.parse()?;
        Ok(match v {
            0 => Self::VeryHard,
            1 => Self::Hard,
            2 => Self::Normal,
            3 => Self::Easy,
            4 => Self::VeryEasy,
            n => Self::Other(n),
        })
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = match self {
            Self::VeryHard => 0,
            Self::Hard => 1,
            Self::Normal => 2,
            Self::Easy => 3,
            Self::VeryEasy => 4,
            Self::Other(n) => *n,
        };
        write!(f, "{v}")
    }
}

// BmsBase is defined in crate::index::BmsBase (the same type as the former
// BmsBaseMode). It is used here through an import; the BmsTokenAttr derive
// ensures `#BASE 16`, `#BASE 36`, and `#BASE 62` parse correctly.

/// Gameplay behavior headers.
///
/// These commands control *how* the chart is played: judgment strictness, life
/// gauge behavior, long-note interpretation, and chart options.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderGameplay {
    /// `#PLAYER`: game mode (Single / Couple / Double / Battle).
    ///
    /// Modern players mostly ignore it and infer the mode from channels.
    #[bms_token("#PLAYER {}")]
    Player(PlayerMode),
    /// `#RANK`: judgment difficulty (VERY HARD ... VERY EASY).
    ///
    /// Default when omitted: `Normal` (2).
    #[bms_token("#RANK {}")]
    Rank(Rank),
    /// `#DEFEXRANK`: fine-grained judgment difficulty as a percentage.
    ///
    /// `100` equals `#RANK 2` (NORMAL). When both are present it overrides
    /// `#RANK` (the line closest to EOF wins). Decimal values are supported.
    #[bms_token("#DEFEXRANK {}")]
    DefExRank(f64),
    /// `#EXRANK{id}`: per-position judgment override.
    ///
    /// Referenced by channel `#xxxA0`. When an `#EXRANK` object crosses the
    /// judgment line, the judgment window changes to the specified percentage.
    /// In nanasi the displayed difficulty label becomes "RANDOM".
    #[bms_token("#EXRANK{id} {value}")]
    ExRank {
        /// 2-character index.
        id: ExRankIndex,
        /// Judgment width as a percentage (NORMAL = 100).
        value: f64,
    },
    /// `#TOTAL`: maximum life gauge increase (percentage).
    ///
    /// Summing all perfect judgments increases the gauge by `TOTAL / N`
    /// percentage points per note, where `N` is the total number of visible
    /// notes. For example, `#TOTAL 200` with 400 notes gives +0.5% per note.
    ///
    /// **Strongly recommended** to always specify: defaults vary widely by
    /// player (BM98: `200+NOTES`; LR2: `160`; nanasi: `350`; fgt++:
    /// `100+NOTES/8`).
    ///
    /// Some players (nazo, nazoZZ) support negative values, making *perfect*
    /// judgments *decrease* the life gauge.
    #[bms_token("#TOTAL {}")]
    Total(f64),
    /// `#VOLWAV`: master volume percentage for all audio.
    ///
    /// `100` means original volume. Default: `100`.
    ///
    /// **Deprecated**: highly implementation- and hardware-dependent. Results
    /// vary by player and driver. beatoraja caps it at 100.
    #[bms_token("#VOLWAV {}")]
    VolWav(f64),
    /// `#LNTYPE`: long-note notation (RDM or MGQ).
    ///
    /// `1` means RDM (default); `2` means MGQ (deprecated).
    #[bms_token("#LNTYPE {}")]
    LnType(LnType),
    /// `#LNOBJ`: designates a `#WAV` index as a long-note terminator marker.
    ///
    /// When a note with this index appears on channels `#xxx11-29`, it acts as
    /// the *end* of a long note (the previous visible note is the start). This
    /// is an alternative to `#LNTYPE 1` with channels `#xxx51-69`; it became
    /// popular because BMSE crashes when `#xxx51-69` objects are moved to BGM.
    ///
    /// **Note**: nanasi and fgt++ have a bug where a lowercase index is not
    /// recognized as an `#LNOBJ` marker; use uppercase.
    #[bms_token("#LNOBJ {}")]
    LnObj(LnObjIndex),
    /// `#LNMODE`: forces LN / CN / HCN mode (beatoraja extension).
    ///
    /// When present, the chart's long-note type is locked regardless of the
    /// player's LN MODE option.
    #[bms_token("#LNMODE {}")]
    LnMode(LnMode),
    /// `#OCT` / `#FP` / `#OCT/FP`: OCTAVE MODE flag.
    ///
    /// Originally a nanasi marker for a 14KEYS to OCT/FP visual remap. The
    /// value is discarded; no known player uses it. Regardless of the input
    /// form, `format_header` always emits `#OCT/FP`.
    #[bms_token("#OCT/FP")]
    #[bms_token("#OCT")]
    #[bms_token("#FP")]
    OctFp,
    /// `#OPTION`: forces player-side options from the BMS file (nanasi).
    ///
    /// Values use a vendor prefix (for example, `774:HI-SPEED_x0.77`). Multiple
    /// `#OPTION` lines may coexist; for the same option, the line closest to
    /// EOF is used.
    #[bms_token("#OPTION {}")]
    Option(String),
    /// `#CHANGEOPTION{id}`: changes options dynamically during play (nanasi).
    ///
    /// Referenced by channel `#xxxA6`. Not every option supports dynamic
    /// changes (for example, the `RANDOM` and `NOTES` families do not).
    #[bms_token("#CHANGEOPTION{id} {value}")]
    ChangeOption {
        /// 2-character index.
        id: ChangeOptionIndex,
        /// Option string (for example, `"774:HIDDEN_STEALTH"`).
        value: String,
    },
    /// `#BASE`: declares the radix base for indexed commands.
    ///
    /// Valid values: `16` (hexadecimal, 256 slots), `36` (base-36, 1296 slots,
    /// default), and `62` (case-sensitive base-62, 3844 slots, beatoraja
    /// extension). Unknown values fall back to `BmsHeaderFallback`.
    #[bms_token("#BASE {}")]
    #[bms_fallback]
    Base(BmsBase),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_mode_single() {
        assert_eq!("1".parse::<PlayerMode>().unwrap(), PlayerMode::Single);
        assert_eq!("SP".parse::<PlayerMode>().unwrap(), PlayerMode::Single);
        assert_eq!("sp".parse::<PlayerMode>().unwrap(), PlayerMode::Single);
    }

    #[test]
    fn player_mode_couple() {
        assert_eq!("2".parse::<PlayerMode>().unwrap(), PlayerMode::Couple);
        assert_eq!("CP".parse::<PlayerMode>().unwrap(), PlayerMode::Couple);
    }

    #[test]
    fn player_mode_double() {
        assert_eq!("3".parse::<PlayerMode>().unwrap(), PlayerMode::Double);
        assert_eq!("DP".parse::<PlayerMode>().unwrap(), PlayerMode::Double);
    }

    #[test]
    fn player_mode_battle() {
        assert_eq!("4".parse::<PlayerMode>().unwrap(), PlayerMode::Battle);
        assert_eq!("BP".parse::<PlayerMode>().unwrap(), PlayerMode::Battle);
    }

    #[test]
    fn player_mode_invalid() {
        assert!("5".parse::<PlayerMode>().is_err());
        assert!("".parse::<PlayerMode>().is_err());
        assert!("abc".parse::<PlayerMode>().is_err());
        assert!("0".parse::<PlayerMode>().is_err());
    }

    #[test]
    fn ln_type_1() {
        assert_eq!("1".parse::<LnType>().unwrap(), LnType::Type1);
        assert_eq!("01".parse::<LnType>().unwrap(), LnType::Type1);
    }

    #[test]
    fn ln_type_2() {
        assert_eq!("2".parse::<LnType>().unwrap(), LnType::Type2);
        assert_eq!("02".parse::<LnType>().unwrap(), LnType::Type2);
    }

    #[test]
    fn ln_type_invalid() {
        assert!("0".parse::<LnType>().is_err());
        assert!("3".parse::<LnType>().is_err());
        assert!("abc".parse::<LnType>().is_err());
    }

    #[test]
    fn ln_mode_ln() {
        assert_eq!("1".parse::<LnMode>().unwrap(), LnMode::Ln);
        assert_eq!("01".parse::<LnMode>().unwrap(), LnMode::Ln);
    }

    #[test]
    fn ln_mode_cn() {
        assert_eq!("2".parse::<LnMode>().unwrap(), LnMode::Cn);
        assert_eq!("02".parse::<LnMode>().unwrap(), LnMode::Cn);
    }

    #[test]
    fn ln_mode_hcn() {
        assert_eq!("3".parse::<LnMode>().unwrap(), LnMode::Hcn);
        assert_eq!("03".parse::<LnMode>().unwrap(), LnMode::Hcn);
    }

    #[test]
    fn ln_mode_invalid() {
        assert!("0".parse::<LnMode>().is_err());
        assert!("4".parse::<LnMode>().is_err());
        assert!("abc".parse::<LnMode>().is_err());
    }

    #[test]
    fn rank_standard_values() {
        assert_eq!("0".parse::<Rank>().unwrap(), Rank::VeryHard);
        assert_eq!("1".parse::<Rank>().unwrap(), Rank::Hard);
        assert_eq!("2".parse::<Rank>().unwrap(), Rank::Normal);
        assert_eq!("3".parse::<Rank>().unwrap(), Rank::Easy);
        assert_eq!("4".parse::<Rank>().unwrap(), Rank::VeryEasy);
    }

    #[test]
    fn rank_non_standard_preserved() {
        assert_eq!("5".parse::<Rank>().unwrap(), Rank::Other(5));
        assert_eq!("255".parse::<Rank>().unwrap(), Rank::Other(255));
    }

    #[test]
    fn rank_non_numeric_is_error() {
        assert!("abc".parse::<Rank>().is_err());
        assert!("".parse::<Rank>().is_err());
    }

    #[test]
    fn rank_display_roundtrip() {
        assert_eq!("0".parse::<Rank>().unwrap().to_string(), "0");
        assert_eq!("4".parse::<Rank>().unwrap().to_string(), "4");
        assert_eq!("5".parse::<Rank>().unwrap().to_string(), "5");
    }
}
