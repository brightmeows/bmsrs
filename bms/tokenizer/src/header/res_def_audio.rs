//! `#wav` and related audio resource definitions.

use crate::BmsTokenAttr;
use crate::id::{BmsChannelId, WavTag};

/// Audio resource definition headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefAudio<'a> {
    /// `#WAV{id}` — a sound effect definition.
    #[bms_token("#WAV{id} {filename}")]
    Wav {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        id: BmsChannelId<WavTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#EXWAV{id}`
    #[bms_token("#EXWAV{id} {filename}")]
    ExWav {
        /// The 2-character index.
        id: BmsChannelId<WavTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#WAVCMD`
    #[bms_token("#WAVCMD {value}")]
    WavCmd(&'a str),
    /// `#CDDA`
    #[bms_token("#CDDA {value}")]
    Cdda(&'a str),
    /// `#MIDIFILE`
    #[bms_token("#MIDIFILE {value}")]
    Midifile(&'a str),
    /// `#PATH_WAV`
    #[bms_token("#PATH_WAV {value}")]
    PathWav(&'a str),
}
