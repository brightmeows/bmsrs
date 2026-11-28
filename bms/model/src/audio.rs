//! Audio resource definitions.
//!
//! Corresponds to the tokenizer's [`BmsHeaderResDefAudio`].

use std::collections::BTreeMap;

use bmsrs_bms_values::{BmsBase, BmsHeaderResDefAudio, ExWavParams, WavCmdParams, WavIndex};
use bmsrs_pipeline::SourceFormat;

use crate::SynthParams;

/// One `#WAV` source.
///
/// `File` is the standard BMS file reference. `Synth` is a `KaleidBeat`
/// extension used by BMD charts and is not part of the BMS specification or
/// beatoraja.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WavSource {
    /// Ordinary audio file name.
    File(String),
    /// BMD synthesis parameters.
    Synth(SynthParams),
}

impl WavSource {
    /// Return the file name when this is a file source.
    #[must_use]
    pub fn file(&self) -> Option<&str> {
        match self {
            Self::File(path) => Some(path),
            Self::Synth(_) => None,
        }
    }
}

/// Audio resource definitions.
///
/// Indexed definitions use `BTreeMap`; scalar fields use `Option`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Audio {
    /// `#WAV` definitions: file names or BMD synthesis parameters.
    ///
    /// `BMD` synthesis entries are a `KaleidBeat` extension; ordinary charts
    /// only contain [`WavSource::File`].
    pub wav_sources: BTreeMap<WavIndex, WavSource>,
    /// `#EXWAV` effect parameters by index (pan, volume, frequency).
    pub ex_wav_params: BTreeMap<WavIndex, ExWavParams>,
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
    pub fn apply(&mut self, header: &BmsHeaderResDefAudio, base: BmsBase, format: SourceFormat) {
        match header {
            BmsHeaderResDefAudio::Wav { id, filename } => {
                let normalized = WavIndex::from(id.normalize(base));
                if format == SourceFormat::Bmd
                    && let Some(params) = SynthParams::parse(filename)
                {
                    self.wav_sources
                        .insert(normalized, WavSource::Synth(params));
                } else {
                    self.wav_sources
                        .insert(normalized, WavSource::File(filename.clone()));
                }
            }
            BmsHeaderResDefAudio::ExWav { id, params } => {
                let nid = WavIndex::from(id.normalize(base));
                self.wav_sources
                    .insert(nid, WavSource::File(params.filename.clone()));
                self.ex_wav_params.insert(nid, params.clone());
            }
            BmsHeaderResDefAudio::WavCmd { params } => {
                self.wav_cmd = Some(params.clone());
            }
            BmsHeaderResDefAudio::Cdda(s) => self.cdda = Some(s.clone()),
            BmsHeaderResDefAudio::Midifile(s) => self.midifile = Some(s.clone()),
            BmsHeaderResDefAudio::PathWav(s) => self.path_wav = Some(s.clone()),
        }
    }

    /// Return the source for one `#WAV` index.
    #[must_use]
    pub fn wav_source(&self, id: WavIndex) -> Option<&WavSource> {
        self.wav_sources.get(&id)
    }
}
