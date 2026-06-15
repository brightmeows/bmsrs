//! Note types and the [`NoteData`] trait.
//!
//! Each note in a [`crate::Chart`] carries a `T: NoteData` that provides
//! the minimum information the Player needs: lane and kind.

use std::fmt::Debug;

/// The kind of a playable note.
#[derive(Clone, Debug, Default, PartialEq)]
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

/// Required note data for Player simulation.
///
/// Every note stored in a [`crate::Chart`] carries a type `T: NoteData`.
/// The Player accesses lane and kind through this trait, allowing
/// format-specific extensions to be carried alongside the common data.
///
/// # Example
///
/// ```
/// use bmsrs_chart::note::{DefaultNoteData, NoteData, NoteKind};
///
/// let data = DefaultNoteData { lane: 3, kind: NoteKind::Normal };
/// assert_eq!(data.lane(), 3);
/// assert_eq!(data.kind(), NoteKind::Normal);
/// ```
pub trait NoteData: Clone + Debug + PartialEq {
    /// Lane index (0-based, left to right within the layout).
    fn lane(&self) -> u16;
    /// Note kind.
    fn kind(&self) -> NoteKind;
}

/// Default [`NoteData`] implementation: just lane + kind.
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultNoteData {
    /// Lane index (0-based).
    pub lane: u16,
    /// Note kind.
    pub kind: NoteKind,
}

impl NoteData for DefaultNoteData {
    fn lane(&self) -> u16 {
        self.lane
    }

    fn kind(&self) -> NoteKind {
        self.kind.clone()
    }
}

/// A note in the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Note<T: NoteData = DefaultNoteData> {
    /// Tick position.
    pub tick: u64,
    /// Audio asset index into [`crate::Chart::audio_assets`], or `None` if silent.
    pub audio: Option<u32>,
    /// Format-specific note data (lane, kind, extensions).
    pub data: T,
}
