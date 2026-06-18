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

/// A decoded BMS channel identifier: the `(player, lane)` pair extracted
/// from a raw BMS channel byte (e.g. `"19"` → `{ player: 1, lane: 9 }`).
///
/// Construction validates `player ∈ {1, 2}` and `lane ∈ {1..=9}`, so
/// downstream code can safely call [`player_side`](Self::player_side) and
/// [`lane`](Self::lane) without extra checks.  The lane value is the decoded
/// number (1–9), **not** the raw channel byte.
///
/// # Zero-cost
///
/// `BmsChannel` is a `Copy` 2-byte struct — passes through registers on
/// all modern architectures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BmsChannel {
    /// Player number (1 or 2).
    player: u8,
    /// Lane / key number (1–9).
    lane: u8,
}

impl BmsChannel {
    /// Try to construct a `BmsChannel` from a decoded `(player, lane)` pair.
    ///
    /// Returns `None` when `player` is not 1 or 2, or `lane` is not 1–9.
    #[must_use]
    pub const fn new(player: u8, lane: u8) -> Option<Self> {
        if matches!((player, lane), (1 | 2, 1..=9)) {
            Some(Self { player, lane })
        } else {
            None
        }
    }

    /// The player number (1 or 2).
    #[must_use]
    pub const fn player(self) -> u8 {
        self.player
    }

    /// The lane / key number (1–9).
    #[must_use]
    pub const fn lane(self) -> u8 {
        self.lane
    }

    /// Convert the validated player number to [`PlayerSide`].
    ///
    /// # Panics
    ///
    /// Never panics in practice — `new()` guarantees `player ∈ {1, 2}`.
    #[must_use]
    pub fn player_side(self) -> PlayerSide {
        match self.player {
            1 => PlayerSide::Player1,
            2 => PlayerSide::Player2,
            _ => panic!(
                "BmsChannel::player_side: player must be 1 or 2, got {}",
                self.player
            ),
        }
    }
}
