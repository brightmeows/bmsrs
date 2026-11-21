//! Audio resource definitions.
//!
//! Corresponds to the tokenizer's [`BmsHeaderResDefAudio`].

use std::collections::BTreeMap;

use bmsrs_bms_values::{BmsBase, BmsHeaderResDefAudio, ExWavParams, WavCmdParams, WavIndex};
use bmsrs_pipeline::SourceFormat;

use crate::SynthParams;

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

/// Audio resource definitions.
///
/// Indexed definitions use `BTreeMap`; scalar fields use `Option`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Audio {
    /// Sound-effect and BGM file definitions (`#WAV`, `#EXWAV`).
    pub wav_files: BTreeMap<WavIndex, String>,
    /// `BMD` synthesis definitions by index (`KaleidBeat` extension).
    pub wav_sources: BTreeMap<WavIndex, SynthParams>,
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
    pub fn apply(&mut self, header: &BmsHeaderResDefAudio, base: BmsBase, format: SourceFormat) {
        match header {
            BmsHeaderResDefAudio::Wav { id, filename } => {
                let normalized = WavIndex::from(id.normalize(base));
                if format == SourceFormat::Bmd
                    && let Some(params) = SynthParams::parse(filename)
                {
                    self.wav_sources.insert(normalized, params);
                } else {
                    self.wav_files.insert(normalized, filename.clone());
                }
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

    /// Return the source for one `#WAV` index.
    ///
    /// File entries and BMD synthesis entries are disjoint; this method hides
    /// the two backing maps from callers.
    #[must_use]
    pub fn wav_source(&self, id: WavIndex) -> Option<WavSource> {
        self.wav_files
            .get(&id)
            .cloned()
            .map(WavSource::File)
            .or_else(|| self.wav_sources.get(&id).cloned().map(WavSource::Synth))
    }
}
