//! Audio asset and BGM event types.

use std::path::PathBuf;
use std::time::Duration;

/// An audio asset — a segment of an audio file.
///
/// For BMS charts, each asset is a complete WAV file (`start: Duration::ZERO`,
/// `duration: None`).
///
/// For BMSON charts, the processor pre-computes slices using the
/// sound-channel slicing algorithm, producing assets with specific
/// `start` and `duration` values within the same audio file.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioAsset {
    /// File path relative to the chart file's directory.
    pub path: PathBuf,
    /// Slice start offset from the beginning of the file.
    /// For BMS (no slicing) this is always [`Duration::ZERO`].
    pub start: Duration,
    /// Slice duration.
    /// `None` means play to the end of the file.
    pub duration: Option<Duration>,
}

/// A BGM (background music) event — an audio-only trigger with no
/// gameplay interaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgmEvent {
    /// Tick position.
    pub tick: u64,
    /// Index into [`crate::Chart::audio_assets`].
    pub audio: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_asset_bms_style() {
        let asset = AudioAsset {
            path: PathBuf::from("kick.wav"),
            start: Duration::ZERO,
            duration: None,
        };
        assert_eq!(asset.path, PathBuf::from("kick.wav"));
        assert_eq!(asset.start, Duration::ZERO);
        assert!(asset.duration.is_none());
    }

    #[test]
    fn audio_asset_bmson_slice_style() {
        let asset = AudioAsset {
            path: PathBuf::from("vox.wav"),
            start: Duration::from_millis(250),
            duration: Some(Duration::from_millis(750)),
        };
        assert_eq!(asset.start, Duration::from_millis(250));
        assert_eq!(asset.duration, Some(Duration::from_millis(750)));
    }

    #[test]
    fn bgm_event_fields() {
        let ev = BgmEvent {
            tick: 960,
            audio: 3,
        };
        assert_eq!(ev.tick, 960);
        assert_eq!(ev.audio, 3);
    }
}
