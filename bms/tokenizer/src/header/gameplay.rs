//! `#difficulty`, `#total`, `#rank`, `#bpm`, `#exbpm` and related gameplay parameters.
//!
//! This module also defines the domain types used by [`BmsHeaderGameplay`]:
//! [`PlayerMode`], [`LnType`], and [`LnMode`].

use std::str::FromStr;

use thiserror::Error;

use crate::id::{BmsChannelId, ExRankTag, LnObjTag};

// ---------------------------------------------------------------------------
// PlayerMode
// ---------------------------------------------------------------------------

/// The play mode specified by `#PLAYER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerMode {
    /// 1-player (Single Play). Value: `"1"` or `"SP"`.
    Single,
    /// 2-player co-op (Couple Play). Value: `"2"` or `"CP"`.
    Couple,
    /// Double Play (one player, two sides). Value: `"3"` or `"DP"`.
    Double,
    /// Battle Play (two players, same chart). Value: `"4"` or `"BP"`.
    Battle,
}

/// Error returned when a `#PLAYER` value cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid #PLAYER value: {0}")]
pub struct ParsePlayerModeError(pub String);

impl FromStr for PlayerMode {
    type Err = ParsePlayerModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "1" | "SP" | "sp" | "Sp" => Ok(PlayerMode::Single),
            "2" | "CP" | "cp" | "Cp" => Ok(PlayerMode::Couple),
            "3" | "DP" | "dp" | "Dp" => Ok(PlayerMode::Double),
            "4" | "BP" | "bp" | "Bp" => Ok(PlayerMode::Battle),
            _ => Err(ParsePlayerModeError(s.to_owned())),
        }
    }
}

// ---------------------------------------------------------------------------
// LnType
// ---------------------------------------------------------------------------

/// The long-note type specified by `#LNTYPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LnType {
    /// RDM-type LN (`#LNTYPE 1`).
    Type1,
    /// MGQ-type LN (`#LNTYPE 2`).
    Type2,
}

/// Error returned when a `#LNTYPE` value is not `"1"` or `"2"`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid #LNTYPE value: {0} (expected 1 or 2)")]
pub struct ParseLnTypeError(pub String);

impl FromStr for LnType {
    type Err = ParseLnTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "1" | "01" => Ok(LnType::Type1),
            "2" | "02" => Ok(LnType::Type2),
            _ => Err(ParseLnTypeError(s.to_owned())),
        }
    }
}

// ---------------------------------------------------------------------------
// LnMode
// ---------------------------------------------------------------------------

/// The LN mode specified by `#LNMODE` (beatoraja extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LnMode {
    /// Standard long note (`#LNMODE 1`).
    Ln,
    /// Charge note (`#LNMODE 2`).
    Cn,
    /// Hell charge note (`#LNMODE 3`).
    Hcn,
}

/// Error returned when a `#LNMODE` value is not `"1"`, `"2"`, or `"3"`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid #LNMODE value: {0} (expected 1, 2, or 3)")]
pub struct ParseLnModeError(pub String);

impl FromStr for LnMode {
    type Err = ParseLnModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "1" | "01" => Ok(LnMode::Ln),
            "2" | "02" => Ok(LnMode::Cn),
            "3" | "03" => Ok(LnMode::Hcn),
            _ => Err(ParseLnModeError(s.to_owned())),
        }
    }
}

// ---------------------------------------------------------------------------
// BmsHeaderGameplay
// ---------------------------------------------------------------------------

/// Gameplay behaviour headers.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsHeaderGameplay<'a> {
    /// `#PLAYER`
    Player(PlayerMode),
    /// `#RANK`
    Rank(u8),
    /// `#DEFEXRANK`
    DefExRank(f64),
    /// `#EXRANKxx` with its 2-character index.
    ExRank {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        index: BmsChannelId<ExRankTag>,
        /// The raw value string.
        value: f64,
    },
    /// `#TOTAL`
    Total(f64),
    /// `#VOLWAV`
    VolWav(f64),
    /// `#LNTYPE`
    LnType(LnType),
    /// `#LNOBJ`
    LnObj(BmsChannelId<LnObjTag>),
    /// `#LNMODE` (beatoraja extension)
    LnMode(LnMode),
    /// `#OCT`
    Oct(f64),
    /// `#FP`
    Fp(f64),
    /// `#OPTION`
    Option(&'a str),
    /// `#CHANGEOPTION`
    ChangeOption(&'a str),
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
