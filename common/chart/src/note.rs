//! Note kind definitions.
//!
//! Note position (`NoteSide`, `Lane`) is carried directly in
//! [`Event::Note`](crate::Event::Note).  Per-note format extensions use the
//! [`NoteExt`](crate::NoteExt) trait.

use std::fmt::Debug;

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
