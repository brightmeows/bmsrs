//! Per-note extension data for osu!mania.

use std::num::NonZeroU32;

use bmsrs_chart::NoteExt;

/// An osu! hit sample name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OsuHitSampleName {
    /// The default normal sound.
    Normal,
    /// The default whistle sound.
    Whistle,
    /// The default finish sound.
    Finish,
    /// The default clap sound.
    Clap,
    /// A custom filename in the beatmap directory.
    File(String),
}

/// An osu! sample bank.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OsuSampleBank {
    /// No bank specified.
    #[default]
    None,
    /// The normal bank.
    Normal,
    /// The soft bank.
    Soft,
    /// The drum bank.
    Drum,
}

/// A single hit sample carried by an osu! note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OsuHitSample {
    /// The sample name; custom files use [`OsuHitSampleName::File`].
    pub name: OsuHitSampleName,
    /// The sample bank.
    pub bank: OsuSampleBank,
    /// The custom sample suffix; `None` means no suffix.
    pub suffix: Option<NonZeroU32>,
    /// The volume (0 to 100).
    pub volume: u8,
    /// The custom sample bank number.
    pub custom_sample_bank: u16,
    /// Whether the bank was specified explicitly.
    pub bank_specified: bool,
    /// Whether the sample layers over the normal sound.
    pub is_layered: bool,
}

/// Per-note extension data for osu!mania.
///
/// Currently this only carries hit samples; column positions are expressed by
/// [`NoteSide`](bmsrs_chart::NoteSide) and [`Lane`](bmsrs_chart::Lane), and
/// long-note durations by [`NoteKind::Long`](bmsrs_chart::NoteKind::Long).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OsuNoteExt {
    /// All hit samples triggered by this note.
    pub hit_samples: Vec<OsuHitSample>,
}

impl NoteExt for OsuNoteExt {}
