//! `#stop`, `#setrandom` and related flow-control commands.

/// Control-flow headers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeaderControlFlow<'a> {
    /// `#RANDOM`
    #[serde(borrow)]
    Random(&'a str),
    /// `#SETRANDOM`
    #[serde(borrow)]
    SetRandom(&'a str),
    /// `#ENDRANDOM`
    EndRandom,
    /// `#IF`
    #[serde(borrow)]
    If(&'a str),
    /// `#ELSEIF`
    #[serde(borrow)]
    ElseIf(&'a str),
    /// `#ELSE`
    Else,
    /// `#ENDIF`
    EndIf,
    /// `#SWITCH`
    #[serde(borrow)]
    Switch(&'a str),
    /// `#SETSWITCH`
    #[serde(borrow)]
    SetSwitch(&'a str),
    /// `#ENDSW` or `#ENDSWITCH`
    EndSwitch,
    /// `#CASE`
    #[serde(borrow)]
    Case(&'a str),
    /// `#SKIP`
    #[serde(borrow)]
    Skip(&'a str),
    /// `#DEF`
    Def,
}
