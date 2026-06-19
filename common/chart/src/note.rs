//! Note types and the [`NoteDataLike`] trait.
//!
//! Each note in a [`crate::Chart`] carries a `T: NoteDataLike` that provides
//! the position triple `(PlayerSide, Lane)` plus the note kind — the minimum the
//! Player needs to match and judge notes.

use std::fmt::Debug;

use crate::mode::{Lane, PlayerSide};

/// The kind of a playable note.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum NoteKind {
    /// Normal (short) note — tap once.
    #[default]
    Normal,
    /// Long note — hold from the note's tick for `duration` ticks.
    Long {
        /// Hold duration in ticks.
        duration: u64,
    },
    /// Mine / landmine — damages the player if pressed.
    Mine {
        /// Damage amount (game-defined units).
        damage: f64,
    },
    /// Invisible note — triggers audio but is not displayed or judged normally.
    ///
    /// In BMS these are "key" notes (channels `31`–`39`, `41`–`49`).
    /// In BMSON these come from `key_channels`.
    Invisible,
}

/// Trait for types that can serve as note data in a [`Chart`](crate::Chart).
///
/// The built-in [`NoteData`] struct implements this trait. Custom types can
/// carry format-specific extensions (volume, pan, LN mode, etc.) while still
/// providing the minimum position triple required by the Player.
///
/// # Example
///
/// ```
/// use std::num::NonZeroU8;
/// use bmsrs_chart::mode::{Lane, PlayerSide};
/// use bmsrs_chart::note::{NoteData, NoteDataLike, NoteKind};
///
/// let data = NoteData {
///     side: PlayerSide::Player1,
///     lane: Lane::Key(NonZeroU8::new(3).unwrap()),
///     kind: NoteKind::Normal,
/// };
/// assert_eq!(data.side(), PlayerSide::Player1);
/// assert_eq!(data.lane(), Lane::Key(NonZeroU8::new(3).unwrap()));
/// assert_eq!(data.kind(), NoteKind::Normal);
/// ```
pub trait NoteDataLike: Clone + Debug + PartialEq {
    /// Which player side the note belongs to.
    fn side(&self) -> PlayerSide;
    /// Which key the note sits on.
    fn lane(&self) -> Lane;
    /// Note kind.
    fn kind(&self) -> NoteKind;
}

/// Default [`NoteDataLike`] implementation: position triple + kind.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteData {
    /// Player side.
    pub side: PlayerSide,
    /// Key position.
    pub lane: Lane,
    /// Note kind.
    pub kind: NoteKind,
}

impl NoteDataLike for NoteData {
    #[inline]
    fn side(&self) -> PlayerSide {
        self.side
    }

    #[inline]
    fn lane(&self) -> Lane {
        self.lane
    }

    #[inline]
    fn kind(&self) -> NoteKind {
        self.kind
    }
}

/// A note in the chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note<T: NoteDataLike = NoteData> {
    /// Tick position.
    pub tick: u64,
    /// Audio asset index into [`crate::Chart::audio_assets`], or `None` if silent.
    pub audio: Option<u32>,
    /// Format-specific note data (position, kind, extensions).
    pub data: T,
}
