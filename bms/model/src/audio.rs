//! Audio resource definitions.
//!
//! Corresponds to the tokenizer's [`BmsHeaderResDefAudio`].

use std::collections::BTreeMap;

use bmsrs_bms_values::{BmsBase, BmsHeaderResDefAudio, ExWavParams, WavCmdParams, WavIndex};

/// Owned parameters for `#EXWAV` (the owned counterpart of the tokenizer's
/// `ExWavParams`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedExWavParams {
    /// Pan.
    pub pan: Option<i32>,
    /// Volume attenuation.
    pub volume: Option<i32>,
    /// Frequency.
    pub frequency: Option<u32>,
    /// File name.
    pub filename: String,
}

impl From<&ExWavParams> for OwnedExWavParams {
    fn from(p: &ExWavParams) -> Self {
        Self {
            pan: p.pan,
            volume: p.volume,
            frequency: p.frequency,
            filename: p.filename.clone(),
        }
    }
}

/// Audio resource definitions.
///
/// Indexed definitions use `BTreeMap`; scalar fields use `Option`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Audio {
    /// Sound-effect and BGM file definitions (`#WAV`, `#EXWAV`).
    pub wav_files: BTreeMap<WavIndex, String>,
    /// `#EXWAV` effect parameters by index (pan, volume, frequency).
    pub ex_wav_params: BTreeMap<WavIndex, OwnedExWavParams>,
    /// Audio playback command (`#WAVCMD`).
    pub wav_cmd: Option<WavCmdParams>,
    /// CD track reference (`#CDDA`).
    pub cdda: Option<String>,
    /// MIDI file reference (`#MIDIFILE`).
    pub midifile: Option<String>,
    /// Directory prefix for audio file lookup (`#PATH_WAV`).
    pub path_wav: Option<String>,
}

impl Audio {
    /// Apply one audio resource header command to this structure.
    ///
    /// Index keys (`WavIndex`) are normalized with `base` for case-insensitive
    /// comparison in standard (Base36) BMS files.
    pub fn apply(&mut self, header: &BmsHeaderResDefAudio, base: BmsBase) {
        match header {
            BmsHeaderResDefAudio::Wav { id, filename } => {
                self.wav_files
                    .insert(WavIndex::from(id.normalize(base)), filename.clone());
            }
            BmsHeaderResDefAudio::ExWav { id, params } => {
                let nid = WavIndex::from(id.normalize(base));
                self.wav_files.insert(nid, params.filename.clone());
                self.ex_wav_params
                    .insert(nid, OwnedExWavParams::from(params));
            }
            BmsHeaderResDefAudio::WavCmd { params } => {
                self.wav_cmd = Some(params.clone());
            }
            BmsHeaderResDefAudio::Cdda(s) => self.cdda = Some(s.clone()),
            BmsHeaderResDefAudio::Midifile(s) => self.midifile = Some(s.clone()),
            BmsHeaderResDefAudio::PathWav(s) => self.path_wav = Some(s.clone()),
        }
    }
}
