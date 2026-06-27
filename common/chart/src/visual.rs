//! Visual elements: BGA layer types and resource declarations.
//!
//! Bar lines, scroll changes, and BGA events are now part of the unified
//! [`Event`](crate::Event) enum — see [`crate::Event::Bar`],
//! [`crate::Event::Scroll`], and [`crate::Event::Bga`].

use std::path::PathBuf;

/// Which BGA layer a display event targets.
///
/// Layers are composited by the renderer in order:
/// `Base` (bottom) → `Layer` → `Layer2` → `Poor` (top, on miss).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BgaLayer {
    /// Base / primary background (channel `04`).
    #[default]
    Base,
    /// Miss / poor-performance layer (channels `05`, `06`).
    Poor,
    /// Overlay layer composited on top of the primary BGA (channel `07`).
    Layer,
    /// Secondary overlay layer (channel `0A`, nanasi extension).
    ///
    /// LAYER2 is composited on top of LAYER.
    Layer2,
}

/// A BGA resource (image or video file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgaResource {
    /// Unique identifier within the chart.
    pub id: u32,
    /// File path relative to the chart file's directory.
    pub path: PathBuf,
}
