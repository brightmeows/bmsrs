//! BMS mode-family layouts: channel-to-lane mapping using [`BmsLayout`].
//!
//! Each family is expressed as a zero-sized newtype that maps a decoded
//! [`BmsChannel`] `(player, lane)` pair to a `(NoteSide, Lane)` position.
//!
//! Families do **not** carry a key count: whether a chart is 5K or 7K is a
//! property of which lanes its notes actually use, not of the family.

use std::num::NonZeroU8;

use bmsrs_chart::mode::{Lane, NoteSide};

/// A decoded BMS channel identifier: the `(player, lane)` pair extracted
/// from a raw BMS channel byte (e.g. `"19"` → `{ player: 1, lane: 9 }`).
///
/// Construction validates `player ∈ {1, 2}` and `lane ∈ {1..=9}`, so
/// downstream code can safely call [`note_side`](Self::note_side) and
/// [`lane`](Self::lane) without extra checks.  The lane value is the decoded
/// number (1–9), **not** the raw channel byte.
/// A validated BMS `(player, lane)` channel pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BmsChannel {
    /// Player number (1 or 2).
    player: u8,
    /// Lane number (1–9).
    lane: u8,
}

impl BmsChannel {
    /// Create a new `BmsChannel` from a decoded `(player, lane)`.
    ///
    /// Returns `None` if `player` is not 1 or 2, or if `lane` is not 1–9.
    #[must_use]
    pub const fn new(player: u8, lane: u8) -> Option<Self> {
        if (player == 1 || player == 2) && lane >= 1 && lane <= 9 {
            Some(Self { player, lane })
        } else {
            None
        }
    }

    /// Player side (1 or 2).
    #[must_use]
    pub const fn player(self) -> u8 {
        self.player
    }

    /// Lane number (1–9).
    #[must_use]
    pub const fn lane(self) -> u8 {
        self.lane
    }

    /// Return the player side as a [`NoteSide`].
    #[must_use]
    pub const fn note_side(self) -> NoteSide {
        match self.player {
            1 => NoteSide::P1,
            _ => NoteSide::P2,
        }
    }
}

/// BMS-side mapping: decodes a [`BmsChannel`] into a `(NoteSide, Lane)`
/// position pair, or `None` if the channel should be discarded.
///
/// Note kind (normal/LN/mine/invisible) is set by the processor after this
/// mapping.
pub trait BmsLayout {
    /// Map a decoded BMS channel to the note's position.
    #[must_use]
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)>;
}

/// Construct a `NonZeroU8` from a value known to be ≥ 1 at the call site.
const fn nz(n: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(n)
}

/// BME family: covers `beat-5k`, `beat-7k`, `beat-10k`, `beat-14k` (and the
/// `dj-*` aliases). One mapping table serves all of them; the key count of a
/// given chart is whatever its notes happen to use.
///
/// Physical layout (left → right): `KEY1-5 | SC | KEY6-7` per side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bme;

impl BmsLayout for Bme {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Scratch(nz(1)?),
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// Nanasi family: BME keys plus a foot pedal on channel `17`/`27`. Covers the
/// nanasi and Angolmois pedal variants (their channel mapping is identical;
/// the SC-rendering-side difference is a renderer concern, not a mapping one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Nanasi;

impl BmsLayout for Nanasi {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Scratch(nz(1)?),
            7 => Lane::FootPedal,
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// Native PMS family: a single-player 9-key layout whose KEY6-9 reuse the 2P
/// channels `22-25`. Covers `popn-9k`, `popn-5k` (subset), and the
/// `pomu-battle` 3K subset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pms;

impl BmsLayout for Pms {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let key = match (ch.note_side().as_u8(), ch.lane()) {
            (1, 1) => Lane::Key(nz(1)?),
            (1, 2) => Lane::Key(nz(2)?),
            (1, 3) => Lane::Key(nz(3)?),
            (1, 4) => Lane::Key(nz(4)?),
            (1, 5) => Lane::Key(nz(5)?),
            (2, 2) => Lane::Key(nz(6)?),
            (2, 3) => Lane::Key(nz(7)?),
            (2, 4) => Lane::Key(nz(8)?),
            (2, 5) => Lane::Key(nz(9)?),
            _ => return None,
        };
        Some((NoteSide::P1, key))
    }
}

/// PMS BME-type family: 9 keys per side using the BME channel shape
/// (`KEY6=18 KEY7=19 KEY8=16 KEY9=17`). Note that channels `16`/`17`, which
/// the BME family reads as scratch/free, here carry `KEY8`/`KEY9` — this is
/// why a separate family exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PmsBme;

impl BmsLayout for PmsBme {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        let side = ch.note_side();
        let key = match ch.lane() {
            1 => Lane::Key(nz(1)?),
            2 => Lane::Key(nz(2)?),
            3 => Lane::Key(nz(3)?),
            4 => Lane::Key(nz(4)?),
            5 => Lane::Key(nz(5)?),
            6 => Lane::Key(nz(8)?),
            7 => Lane::Key(nz(9)?),
            8 => Lane::Key(nz(6)?),
            9 => Lane::Key(nz(7)?),
            _ => return None,
        };
        Some((side, key))
    }
}

/// DSC/FPP + OCT/FP family: a dual-side layout with up to two scratches and a
/// foot pedal. DSC/FPP (dual scratch, no pedal) and OCT/FP (13 keys + 2nd
/// scratch + pedal) are both subsets of this family's key set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DscOctFp;

impl BmsLayout for DscOctFp {
    fn map_channel(ch: BmsChannel) -> Option<(NoteSide, Lane)> {
        Some(match (ch.note_side().as_u8(), ch.lane()) {
            (1, 1) => (NoteSide::P1, Lane::Key(nz(1)?)),
            (1, 2) => (NoteSide::P1, Lane::Key(nz(2)?)),
            (1, 3) => (NoteSide::P1, Lane::Key(nz(3)?)),
            (1, 4) => (NoteSide::P1, Lane::Key(nz(4)?)),
            (1, 5) => (NoteSide::P1, Lane::Key(nz(5)?)),
            (1, 6) => (NoteSide::P1, Lane::Scratch(nz(1)?)),
            (1, 8) => (NoteSide::P1, Lane::Key(nz(6)?)),
            (1, 9) => (NoteSide::P1, Lane::Key(nz(7)?)),
            (2, 1) => (NoteSide::P2, Lane::FootPedal),
            (2, 2) => (NoteSide::P2, Lane::Key(nz(1)?)),
            (2, 3) => (NoteSide::P2, Lane::Key(nz(2)?)),
            (2, 4) => (NoteSide::P2, Lane::Key(nz(3)?)),
            (2, 5) => (NoteSide::P2, Lane::Key(nz(4)?)),
            (2, 6) => (NoteSide::P2, Lane::Scratch(nz(2)?)),
            (2, 8) => (NoteSide::P2, Lane::Key(nz(5)?)),
            (2, 9) => (NoteSide::P2, Lane::Key(nz(6)?)),
            _ => return None,
        })
    }
}
