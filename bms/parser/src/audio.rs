//! Audio resource definitions.
//!
//! Corresponds to [`BmsHeaderResDefAudio`] from the tokenizer.

use std::collections::BTreeMap;

use bms_tokenizer::{BmsHeaderResDefAudio, WavIndex};

/// Audio resource definitions.
///
/// Indexed definitions use `BTreeMap`; scalar fields use `Option`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Audio {
    /// Sound effect / BGM file definitions (`#WAV`, `#EXWAV`).
    pub wav_files: BTreeMap<WavIndex, String>,
    /// Audio playback command (`#WAVCMD`).
    pub wav_cmd: Option<String>,
    /// CD audio track reference (`#CDDA`).
    pub cdda: Option<String>,
    /// MIDI file reference (`#MIDIFILE`).
    pub midifile: Option<String>,
    /// Directory prefix for audio file lookup (`#PATH_WAV`).
    pub path_wav: Option<String>,
}

impl Audio {
    /// Apply an audio resource header to this struct.
    pub fn apply<C: AsRef<str>>(&mut self, header: &BmsHeaderResDefAudio<C>) {
        match header {
            BmsHeaderResDefAudio::Wav { id, filename } => {
                self.wav_files.insert(*id, filename.as_ref().to_owned());
            }
            BmsHeaderResDefAudio::ExWav { id, params } => {
                self.wav_files
                    .insert(*id, params.filename.as_ref().to_owned());
            }
            BmsHeaderResDefAudio::WavCmd(s) => self.wav_cmd = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::Cdda(s) => self.cdda = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::Midifile(s) => self.midifile = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::PathWav(s) => self.path_wav = Some(s.as_ref().to_owned()),
        }
    }
}
