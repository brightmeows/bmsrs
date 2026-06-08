//! `#wav` and related audio resource definitions.

use crate::id::{BmsChannelId, WavTag};

/// Audio resource definition headers.
#[derive(Debug, Clone, PartialEq)]
pub enum BmsHeaderResDefAudio<'a> {
    /// `#WAVxx` — a sound effect definition.
    Wav {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        index: BmsChannelId<WavTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#EXWAVxx`
    ExWav {
        /// The 2-character index.
        index: BmsChannelId<WavTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#WAVCMD`
    WavCmd(&'a str),
    /// `#CDDA`
    Cdda(&'a str),
    /// `#MIDIFILE`
    Midifile(&'a str),
    /// `#PATH_WAV`
    PathWav(&'a str),
}
