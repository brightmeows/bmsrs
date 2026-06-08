//! `#stop`, `#setrandom` and related flow-control commands.

use crate::BmsTokenAttr;

/// Control-flow headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderControlFlow {
    /// `#RANDOM`
    #[bms_token("#RANDOM {value}")]
    Random(u64),
    /// `#SETRANDOM`
    #[bms_token("#SETRANDOM {value}")]
    SetRandom(u64),
    /// `#ENDRANDOM`
    #[bms_token("#ENDRANDOM")]
    EndRandom,
    /// `#IF`
    #[bms_token("#IF {value}")]
    If(u64),
    /// `#ELSEIF`
    #[bms_token("#ELSEIF {value}")]
    ElseIf(u64),
    /// `#ELSE`
    #[bms_token("#ELSE")]
    Else,
    /// `#ENDIF`
    #[bms_token("#ENDIF")]
    EndIf,
    /// `#SWITCH`
    #[bms_token("#SWITCH {value}")]
    Switch(u64),
    /// `#SETSWITCH`
    #[bms_token("#SETSWITCH {value}")]
    SetSwitch(u64),
    /// `#ENDSW` or `#ENDSWITCH`
    #[bms_token("#ENDSW")]
    #[bms_token("#ENDSWITCH")]
    EndSwitch,
    /// `#CASE`
    #[bms_token("#CASE {value}")]
    Case(u64),
    /// `#SKIP`
    #[bms_token("#SKIP {value}")]
    Skip(u64),
    /// `#DEF`
    #[bms_token("#DEF")]
    Def,
}
