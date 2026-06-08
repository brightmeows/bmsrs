//! `#stop`, `#setrandom` and related flow-control commands.

/// Control-flow headers.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsHeaderControlFlow {
    /// `#RANDOM`
    Random(u64),
    /// `#SETRANDOM`
    SetRandom(u64),
    /// `#ENDRANDOM`
    EndRandom,
    /// `#IF`
    If(u64),
    /// `#ELSEIF`
    ElseIf(u64),
    /// `#ELSE`
    Else,
    /// `#ENDIF`
    EndIf,
    /// `#SWITCH`
    Switch(u64),
    /// `#SETSWITCH`
    SetSwitch(u64),
    /// `#ENDSW` or `#ENDSWITCH`
    EndSwitch,
    /// `#CASE`
    Case(u64),
    /// `#SKIP`
    Skip(u64),
    /// `#DEF`
    Def,
}
