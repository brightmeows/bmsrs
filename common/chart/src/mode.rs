//! Public position types identifying a note's key.
//!
//! A note's position is the pair `(NoteSide, Lane)` — the deepest public
//! types of the chart model. Every note carries this pair alongside its
//! [`NoteKind`](crate::NoteKind).

use std::num::NonZeroU8;

/// Which side of the playfield a note belongs to, as a 1-based index.
///
/// Stored as a [`NonZeroU8`] so that side numbering is open-ended: current
/// formats use only sides 1 and 2 (dual-player BMS/BMSON), but future formats
/// or battle modes may introduce additional sides without changing this type.
///
/// Use [`NoteSide::new`] for arbitrary indices, or the [`NoteSide::P1`] /
/// [`NoteSide::P2`] constants for the common dual-player case.
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

    /// Construct a side from a non-zero index. Any positive value is valid,
    /// so the type is open to future formats with more than two sides.
    #[must_use]
    pub const fn new(value: NonZeroU8) -> Self {
        Self(value)
    }

    /// The 1-based side index.
    #[must_use]
    pub const fn get(self) -> NonZeroU8 {
        self.0
    }

    /// The side index as a plain `u8`, for table lookups.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0.get()
    }
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
