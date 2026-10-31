//! BMSON mode-family layouts: map `x` values to lanes through [`BmsonLayout`].
//!
//! Each mode family is a zero-sized newtype that maps a sound channel's `x`
//! value to a `(NoteSide, Lane)` pair. Stateful decoders (such as
//! [`GenericLayout`] for `generic-nkeys`) are separate structs with an inherent
//! `map_x` method.

use std::num::NonZeroU8;

use bmsrs_chart::mode::{Lane, NoteSide};

/// BMSON side mapping: decode a sound channel's `x` value into a
/// `(NoteSide, Lane)` pair.
///
/// Returning `None` drops the note. Only stateless mode families implement this
/// trait; stateful decoders (such as [`GenericLayout`]) have their own `map_x`
/// method and are used through
/// [`crate::BmsonProcessor::process_nkeys`].
pub trait BmsonLayout {
    /// Map a BMSON player channel's `x` to a note position.
    #[must_use]
    fn map_x(x: u64) -> Option<(NoteSide, Lane)>;
}

/// Construct a `NonZeroU8` from a value known at the call site to be at least 1.
const fn nz(n: u8) -> Option<NonZeroU8> {
    NonZeroU8::new(n)
}

/// Beat mode family (BMSON): covers `beat-5k`, `beat-7k`, `beat-10k`, and
/// `beat-14k` (plus the `dj-*` aliases).
///
/// Physical layout (left to right): `KEY1-5 | SC | KEY6-7` per side.
///
/// Per the bmson specification, `beat-10k` charts must leave `x` in
/// `{6, 7, 14, 15}` empty (those slots do not exist in a 5K-per-side mode). This
/// family maps them to `KEY6`/`KEY7` when present, preserving rather than
/// dropping non-conforming input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Beat;

impl Beat {
    /// Decode a BMSON `x` value into a position in the beat mode family.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x bounded by match arms to ≤16"
    )]
    #[must_use]
    pub fn from_bmson(x: u64) -> Option<(NoteSide, Lane)> {
        match x {
            1..=7 => Some((NoteSide::P1, Lane::Key(nz(x as u8)?))),
            8 => Some((NoteSide::P1, Lane::Scratch(nz(1)?))),
            9..=15 => Some((NoteSide::P2, Lane::Key(nz((x - 8) as u8)?))),
            16 => Some((NoteSide::P2, Lane::Scratch(nz(1)?))),
            _ => None,
        }
    }
}

impl BmsonLayout for Beat {
    fn map_x(x: u64) -> Option<(NoteSide, Lane)> {
        Self::from_bmson(x)
    }
}

/// PMS mode family (BMSON): single-player 9-key layout covering `popn-9k` and
/// `popn-5k` (a subset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pms;

impl Pms {
    /// Decode a BMSON `x` value into a position in the PMS mode family
    /// (`popn-9k`/`5k`).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "x bounded by match arm to ≤9"
    )]
    #[must_use]
    pub fn from_bmson(x: u64) -> Option<(NoteSide, Lane)> {
        match x {
            1..=9 => Some((NoteSide::P1, Lane::Key(nz(x as u8)?))),
            _ => None,
        }
    }
}

impl BmsonLayout for Pms {
    fn map_x(x: u64) -> Option<(NoteSide, Lane)> {
        Self::from_bmson(x)
    }
}

/// Generic n-keys mode family for BMSON `generic-nkeys`. `keys` is the number of
/// channels; channels `1..=keys` map to `Lane::Key(1..=keys)` from left to right.
///
/// This family carries runtime state (the key count), so it does **not**
/// implement the stateless [`BmsonLayout`]. Use it through
/// [`crate::BmsonProcessor::process_nkeys`].
///
/// `keys` is a `u8`: [`Lane::Key`] is bounded by `NonZeroU8`, so key counts
/// above 255 cannot be mapped correctly and are rejected by the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericLayout {
    /// Number of keys (all on player 1, at least 1).
    pub keys: u8,
}

impl GenericLayout {
    /// Map a BMSON `x` value to a note position for the current key count.
    #[must_use]
    pub fn map_x(&self, x: u64) -> Option<(NoteSide, Lane)> {
        let n = u8::try_from(x).ok()?;
        if n == 0 || n > self.keys {
            return None;
        }
        Some((NoteSide::P1, Lane::Key(nz(n)?)))
    }
}
