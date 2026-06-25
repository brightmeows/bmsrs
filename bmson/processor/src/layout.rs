//! BMSON mode-family layouts: `x`-value-to-lane mapping using [`BmsonLayout`].
//!
//! Each family is expressed as a zero-sized newtype that maps a sound-channel
//! `x` value to a [`NoteData`] note position.  Stateful decoders (e.g.
//! [`GenericLayout`] for `generic-nkeys`) are standalone structs with an
//! inherent `map_x` method.

use std::num::NonZeroU8;

use bmsrs_chart::mode::{Lane, NoteSide};
use bmsrs_chart::note::{NoteData, NoteKind};

/// BMSON-side mapping: decodes a sound-channel `x` value into a
/// [`NoteData`] position (side + lane; kind is set to [`NoteKind::Normal`]).
///
/// `None` discards the note. Only stateless families implement this trait;
/// stateful decoders (e.g. [`GenericLayout`]) have their own `map_x` method
/// and are used via [`crate::BmsonProcessor::process_nkeys`].
pub trait BmsonLayout {
    /// Map a BMSON player channel `x` to the note's position.
    #[must_use]
    fn map_x(x: u64) -> Option<NoteData>;
}

/// Construct a `NonZeroU8` from a value known to be ≥ 1 at the call site.
const fn nz(n: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(n)
}

/// Beat family (BMSON): covers `beat-5k`, `beat-7k`, `beat-10k`, `beat-14k`
/// (and `dj-*` aliases).
///
/// Physical layout (left → right): `KEY1-5 | SC | KEY6-7` per side.
///
/// Per the bmson spec, `beat-10k` charts must leave `x ∈ {6, 7, 14, 15}`
/// empty (those slots do not exist in 5K-per-side mode). This family maps
/// them as `KEY6`/`KEY7` if present, so non-conforming input is preserved
/// rather than discarded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Beat;

impl Beat {
    /// Decode a BMSON `x` value into the beat-family position.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x bounded by match arms to ≤16"
    )]
    #[must_use]
    pub fn from_bmson(x: u64) -> Option<NoteData> {
        match x {
            1..=7 => Some(NoteData {
                side: NoteSide::ONE,
                lane: Lane::Key(nz(x as u8)?),
                kind: NoteKind::Normal,
            }),
            8 => Some(NoteData {
                side: NoteSide::ONE,
                lane: Lane::Scratch(nz(1)?),
                kind: NoteKind::Normal,
            }),
            9..=15 => Some(NoteData {
                side: NoteSide::TWO,
                lane: Lane::Key(nz((x - 8) as u8)?),
                kind: NoteKind::Normal,
            }),
            16 => Some(NoteData {
                side: NoteSide::TWO,
                lane: Lane::Scratch(nz(1)?),
                kind: NoteKind::Normal,
            }),
            _ => None,
        }
    }
}

impl BmsonLayout for Beat {
    fn map_x(x: u64) -> Option<NoteData> {
        Self::from_bmson(x)
    }
}

/// PMS family (BMSON): a single-player 9-key layout covering `popn-9k` and
/// `popn-5k` (subset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pms;

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
                side: NoteSide::ONE,
                lane: Lane::Key(nz(x as u8)?),
                kind: NoteKind::Normal,
            }),
            _ => None,
        }
    }
}

impl BmsonLayout for Pms {
    fn map_x(x: u64) -> Option<NoteData> {
        Self::from_bmson(x)
    }
}

/// Generic n-keys family for BMSON `generic-nkeys`. `keys` is the channel
/// count; channels `1..=keys` map left-to-right to `Lane::Key(1..=keys)`.
///
/// This family has runtime state (the key count), so it does **not** implement
/// [`BmsonLayout`] — which is stateless.  Use
/// [`crate::BmsonProcessor::process_nkeys`] to invoke it.
///
/// Its `keys` is a `u16` and may exceed the `u8` range of [`Lane::Key`];
/// channels above 255 are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericLayout {
    /// Number of keys (all on player 1).
    pub keys: u16,
}

impl GenericLayout {
    /// Map a BMSON `x` value to a note position for this key count.
    #[must_use]
    pub fn map_x(&self, x: u64) -> Option<NoteData> {
        if !(1..=u64::from(self.keys)).contains(&x) {
            return None;
        }
        let n = nz(u8::try_from(x).ok()?)?;
        Some(NoteData {
            side: NoteSide::ONE,
            lane: Lane::Key(n),
            kind: NoteKind::Normal,
        })
    }
}
