//! Gameplay behaviour headers: `#PLAYER`, `#RANK`, `#TOTAL`, `#VOLWAV`,
//! `#LNTYPE`, `#LNOBJ`, `#LNMODE`, `#OCT/FP`, `#OPTION`, `#CHANGEOPTION`,
//! `#BASE`.
//!
//! This module also defines the domain types used by [`BmsHeaderGameplay`]:
//! [`PlayerMode`], [`Rank`], [`LnType`], [`LnMode`], and [`BmsBaseMode`].

use std::fmt;
use std::str::FromStr;

use crate::BmsTokenAttr;
use crate::id::{BmsChannelId, ChangeOptionTag, ExRankTag, LnObjTag};

/// The play mode specified by `#PLAYER`.
///
/// Modern players (LR2, nanasi, ruvit, beatoraja) generally **ignore**
/// `#PLAYER` and infer the actual mode from the channels present in the
/// chart.  The command is retained for backward compatibility only.
///
/// | Value | Mode | Groove gauges | Notes |
/// |-------|------|---------------|-------|
/// | `1` / `SP` | Single Play | 1 | default; 1P side only |
/// | `2` / `CP` | Couple Play | 2 | two players co-op; rarely supported today |
/// | `3` / `DP` | Double Play | 1 | one player uses both sides |
/// | `4` / `BP` | Battle Play | 2 | two players on the same chart; only BM98 supports this |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum PlayerMode {
    /// 1-player (Single Play). Value: `"1"` or `"SP"`.
    #[bms_token("1")]
    #[bms_token("SP")]
    #[bms_token("sp")]
    #[bms_token("Sp")]
    Single,
    /// 2-player co-op (Couple Play). Value: `"2"` or `"CP"`.
    #[bms_token("2")]
    #[bms_token("CP")]
    #[bms_token("cp")]
    #[bms_token("Cp")]
    Couple,
    /// Double Play (one player, two sides). Value: `"3"` or `"DP"`.
    #[bms_token("3")]
    #[bms_token("DP")]
    #[bms_token("dp")]
    #[bms_token("Dp")]
    Double,
    /// Battle Play (two players, same chart). Value: `"4"` or `"BP"`.
    #[bms_token("4")]
    #[bms_token("BP")]
    #[bms_token("bp")]
    #[bms_token("Bp")]
    Battle,
}

/// The long-note notation specified by `#LNTYPE`.
///
/// - **RDM** (`Type1`, `#LNTYPE 1`): the LN starts at the first non-`00`
///   note and ends at the next non-`00` note.  This is the modern default;
///   omitting `#LNTYPE` implies RDM.
/// - **MGQ** (`Type2`, `#LNTYPE 2`): the LN persists while non-`00` notes
///   are consecutive and closes on `00`.  **Obsolete** — no modern player
///   uses MGQ notation.
///
/// Both types use channels `#xxx51-69`.  An alternative approach is
/// [`LnObj`](BmsHeaderGameplay::LnObj) (RDM-type #2), which consumes one
/// `#WAV` index as an LN termination marker and lets authors edit LNs on
/// the normal `#xxx11-29` channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum LnType {
    /// RDM-type LN (`#LNTYPE 1`).
    #[bms_token("1")]
    #[bms_token("01")]
    Type1,
    /// MGQ-type LN (`#LNTYPE 2`).
    #[bms_token("2")]
    #[bms_token("02")]
    Type2,
}

/// The LN mode specified by `#LNMODE` (beatoraja extension).
///
/// Determines how long notes behave when the chart is played in beatoraja.
/// When present, the chart's LN kind is **forced** and unaffected by the
/// player's LN MODE option.
///
/// | Value | Mode | Behaviour |
/// |-------|------|-----------|
/// | `1` | LN | Standard long note — key down at start, key up at end |
/// | `2` | CN | Charge note — hold through the note; no key-up required at end |
/// | `3` | HCN | Hell charge note — like CN but with stricter judgment |
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

/// The judgment difficulty specified by `#RANK`.
///
/// Controls how strictly the player's timing is judged.  Standard values
/// 0–4 map to named variants; non-standard values (e.g., fgt++ relative
/// rank) are preserved as [`Rank::Other`].
///
/// Default when `#RANK` is omitted: **`Normal` (2)** (in most players).
/// Notable exceptions: BMSE and iBMSC default to `Easy` (3).
///
/// | Value | Label | Approx. window (LR2) | Notes |
/// |-------|-------|----------------------|-------|
/// | `0` | VERY HARD | ±8 ms | |
/// | `1` | HARD | ±15 ms | |
/// | `2` | NORMAL | ±18 ms | default |
/// | `3` | EASY | ±21 ms | |
/// | `4` | VERY EASY | — | nanasi/beatoraja extension |
///
/// Some players (fgt++, Angolmois, `TechnicalGroove`) accept values outside
/// 0–4 and treat them as relative multipliers.  These are captured by
/// [`Rank::Other`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    /// `#RANK 0` — VERY HARD (±8 ms in LR2).
    VeryHard,
    /// `#RANK 1` — HARD (±15 ms in LR2).
    Hard,
    /// `#RANK 2` — NORMAL (±18 ms in LR2). Default when `#RANK` is omitted.
    Normal,
    /// `#RANK 3` — EASY (±21 ms in LR2).
    Easy,
    /// `#RANK 4` — VERY EASY (nanasi/beatoraja extension).
    VeryEasy,
    /// A non-standard rank value preserved for forward compatibility.
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

/// The base numbering mode declared by `#BASE`.
///
/// Controls the character set used for two-character indices in
/// `#WAV`, `#BMP`, `#BPM`, `#STOP`, `#SCROLL`, `#SPEED`, and `#LNOBJ`.
///
/// | Value | Slots | Charset | Notes |
/// |-------|-------|---------|-------|
/// | `16`  | 256   | `[0-9A-F]` | Original BM98 format |
/// | `36`  | 1296  | `[0-9A-Z]` | Modern default (no `#BASE` = 36) |
/// | `62`  | 3844  | `[0-9A-Za-z]` | beatoraja extension |
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum BmsBaseMode {
    /// `#BASE 16` — hexadecimal (256 slots).
    #[bms_token("16")]
    Base16,
    /// `#BASE 36` — base-36 uppercase (1296 slots). Default when omitted.
    #[bms_token("36")]
    Base36,
    /// `#BASE 62` — case-sensitive base-62 (3844 slots, beatoraja extension).
    #[bms_token("62")]
    Base62,
}

/// Gameplay behaviour headers.
///
/// These commands control *how* the chart plays — judgment strictness,
/// gauge (life-bar) behaviour, long-note interpretation, and chart
/// options.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderGameplay<'a> {
    /// `#PLAYER` — game mode (Single / Couple / Double / Battle).
    ///
    /// Largely ignored by modern players, which infer mode from channels.
    #[bms_token("#PLAYER {value}")]
    Player(PlayerMode),
    /// `#RANK` — judgment difficulty (VERY HARD … VERY EASY).
    ///
    /// Default when omitted: `Normal` (2).
    #[bms_token("#RANK {value}")]
    Rank(Rank),
    /// `#DEFEXRANK` — fine-grained judgment difficulty as a percentage.
    ///
    /// `100` equals `#RANK 2` (NORMAL).  Overrides `#RANK` when both are
    /// present (the line closest to EOF wins).  Supports fractional values.
    #[bms_token("#DEFEXRANK {value}")]
    DefExRank(f64),
    /// `#EXRANK{id}` — per-position judgment override.
    ///
    /// Referenced by channel `#xxxA0`.  When an `#EXRANK` object crosses
    /// the judgment line, the judgment window changes to the specified
    /// percentage.  The chart's displayed difficulty label becomes
    /// "RANDOM" in nanasi.
    #[bms_token("#EXRANK{id} {value}")]
    ExRank {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        id: BmsChannelId<ExRankTag>,
        /// Judgment width as a percentage (NORMAL = 100).
        value: f64,
    },
    /// `#TOTAL` — maximum groove gauge increase (in percent).
    ///
    /// All notes judged perfectly will increase the gauge by `TOTAL / N`
    /// percent each, where `N` is the total visible note count.
    /// For example, `#TOTAL 200` with 400 notes gives +0.5% per note.
    ///
    /// **Strongly recommended** to always specify — the default varies
    /// wildly across players (BM98: `200+NOTES`; LR2: `160`; nanasi:
    /// `350`; fgt++: `100+NOTES/8`).
    ///
    /// Negative values are supported by some players (nazo, nazoZZ) and
    /// cause *perfect* judgments to *decrease* the gauge.
    #[bms_token("#TOTAL {value}")]
    Total(f64),
    /// `#VOLWAV` — master volume percentage for all audio.
    ///
    /// `100` = original volume.  Default: `100`.
    ///
    /// **Deprecated** — highly implementation- and hardware-dependent.
    /// Results vary across players and drivers.  beatoraja caps at 100.
    #[bms_token("#VOLWAV {value}")]
    VolWav(f64),
    /// `#LNTYPE` — long-note notation (RDM or MGQ).
    ///
    /// `1` = RDM (default); `2` = MGQ (obsolete).
    #[bms_token("#LNTYPE {value}")]
    LnType(LnType),
    /// `#LNOBJ` — designate a `#WAV` index as an LN termination marker.
    ///
    /// When a note with this index appears on channels `#xxx11-29`, it
    /// acts as the *end* of a long note (the previous visible note is the
    /// start).  This is an alternative to `#LNTYPE 1` + channels
    /// `#xxx51-69` — popular because BMSE crashes when moving `#xxx51-69`
    /// objects to BGM.
    ///
    /// **Caveat**: nanasi and fgt++ have a bug where lowercase indices
    /// are not recognised as `#LNOBJ` markers — use uppercase.
    #[bms_token("#LNOBJ {value}")]
    LnObj(BmsChannelId<LnObjTag>),
    /// `#LNMODE` — force LN / CN / HCN mode (beatoraja extension).
    ///
    /// When present, the chart's long-note type is locked regardless of
    /// the player's LN MODE option.
    #[bms_token("#LNMODE {value}")]
    LnMode(LnMode),
    /// `#OCT` / `#FP` / `#OCT/FP` — OCTAVE MODE flag.
    ///
    /// Originally a nanasi identifier for 14KEYS → OCT/FP visual remap.
    /// The numeric value is discarded — no known player uses it.
    /// `format_header` always outputs `#OCT/FP` regardless of input form.
    #[bms_token("#OCT/FP")]
    #[bms_token("#OCT")]
    #[bms_token("#FP")]
    OctFp,
    /// `#OPTION` — force player-side options from the BMS file (nanasi).
    ///
    /// Values use vendor prefixes (e.g., `774:HI-SPEED_x0.77`).
    /// Multiple `#OPTION` lines can coexist; same-category options use
    /// the line closest to EOF.
    #[bms_token("#OPTION {value}")]
    Option(&'a str),
    /// `#CHANGEOPTION{id}` — dynamically change options mid-play (nanasi).
    ///
    /// Referenced by channel `#xxxA6`.  Not all options support dynamic
    /// changes (e.g., `RANDOM`, `NOTES` series do not).
    #[bms_token("#CHANGEOPTION{id} {value}")]
    ChangeOption {
        /// The 2-character index.
        id: BmsChannelId<ChangeOptionTag>,
        /// The option string (e.g., `"774:HIDDEN_STEALTH"`).
        value: &'a str,
    },
    /// `#BASE` — declare the numbering base for indexed commands.
    ///
    /// Valid values: `16` (hex, 256 slots), `36` (base-36, 1296 slots, default),
    /// `62` (case-sensitive base-62, 3844 slots, beatoraja extension).
    /// Unknown values fall through to `BmsHeaderFallback`.
    #[bms_token("#BASE {value}")]
    #[bms_fallback]
    Base(BmsBaseMode),
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- PlayerMode --

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

    // -- LnType --

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

    // -- LnMode --

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

    // -- Rank --

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
