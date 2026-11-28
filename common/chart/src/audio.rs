//! Audio asset types.
//!
//! BGM events now live in the unified [`Event`](crate::Event) enum; see
//! [`crate::EventKind::Bgm`].

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

crate::index_newtype! {
    /// Index into [`ChartData::audio_assets`](crate::ChartData::audio_assets).
    ///
    /// The dedicated newtype prevents accidental assignment between audio
    /// indices and raw `u32` values such as BGA resource indices.
    pub struct AudioIndex;
}

/// Audio asset: a segment of one audio file.
///
/// For BMS charts, each asset is an entire WAV file (`start: Duration::ZERO`,
/// `duration: None`).
///
/// For BMSON charts, the processor precomputes slices with the sound-channel
/// slicing algorithm, producing assets with specific `start` and `duration`
/// values within the same audio file.
///
/// Paths are stored as [`Arc<Path>`] so multiple slices of the same file can
/// share the path without duplicate heap allocations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioAsset {
    /// File path relative to the directory containing the chart file.
    pub path: Arc<Path>,
    /// Slice start offset from the beginning of the file. For BMS (no slicing)
    /// this is always [`Duration::ZERO`].
    pub start: Duration,
    /// Slice duration. `None` means play to the end of the file.
    pub duration: Option<Duration>,
}

impl AudioAsset {
    /// Create an `AudioAsset` from a `PathBuf`, automatically wrapping it as
    /// `Arc<Path>`.
    ///
    /// Convenience constructor for the BMS processor, where each asset
    /// corresponds to its own file.
    #[must_use]
    pub fn from_path_buf(path: PathBuf, start: Duration, duration: Option<Duration>) -> Self {
        Self {
            path: Arc::from(path),
            start,
            duration,
        }
    }
}
