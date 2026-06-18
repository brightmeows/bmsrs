//! Mode-family layouts and the channel/lane mapping traits.
//!
//! A **mode family** is a lineage of BMS/BMSON play modes that share a common
//! channel-to-key mapping table (e.g. the `Bme` family covers beat-5k/7k/10k/14k).
//! Each family is expressed as a zero-sized newtype that implements the
//! relevant mapping trait(s).
//!
//! Families do **not** carry a key count: whether a chart is 5K or 7K is a
//! property of which lanes its notes actually use, not of the family. This
//! mirrors the reference design where a single `Beat` layout handles all
//! beat-* variants.
//!
//! # Mapping
//!
//! Each family decodes BMS `(player, lane)` bytes and — where a BMSON
//! `mode_hint` exists — `x` values directly into a `(PlayerSide, Lane)` pair
//! stored on the note. There is no separate flat-lane layer: the position
//! triple `(side, lane, kind)` is the chart's own representation.
//!
//! BMS-only families (`Nanasi`, `PmsBme`, `DscOctFp`) have no BMSON
//! `mode_hint`, so they only implement [`BmsLayout`]. [`GenericLayout`] is
//! BMSON-only and implements [`BmsonLayout`].

use std::num::NonZeroU8;

use crate::mode::{BmsChannel, Lane, PlayerSide};
use crate::note::{NoteData, NoteKind};

/// BMS-side mapping: decodes a [`BmsChannel`] (the post-tokenizer
/// `(player, lane)` pair) into a [`NoteData`] position (side + lane; kind is set
/// to [`NoteKind::Normal`] and overridden by the caller as needed).
///
/// `None` discards the note.
pub trait BmsLayout {
    /// Map a decoded BMS channel to the note's position.
    #[must_use]
    fn map_channel(ch: BmsChannel) -> Option<NoteData>;
}

/// BMSON-side mapping: decodes a sound-channel `x` value into a
/// [`NoteData`] position (side + lane; kind is set to [`NoteKind::Normal`]).
///
/// `None` discards the note. Only families with a BMSON `mode_hint`
/// counterpart implement this.
pub trait BmsonLayout {
    /// Map a BMSON player channel `x` to the note's position.
    #[must_use]
    fn map_x(&self, x: u64) -> Option<NoteData>;
}

/// Construct a `NonZeroU8` from a value known to be ≥ 1 at the call site
/// (e.g. inside a `match` arm that has already excluded 0). Used to build
/// `Lane::Key` / `Lane::Scratch` indices in the decoders below.
const fn nz(n: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(n)
}

/// BME family: covers `beat-5k`, `beat-7k`, `beat-10k`, `beat-14k` (and the
/// `dj-*` aliases). One mapping table serves all of them; the key count of a
/// given chart is whatever its notes happen to use.
///
/// Physical layout (left → right): `KEY1-5 | SC | KEY6-7` per side.
///
/// Per the bmson spec, `beat-10k` charts must leave `x ∈ {6, 7, 14, 15}`
/// empty (those slots do not exist in 5K-per-side mode). This family maps
/// them as `KEY6`/`KEY7` if present, so non-conforming input is preserved
/// rather than discarded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bme;

impl BmsLayout for Bme {
    /// Decode a BMS `(player, lane)` pair into the BME-family position.
    fn map_channel(ch: BmsChannel) -> Option<NoteData> {
        let side = ch.player_side();
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
        Some(NoteData {
            side,
            lane: key,
            kind: NoteKind::Normal,
        })
    }
}

impl Bme {
    /// Decode a BMSON `x` value into the BME-family position.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x bounded by match arms to ≤16"
    )]
    #[must_use]
    pub fn from_bmson(x: u64) -> Option<NoteData> {
        match x {
            1..=7 => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(x as u8)?),
                kind: NoteKind::Normal,
            }),
            8 => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Scratch(nz(1)?),
                kind: NoteKind::Normal,
            }),
            9..=15 => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Key(nz((x - 8) as u8)?),
                kind: NoteKind::Normal,
            }),
            16 => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Scratch(nz(1)?),
                kind: NoteKind::Normal,
            }),
            _ => None,
        }
    }
}

impl BmsonLayout for Bme {
    fn map_x(&self, x: u64) -> Option<NoteData> {
        Self::from_bmson(x)
    }
}

/// Nanasi family: BME keys plus a foot pedal on channel `17`/`27`. Covers the
/// nanasi and Angolmois pedal variants (their channel mapping is identical;
/// the SC-rendering-side difference is a renderer concern, not a mapping one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Nanasi;

impl BmsLayout for Nanasi {
    /// Decode a BMS `(player, lane)` pair into the Nanasi-family position.
    fn map_channel(ch: BmsChannel) -> Option<NoteData> {
        let side = ch.player_side();
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
        Some(NoteData {
            side,
            lane: key,
            kind: NoteKind::Normal,
        })
    }
}

/// Native PMS family: a single-player 9-key layout whose KEY6-9 reuse the 2P
/// channels `22-25`. Covers `popn-9k`, `popn-5k` (subset), and the
/// `pomu-battle` 3K subset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pms;

impl BmsLayout for Pms {
    /// Decode a BMS `(player, lane)` pair into the PMS-family position.
    ///
    /// KEY1-5 come from 1P channels `11-15`; KEY6-9 come from 2P channels
    /// `22-25` (decoded `(2, 2..=5)`). All keys report `Player1` because PMS
    /// is single-player.
    fn map_channel(ch: BmsChannel) -> Option<NoteData> {
        let key = match (ch.player(), ch.lane()) {
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
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: key,
            kind: NoteKind::Normal,
        })
    }
}

impl Pms {
    /// Decode a BMSON `x` value into the PMS-family position (`popn-9k`/`5k`).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x bounded by match arm to ≤9"
    )]
    #[must_use]
    pub fn from_bmson(x: u64) -> Option<NoteData> {
        match x {
            1..=9 => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(x as u8)?),
                kind: NoteKind::Normal,
            }),
            _ => None,
        }
    }
}

impl BmsonLayout for Pms {
    fn map_x(&self, x: u64) -> Option<NoteData> {
        Self::from_bmson(x)
    }
}

/// PMS BME-type family: 9 keys per side using the BME channel shape
/// (`KEY6=18 KEY7=19 KEY8=16 KEY9=17`). Note that channels `16`/`17`, which
/// the BME family reads as scratch/free, here carry `KEY8`/`KEY9` — this is
/// why a separate family exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PmsBme;

impl BmsLayout for PmsBme {
    /// Decode a BMS `(player, lane)` pair into the PMS-BME-type position.
    fn map_channel(ch: BmsChannel) -> Option<NoteData> {
        let side = ch.player_side();
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
        Some(NoteData {
            side,
            lane: key,
            kind: NoteKind::Normal,
        })
    }
}

/// DSC/FPP + OCT/FP family: a dual-side layout with up to two scratches and a
/// foot pedal. DSC/FPP (dual scratch, no pedal) and OCT/FP (13 keys + 2nd
/// scratch + pedal) are both subsets of this family's key set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DscOctFp;

impl BmsLayout for DscOctFp {
    /// Decode a BMS `(player, lane)` pair into the DscOctFp-family position.
    fn map_channel(ch: BmsChannel) -> Option<NoteData> {
        match (ch.player(), ch.lane()) {
            (1, 1) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(1)?),
                kind: NoteKind::Normal,
            }),
            (1, 2) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(2)?),
                kind: NoteKind::Normal,
            }),
            (1, 3) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(3)?),
                kind: NoteKind::Normal,
            }),
            (1, 4) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(4)?),
                kind: NoteKind::Normal,
            }),
            (1, 5) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(5)?),
                kind: NoteKind::Normal,
            }),
            (1, 6) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Scratch(nz(1)?),
                kind: NoteKind::Normal,
            }),
            (1, 8) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(6)?),
                kind: NoteKind::Normal,
            }),
            (1, 9) => Some(NoteData {
                side: PlayerSide::Player1,
                lane: Lane::Key(nz(7)?),
                kind: NoteKind::Normal,
            }),
            (2, 1) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::FootPedal,
                kind: NoteKind::Normal,
            }),
            (2, 2) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Key(nz(1)?),
                kind: NoteKind::Normal,
            }),
            (2, 3) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Key(nz(2)?),
                kind: NoteKind::Normal,
            }),
            (2, 4) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Key(nz(3)?),
                kind: NoteKind::Normal,
            }),
            (2, 5) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Key(nz(4)?),
                kind: NoteKind::Normal,
            }),
            (2, 6) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Scratch(nz(2)?),
                kind: NoteKind::Normal,
            }),
            (2, 8) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Key(nz(5)?),
                kind: NoteKind::Normal,
            }),
            (2, 9) => Some(NoteData {
                side: PlayerSide::Player2,
                lane: Lane::Key(nz(6)?),
                kind: NoteKind::Normal,
            }),
            _ => None,
        }
    }
}

/// Generic n-keys family for BMSON `generic-nkeys`. `keys` is the channel
/// count; channels `1..=keys` map left-to-right to `Lane::Key(1..=keys)`.
///
/// Unlike the other families, `GenericLayout` does not use the `(PlayerSide, Lane)`
/// pivot produced by a `from_bms` decoder: it only implements [`BmsonLayout`]
/// — there is no BMS-side counterpart. Its `keys` is a `u16` and may exceed
/// the `u8` range of [`Lane::Key`]; channels above 255 are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericLayout {
    /// Number of keys (all on player 1).
    pub keys: u16,
}

impl BmsonLayout for GenericLayout {
    fn map_x(&self, x: u64) -> Option<NoteData> {
        if !(1..=u64::from(self.keys)).contains(&x) {
            return None;
        }
        let n = nz(u8::try_from(x).ok()?)?;
        Some(NoteData {
            side: PlayerSide::Player1,
            lane: Lane::Key(n),
            kind: NoteKind::Normal,
        })
    }
}
