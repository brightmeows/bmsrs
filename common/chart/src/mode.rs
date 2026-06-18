//! Public position types identifying a note's key.
//!
//! A note's position is the pair `(PlayerSide, Lane)` — the deepest public
//! types of the chart model. Every note carries this pair alongside its
//! [`NoteKind`](crate::NoteKind); processors produce it via the mode-family
//! layouts, and the player matches notes by it.

use std::num::NonZeroU8;

/// Player side of a note.
///
/// `Player2` is only meaningful for dual-side modes (e.g. beat-14k);
/// single-player families (e.g. PMS) record `Player1` uniformly.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerSide {
    /// First player side.
    Player1,
    /// Second player side.
    Player2,
}

/// Which key a note sits on, independent of player side.
///
/// `Key` and `Scratch` carry a 1-based index (so `Key(1)` is the first
/// regular key of its side), using [`NonZeroU8`] for niche optimisation.
/// The index distinguishes multiple scratches (e.g. DSC/FPP dual scratch)
/// and keys within a side.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// Regular playable key. 1-based index within the side, ordered
    /// left-to-right in the family's physical layout.
    Key(NonZeroU8),
    /// Turntable / scratch. 1-based index (1 for the primary scratch, 2 for
    /// a second scratch in dual-scratch modes).
    Scratch(NonZeroU8),
    /// Foot pedal (nanasi / Angolmois / OCT-FP pedal modes).
    FootPedal,
}
