//! `#RANDOM` / `#SWITCH` control-flow commands.
//!
//! These commands let a single BMS file contain multiple chart variations. The
//! tokenizer preserves every branch verbatim; choosing which branch to keep is
//! the responsibility of later pipeline stages (parser and processor).

use crate::BmsTokenAttr;

/// Control-flow headers used for random chart branches.
///
/// BMS supports two branch constructs:
///
/// - **`#RANDOM` blocks**: `#RANDOM N`, then `#IF k` ... `#ENDIF` repeated N
///   times, then `#ENDRANDOM`. At parse time an integer is chosen from
///   `[1, N]`; only the matching `#IF` branch is kept.
/// - **`#SWITCH` blocks**: `#SWITCH N`, then `#CASE k` ... `#DEF` ...
///   `#ENDSW`. Similar to `#RANDOM`, but `#CASE` matches an integer value and
///   `#DEF` provides the default fallback.
///
/// `#SETRANDOM` / `#SETSWITCH` force a specific branch (for tools and tests).
/// `#RONDAM` is a commonly accepted alias for the `#RANDOM` misspelling.
///
/// Nesting and engine compatibility are intricate; see the BMS command memo for
/// the full details.
#[derive(Debug, Clone, PartialEq, Eq, BmsTokenAttr)]
pub enum BmsHeaderControlFlow {
    /// `#RANDOM N` (or `#RONDAM`): starts a random branch block.
    ///
    /// `N` is the number of branches; the engine picks a value in `[1, N]`.
    /// `#RONDAM` is a historical misspelling recognized by some players.
    #[bms_token("#RANDOM {}")]
    #[bms_token("#RONDAM {}")]
    Random(u64),
    /// `#SETRANDOM N`: forces a specific random value instead of rolling.
    ///
    /// Used by tools (such as previews and IR playback) to select a branch
    /// deterministically.
    #[bms_token("#SETRANDOM {}")]
    SetRandom(u64),
    /// `#ENDRANDOM`: closes the current `#RANDOM` block.
    ///
    /// Recommended when nesting `#RANDOM` blocks, because some players (nanasi)
    /// require it to work correctly.
    #[bms_token("#ENDRANDOM")]
    EndRandom,
    /// `#IF N`: branch activated when the random value equals `N`.
    #[bms_token("#IF {}")]
    If(u64),
    /// `#ELSEIF N`: alternative branch (like `else if`).
    #[bms_token("#ELSEIF {}")]
    ElseIf(u64),
    /// `#ELSE`: default branch when no `#IF` / `#ELSEIF` matches.
    #[bms_token("#ELSE")]
    Else,
    /// `#ENDIF`: closes the current `#IF` / `#ELSEIF` / `#ELSE` chain.
    ///
    /// Common misspellings from various engines:
    /// - `#END IF` (IIDXv/HDX, a misread space)
    /// - `#END` (Angolmois/Sonorous, partial match)
    /// - `#IFEND` (the other order)
    #[bms_token("#ENDIF")]
    #[bms_token("#END IF")]
    #[bms_token("#END")]
    #[bms_token("#IFEND")]
    EndIf,
    /// `#SWITCH N`: starts a switch block with `N` cases.
    ///
    /// The engine picks a value in `[1, N]`; `#CASE k` activates when the value
    /// equals `k`.
    #[bms_token("#SWITCH {}")]
    Switch(u64),
    /// `#SETSWITCH N`: forces a specific switch value (like `#SETRANDOM`).
    #[bms_token("#SETSWITCH {}")]
    SetSwitch(u64),
    /// `#ENDSW` / `#ENDSWITCH`: closes the current `#SWITCH` block.
    #[bms_token("#ENDSW")]
    #[bms_token("#ENDSWITCH")]
    EndSwitch,
    /// `#CASE N`: branch activated when the switch value equals `N`.
    #[bms_token("#CASE {}")]
    Case(u64),
    /// `#SKIP`: fall-through jump marker inside a `#SWITCH` block (no
    /// argument).
    ///
    /// When `#SKIP` appears between the matching `#CASE` and the next
    /// `#CASE`/`#DEF`, parsing jumps from `#SKIP` to `#ENDSW` without executing
    /// later branches.
    #[bms_token("#SKIP")]
    Skip,
    /// `#DEF`: default branch inside a `#SWITCH` block.
    #[bms_token("#DEF")]
    Def,
}
