//! Audio resource definitions.
//!
//! Corresponds to [`BmsHeaderResDefAudio`] from the tokenizer.

use std::collections::BTreeMap;

use bms_tokenizer::{BmsHeaderResDefAudio, WavIndex};

/// Extended audio effect parameters for `#EXWAV`.
///
/// Each flag character (`p`/`v`/`f`) has a corresponding numeric value:
/// - **pan** (`p`): `-10000` to `10000`, default `0`.
/// - **volume** (`v`): `-10000` to `0`, default `0` (original).
/// - **frequency** (`f`): `100` to `100000` Hz.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExWavParams {
    /// Flag characters (e.g. `"pvf"`).
    pub flags: String,
    /// Numeric values, one per flag character in order.
    pub values: Vec<f64>,
}

/// Audio resource definitions.
///
/// Indexed definitions use `BTreeMap`; scalar fields use `Option`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Audio {
    /// Sound effect / BGM file definitions (`#WAV`, `#EXWAV`).
    pub wav_files: BTreeMap<WavIndex, String>,
    /// Per-index `#EXWAV` effect parameters (pvf/pan/vol/freq).
    pub ex_wav_params: BTreeMap<WavIndex, ExWavParams>,
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
                self.ex_wav_params.insert(
                    *id,
                    ExWavParams {
                        flags: params.flags.as_ref().to_owned(),
                        values: params.values.clone(),
                    },
                );
            }
            BmsHeaderResDefAudio::WavCmd(s) => self.wav_cmd = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::Cdda(s) => self.cdda = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::Midifile(s) => self.midifile = Some(s.as_ref().to_owned()),
            BmsHeaderResDefAudio::PathWav(s) => self.path_wav = Some(s.as_ref().to_owned()),
        }
    }
}
