//! Note kind definitions.
//!
//! Note positions (`NoteSide`, `Lane`) are carried directly in
//! [`EventKind::Note`](crate::EventKind::Note). Per-note format extensions use
//! the [`NoteExt`](crate::NoteExt) trait.

use std::fmt::Debug;

use crate::audio::AudioIndex;

/// Mine damage value.
///
/// Guaranteed to contain no NaN, so it supports [`Eq`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Damage(f64);

impl Damage {
    /// Create a damage value from `f64`.
    ///
    /// # Panics (debug builds only)
    ///
    /// Panics in debug builds when `value` is NaN.
    #[must_use]
    pub fn new(value: f64) -> Self {
        debug_assert!(!value.is_nan(), "damage must not be NaN");
        Self(value)
    }

    /// Return the raw damage value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for Damage {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Damage {}

/// The set of audio layers for a note.
///
/// Most notes have one audio layer; BMSON allows different `SoundChannel`s to
/// merge into one note at the same position, in which case the note carries
/// multiple audio layers. The internal three-state representation avoids extra
/// allocation for a single audio layer; `Many` always contains at least two
/// elements.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoteAudio(NoteAudioInner);

/// Internal three-state representation of [`NoteAudio`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum NoteAudioInner {
    #[default]
    /// No audio layers.
    None,
    /// One audio layer.
    One(AudioIndex),
    /// Multiple audio layers, at least two elements.
    Many(Box<[AudioIndex]>),
}

impl NoteAudio {
    /// A note with no audio layers.
    #[must_use]
    pub const fn none() -> Self {
        Self(NoteAudioInner::None)
    }

    /// A note with exactly one audio layer.
    #[must_use]
    pub const fn one(index: AudioIndex) -> Self {
        Self(NoteAudioInner::One(index))
    }

    /// Construct from any collection of audio indices; zero layers gives
    /// [`none`](Self::none), one gives [`one`](Self::one), and two or more fold
    /// into `Many`.
    #[must_use]
    pub fn from_indices(indices: impl IntoIterator<Item = AudioIndex>) -> Self {
        let collected: Vec<AudioIndex> = indices.into_iter().collect();
        match collected.as_slice() {
            [] => Self::none(),
            [index] => Self::one(*index),
            _ => Self(NoteAudioInner::Many(collected.into_boxed_slice())),
        }
    }

    /// Return all audio indices as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[AudioIndex] {
        match &self.0 {
            NoteAudioInner::None => &[],
            NoteAudioInner::One(index) => std::slice::from_ref(index),
            NoteAudioInner::Many(indices) => indices,
        }
    }

    /// Iterate over all audio indices.
    pub fn iter(&self) -> impl Iterator<Item = AudioIndex> + '_ {
        self.as_slice().iter().copied()
    }

    /// The first audio layer index; [`None`] when there is no audio.
    #[must_use]
    pub fn primary(&self) -> Option<AudioIndex> {
        self.as_slice().first().copied()
    }

    /// Number of audio layers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Whether there are no audio layers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}

impl From<Option<AudioIndex>> for NoteAudio {
    fn from(value: Option<AudioIndex>) -> Self {
        value.map_or_else(Self::none, Self::one)
    }
}

/// The note's role during gameplay.
///
/// Orthogonal to [`NoteKind`]: a release note (`up: true`) can be either a
/// normal note or a long note. The role belongs to the core model and survives
/// erasing format extensions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoteRole {
    /// Normal judgeable note.
    #[default]
    Normal,
    /// Release note / BSS / MSS endpoint.
    Release,
}

/// Kind of a playable note.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoteKind {
    /// Normal (short) note: a single hit.
    #[default]
    Normal,
    /// Long note held for `duration` ticks starting at this note's tick.
    Long {
        /// Hold duration in ticks.
        duration: u64,
    },
    /// Mine: pressing it drains life.
    Mine {
        /// Damage amount (game-defined units).
        damage: Damage,
    },
    /// Invisible note: triggers audio but is not displayed and is not judged
    /// normally.
    ///
    /// In BMS these are "key" notes (channels `31`-`39`, `41`-`49`). In BMSON
    /// they come from `key_channels`.
    Invisible,
}

/// Chart-level or per-note long-note type hint.
///
/// `Ln` and `Cn` correspond to the BMSON v2 standard values `"ln"`/`"cn"`.
/// `Hcn` is a beatoraja extension and is not part of v2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LnTypeHint {
    /// Long note (LN): judged only on press.
    #[default]
    Ln,
    /// Charge note (CN): judged on both press and release.
    Cn,
    /// Hell charge note (HCN): beatoraja extension, non-standard.
    Hcn,
}

/// Chart-level or per-note long-note judgment hint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LnJudgeHint {
    /// Judge only the note itself.
    #[default]
    Normal,
    /// Judge additional ticks while held.
    Ticks,
}

/// Chart-level or per-note long-note life hint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LnLifeHint {
    /// Only the note itself restores life.
    #[default]
    Normal,
    /// Additional ticks restore life while held.
    Ticks,
}
