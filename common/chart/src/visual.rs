//! Visual elements: BGA layer types and resource declarations.
//!
//! Barlines, scroll changes, and BGA events now live in the unified
//! [`Event`](crate::Event) enum; see [`crate::EventKind::Bar`],
//! [`crate::EventKind::Scroll`], and [`crate::EventKind::Bga`].

use std::path::PathBuf;

/// BGA layer targeted by a display event.
///
/// The renderer composites layers in this order: `Base` (bottom), then `Layer`,
/// then `Layer2`, then `Poor` (top, shown on a miss).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BgaLayer {
    /// Base / main background (channel `04`).
    #[default]
    Base,
    /// Miss / POOR presentation layer (channel `06`).
    Poor,
    /// Overlay layer on top of the main BGA (channel `07`).
    Layer,
    /// Second overlay layer (channel `0A`, nanasi extension).
    ///
    /// LAYER2 sits on top of LAYER.
    Layer2,
}

/// Index into [`ChartInfo::bga_resources`](crate::ChartInfo::bga_resources).
///
/// The vector index is the unique identifier; the dedicated newtype prevents
/// accidental assignment between this and raw `u32` values such as audio
/// indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BgaResourceId(u32);

impl BgaResourceId {
    /// Create a BGA resource index from a raw index.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the raw index value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<u32> for BgaResourceId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<BgaResourceId> for u32 {
    fn from(value: BgaResourceId) -> Self {
        value.0
    }
}

impl std::fmt::Display for BgaResourceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Image crop and placement definition (BMS `#BGA` / `#@BGA`).
///
/// Takes a rectangular region from the source `#BMP` image and places it at the
/// given offset on the BGA canvas. All coordinates are in pixels. The processor
/// normalizes `#@BGA`'s width/height form to this structure's bottom-right form:
/// `x2 = sx + w` and `y2 = sy + h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropRect {
    /// Source rectangle top-left X coordinate (pixels).
    pub x1: i32,
    /// Source rectangle top-left Y coordinate (pixels).
    pub y1: i32,
    /// Source rectangle bottom-right X coordinate (pixels).
    pub x2: i32,
    /// Source rectangle bottom-right Y coordinate (pixels).
    pub y2: i32,
    /// Target display offset X (pixels).
    pub dx: i32,
    /// Target display offset Y (pixels).
    pub dy: i32,
}

/// BGA resource (image or video file).
///
/// `crop` of [`None`] means display the whole image; [`Some`] means crop and
/// place according to [`CropRect`] (from BMS `#BGA` / `#@BGA`). BMSON has no
/// crop concept, so its processor always produces [`None`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgaResource {
    /// File path relative to the directory containing the chart file.
    pub path: PathBuf,
    /// Source crop rectangle and target offset. [`None`] means the whole image.
    pub crop: Option<CropRect>,
}

/// Background video resource (BMS `#VIDEOFILE` / `#MOVIE`).
///
/// BMSON has no corresponding concept, so its processor produces no video
/// resources.
///
/// # `Eq` guarantee
///
/// `fps` is an `Option<f64>`, so [`Eq`] is implemented manually; `fps` comes
/// from BMS parsing and is guaranteed to contain no NaN (following the
/// [`Damage`](crate::Damage) pattern). `colors` and `delay_frames` use `u32` to
/// avoid floating-point issues.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoAsset {
    /// Video file path (relative to the directory containing the chart file).
    pub path: PathBuf,
    /// Whether to loop (`#VIDEOFILE` is true, `#MOVIE` is false).
    pub loop_playback: bool,
    /// Frame-rate override (`#VIDEOf/s`). Frame rates may be fractional, so
    /// `f64` is kept.
    pub fps: Option<f64>,
    /// Palette depth in bits (`#VIDEOCOLORS`). The original BMS value is `f64`
    /// but semantically an integer.
    pub colors: Option<u32>,
    /// Starting frame delay (`#VIDEODLY`). The original BMS value is `f64` but
    /// semantically a frame number.
    pub delay_frames: Option<u32>,
}

impl VideoAsset {
    /// Create a looping or single-play `VideoAsset` with other parameters unset.
    #[must_use]
    pub const fn new(path: PathBuf, loop_playback: bool) -> Self {
        Self {
            path,
            loop_playback,
            fps: None,
            colors: None,
            delay_frames: None,
        }
    }

    /// Set the frame-rate override (`#VIDEOf/s`).
    #[must_use]
    pub const fn with_fps(mut self, fps: f64) -> Self {
        self.fps = Some(fps);
        self
    }

    /// Set the palette depth in bits (`#VIDEOCOLORS`).
    #[must_use]
    pub const fn with_colors(mut self, colors: u32) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set the starting frame delay (`#VIDEODLY`).
    #[must_use]
    pub const fn with_delay_frames(mut self, delay_frames: u32) -> Self {
        self.delay_frames = Some(delay_frames);
        self
    }
}

// fps is guaranteed not to be NaN (BMS parse values are not NaN), so Eq is safe.
impl Eq for VideoAsset {}
