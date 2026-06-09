//! `#difficulty`, `#total`, `#rank`, `#bpm`, `#exbpm` and related gameplay parameters.
//!
//! This module also defines the domain types used by [`BmsHeaderGameplay`]:
//! [`PlayerMode`], [`LnType`], and [`LnMode`].

use crate::BmsTokenAttr;
use crate::id::{BmsChannelId, ExRankTag, LnObjTag};

/// The play mode specified by `#PLAYER`.
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

/// The long-note type specified by `#LNTYPE`.
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

/// Gameplay behaviour headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderGameplay<'a> {
    /// `#PLAYER`
    #[bms_token("#PLAYER {value}")]
    Player(PlayerMode),
    /// `#RANK`
    #[bms_token("#RANK {value}")]
    Rank(u8),
    /// `#DEFEXRANK`
    #[bms_token("#DEFEXRANK {value}")]
    DefExRank(f64),
    /// `#EXRANK{id}` with its 2-character index.
    #[bms_token("#EXRANK{id} {value}")]
    ExRank {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        id: BmsChannelId<ExRankTag>,
        /// The raw value.
        value: f64,
    },
    /// `#TOTAL`
    #[bms_token("#TOTAL {value}")]
    Total(f64),
    /// `#VOLWAV`
    #[bms_token("#VOLWAV {value}")]
    VolWav(f64),
    /// `#LNTYPE`
    #[bms_token("#LNTYPE {value}")]
    LnType(LnType),
    /// `#LNOBJ`
    #[bms_token("#LNOBJ {value}")]
    LnObj(BmsChannelId<LnObjTag>),
    /// `#LNMODE` (beatoraja extension)
    #[bms_token("#LNMODE {value}")]
    LnMode(LnMode),
    /// `#OCT`/`#FP`/`#OCT/FP` — octave/fingering pitch flag.
    ///
    /// Originally carried a numeric value, but no known player uses it.
    /// The original value is discarded — `format_header` always outputs
    /// `#OCT/FP` regardless of which input form was used.
    #[bms_token("#OCT/FP")]
    #[bms_token("#OCT")]
    #[bms_token("#FP")]
    OctFp,
    /// `#OPTION`
    #[bms_token("#OPTION {value}")]
    Option(&'a str),
    /// `#CHANGEOPTION`
    #[bms_token("#CHANGEOPTION {value}")]
    ChangeOption(&'a str),
    /// `#BASE 62` — declares base-62 indexing for WAV/BMP channels.
    #[bms_token("#BASE 62")]
    Base62,
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
}
