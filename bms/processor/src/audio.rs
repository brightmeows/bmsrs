//! Audio sidecar and WAV asset table construction.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bmsrs_bms_model::{Audio, OwnedExWavParams};
use bmsrs_bms_values::{WavCmdParams, WavIndex};
use bmsrs_chart::{AudioAsset, AudioIndex};

use super::BmsConverter;

/// Sidecar for BMS audio parameters.
///
/// `#PATH_WAV` has already been applied to
/// [`AudioAsset::path`](bmsrs_chart::AudioAsset::path); this sidecar keeps the
/// original prefix and the other parameters that do not enter the core model.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "sidecar keeps raw BMS parameters; some fields cannot safely implement Eq"
)]
#[derive(Debug, Clone, PartialEq)]
pub struct BmsAudio {
    /// Per-WAV effect parameters from `#EXWAV`.
    pub ex_wav_params: BTreeMap<WavIndex, OwnedExWavParams>,
    /// `#WAVCMD` global playback command.
    pub wav_cmd: Option<WavCmdParams>,
    /// `#CDDA` track reference.
    pub cdda: Option<String>,
    /// `#MIDIFILE` reference.
    pub midifile: Option<String>,
    /// Original `#PATH_WAV` directory prefix.
    pub path_wav: Option<String>,
}

impl From<&Audio> for BmsAudio {
    fn from(audio: &Audio) -> Self {
        Self {
            ex_wav_params: audio.ex_wav_params.clone(),
            wav_cmd: audio.wav_cmd.clone(),
            cdda: audio.cdda.clone(),
            midifile: audio.midifile.clone(),
            path_wav: audio.path_wav.clone(),
        }
    }
}

impl BmsConverter<'_> {
    /// Build WAV audio assets and return the lookup table and asset vector.
    pub fn build_audio_assets(
        bms_audio: &Audio,
    ) -> (BTreeMap<WavIndex, AudioIndex>, Vec<AudioAsset>) {
        let mut wav_map = BTreeMap::new();
        let mut audio_assets = Vec::new();
        for (&wav_id, path) in &bms_audio.wav_files {
            #[expect(clippy::cast_possible_truncation, reason = "WAV count fits in u32")]
            let idx = AudioIndex::new(audio_assets.len() as u32);
            let resolved_path = bms_audio.path_wav.as_ref().map_or_else(
                || PathBuf::from(path),
                |prefix| Path::new(prefix).join(path),
            );
            audio_assets.push(AudioAsset {
                path: Arc::from(resolved_path),
                start: Duration::ZERO,
                duration: None,
            });
            wav_map.insert(wav_id, idx);
        }
        (wav_map, audio_assets)
    }
}
