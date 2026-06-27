//! Audio asset type.
//!
//! BGM events are now part of the unified [`Event`](crate::Event) enum —
//! see [`crate::Event::Bgm`].

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
#[derive(Clone, Debug, PartialEq, Eq)]
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
