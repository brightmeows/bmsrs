//! `#RANDOM` / `#SWITCH` control-flow commands.
//!
//! These commands allow a single BMS file to contain multiple chart
//! variations.  The tokenizer preserves all branches verbatim; selecting
//! which branch to keep is the responsibility of a later pipeline stage
//! (parser/processor).

use crate::BmsTokenAttr;
use crate::{BmsHeader, BmsTryFromError};

/// Control-flow headers for random chart branching.
///
/// BMS supports two branching constructs:
///
/// - **`#RANDOM` block**: `#RANDOM N` → `#IF k` … `#ENDIF` × N → `#ENDRANDOM`.
///   At parse time, one integer in `[1, N]` is chosen; only the matching
///   `#IF` branch is retained.
/// - **`#SWITCH` block**: `#SWITCH N` → `#CASE k` … `#DEF` … `#ENDSW`.
///   Similar to `#RANDOM`, but `#CASE` matches an integer value and `#DEF`
///   provides a default fallback.
///
/// `#SETRANDOM` / `#SETSWITCH` force a specific branch (used in tools and
/// tests).  `#RONDAM` is a common typo accepted as an alias for `#RANDOM`.
///
/// Nesting and engine compatibility are complex — see the BMS command memo
/// for full details.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderControlFlow {
    /// `#RANDOM N` (or `#RONDAM`) — start a random branch block.
    ///
    /// `N` is the number of branches; the engine picks a value in `[1, N]`.
    /// `#RONDAM` is a historical typo that some players recognise.
    #[bms_token("#RANDOM {}")]
    #[bms_token("#RONDAM {}")]
    Random(u64),
    /// `#SETRANDOM N` — force a specific random value instead of rolling.
    ///
    /// Used by tools (e.g., preview, IR replay) to deterministically
    /// select a branch.
    #[bms_token("#SETRANDOM {}")]
    SetRandom(u64),
    /// `#ENDRANDOM` — close the current `#RANDOM` block.
    ///
    /// Recommended when nesting `#RANDOM` blocks, as some players
    /// (nanasi) require it for correct behaviour.
    #[bms_token("#ENDRANDOM")]
    EndRandom,
    /// `#IF N` — begin a branch that activates when the random value equals `N`.
    #[bms_token("#IF {}")]
    If(u64),
    /// `#ELSEIF N` — an alternative branch (like `else if`).
    #[bms_token("#ELSEIF {}")]
    ElseIf(u64),
    /// `#ELSE` — default branch when no `#IF` / `#ELSEIF` matched.
    #[bms_token("#ELSE")]
    Else,
    /// `#ENDIF` — close the current `#IF` / `#ELSEIF` / `#ELSE` chain.
    ///
    /// Common typos from various engines:
    /// - `#END IF` (IIDXv/HDX, misunderstood spacing)
    /// - `#END` (Angolmois/Sonorous, partial match)
    /// - `#IFEND` (alternative order)
    #[bms_token("#ENDIF")]
    #[bms_token("#END IF")]
    #[bms_token("#END")]
    #[bms_token("#IFEND")]
    EndIf,
    /// `#SWITCH N` — start a switch block with `N` cases.
    ///
    /// The engine picks a value in `[1, N]`; `#CASE k` activates when the
    /// value equals `k`.
    #[bms_token("#SWITCH {}")]
    Switch(u64),
    /// `#SETSWITCH N` — force a specific switch value (analogous to
    /// `#SETRANDOM`).
    #[bms_token("#SETSWITCH {}")]
    SetSwitch(u64),
    /// `#ENDSW` / `#ENDSWITCH` — close the current `#SWITCH` block.
    #[bms_token("#ENDSW")]
    #[bms_token("#ENDSWITCH")]
    EndSwitch,
    /// `#CASE N` — a branch that activates when the switch value equals `N`.
    #[bms_token("#CASE {}")]
    Case(u64),
    /// `#SKIP N` — skip `N` lines (used inside `#SWITCH` blocks to jump
    /// past unwanted cases).
    #[bms_token("#SKIP {}")]
    Skip(u64),
    /// `#DEF` — default branch inside a `#SWITCH` block.
    #[bms_token("#DEF")]
    Def,
}

// From / TryFrom conversions

impl From<BmsHeaderControlFlow> for BmsHeader<'_> {
    #[inline]
    fn from(flow: BmsHeaderControlFlow) -> Self {
        BmsHeader::ControlFlow(flow)
    }
}

impl<'a> TryFrom<BmsHeader<'a>> for BmsHeaderControlFlow {
    type Error = BmsTryFromError<'a>;

    #[inline]
    fn try_from(header: BmsHeader<'a>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::ControlFlow(f) => Ok(f),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}
