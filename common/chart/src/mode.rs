//! Public position types identifying note keys.
//!
//! A note position is a `(NoteSide, Lane)` pair, the lowest-level public type in
//! the chart model. Every note carries this pair along with its
//! [`NoteKind`](crate::NoteKind).

use std::num::NonZeroU8;

/// The playfield side a note belongs to, represented by a one-based index.
///
/// Stored as [`NonZeroU8`] so numbering can grow: current formats use only
/// sides 1 and 2 (two-player BMS/BMSON), but future formats or battle modes can
/// introduce more sides without changing this type.
///
/// Use [`NoteSide::new`] for arbitrary indices; for the common two-player case
/// use the [`NoteSide::P1`] / [`NoteSide::P2`] constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoteSide(NonZeroU8);

impl NoteSide {
    /// Side 1 (1P).
    pub const P1: Self = Self(NonZeroU8::MIN);

    /// Side 2 (2P).
    pub const P2: Self = match NonZeroU8::new(2) {
        Some(n) => Self(n),
        None => panic!("2 is non-zero"),
    };

    /// Construct a side from a non-zero index. Any positive value is valid, so
    /// the type remains open to formats with more than two sides.
    #[must_use]
    pub const fn new(value: NonZeroU8) -> Self {
        Self(value)
    }

    /// One-based side index.
    #[must_use]
    pub const fn get(self) -> NonZeroU8 {
        self.0
    }

    /// Return the side index as a plain `u8` for table lookups.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0.get()
    }
}

/// The key a note is on, independent of the player side.
///
/// `Key` and `Scratch` carry a one-based index (so `Key(1)` is the first regular
/// key on its side) and use [`NonZeroU8`] for niche optimization. The index can
/// distinguish multiple scratches (such as DSC/FPP dual scratch) and multiple
/// keys within the same side.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// Regular playable key. One-based index within its side, ordered left to
    /// right according to the mode family's physical layout.
    Key(NonZeroU8),
    /// Scratch / turntable. One-based index (1 is the main scratch; 2 is the
    /// second scratch in dual-scratch modes).
    Scratch(NonZeroU8),
    /// Foot pedal (nanasi / Angolmois / OCT-FP pedal modes).
    FootPedal,
}
