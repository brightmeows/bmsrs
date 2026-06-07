//! `#wav` and related audio resource definitions.

/// Audio resource definition headers.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeaderResDefAudio<'a> {
    /// `#WAVxx` — a sound effect definition.
    Wav {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#EXWAVxx`
    ExWav {
        /// The 2-character index.
        #[serde(borrow)]
        index: &'a str,
        /// Path or name of the resource file.
        #[serde(borrow)]
        filename: &'a str,
    },
    /// `#WAVCMD`
    #[serde(borrow)]
    WavCmd(&'a str),
    /// `#CDDA`
    #[serde(borrow)]
    Cdda(&'a str),
    /// `#MIDIFILE`
    #[serde(borrow)]
    Midifile(&'a str),
    /// `#PATH_WAV`
    #[serde(borrow)]
    PathWav(&'a str),
}
