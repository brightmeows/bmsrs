//! Visual / BGA resource definitions.
//!
//! Corresponds to [`BmsHeaderResDefVisual`] from the tokenizer.

use std::collections::BTreeMap;

use bms_tokenizer::{
    ArgbParams, AtBgaParams, BgaParams, BmpTag, BmsHeaderResDefVisual, BmsIndex, ExBmpParams,
    PoorBgaMode, SeekTag, SwBgaParams,
};

// Owned parameter types
//
// The tokenizer's ExBmpParams<'_> and SwBgaParams<'_> borrow from the input
// string.  Since Bms owns all its data, we convert to owned equivalents here.

/// Owned version of [`ExBmpParams`] with a `String` filename.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedExBmpParams {
    /// Alpha component (0–255).
    pub a: u8,
    /// Red component (0–255).
    pub r: u8,
    /// Green component (0–255).
    pub g: u8,
    /// Blue component (0–255).
    pub b: u8,
    /// Path or name of the resource file.
    pub filename: String,
}

impl From<&ExBmpParams<'_>> for OwnedExBmpParams {
    fn from(p: &ExBmpParams<'_>) -> Self {
        Self {
            a: p.a,
            r: p.r,
            g: p.g,
            b: p.b,
            filename: p.filename.to_owned(),
        }
    }
}

/// Owned version of [`SwBgaParams`] with a `String` pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedSwBgaParams {
    /// Frame rate.
    pub fr: u32,
    /// Transition time in frames.
    pub time: u32,
    /// Scanline direction.
    pub line: u8,
    /// Whether to loop the transition.
    pub r#loop: bool,
    /// Alpha component.
    pub a: u8,
    /// Red component.
    pub r: u8,
    /// Green component.
    pub g: u8,
    /// Blue component.
    pub b: u8,
    /// Transition pattern name or path.
    pub pattern: String,
}

impl From<&SwBgaParams<'_>> for OwnedSwBgaParams {
    fn from(p: &SwBgaParams<'_>) -> Self {
        Self {
            fr: p.fr,
            time: p.time,
            line: p.line,
            r#loop: p.r#loop,
            a: p.a,
            r: p.r,
            g: p.g,
            b: p.b,
            pattern: p.pattern.to_owned(),
        }
    }
}

// Visual

/// Visual / BGA resource definitions.
///
/// Corresponds to [`BmsHeaderResDefVisual`] from the tokenizer.
/// Indexed definitions use `BTreeMap`; scalar fields use `Option`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Visual {
    /// Image / BGA file definitions (`#BMP`).
    pub bmp_files: BTreeMap<BmsIndex<BmpTag>, String>,
    /// Video seek position definitions (`#SEEKxx`).
    pub seek_defs: BTreeMap<BmsIndex<SeekTag>, f64>,
    /// Extended BMP definitions with ARGB tint (`#EXBMPxx`).
    pub ex_bmp_defs: BTreeMap<BmsIndex<BmpTag>, OwnedExBmpParams>,
    /// BGA crop-and-place definitions (`#BGAxx`).
    pub crop_defs: BTreeMap<BmsIndex<BmpTag>, BgaParams>,
    /// BGA crop-and-place (width/height form) (`#@BGAxx`).
    pub alt_crop_defs: BTreeMap<BmsIndex<BmpTag>, AtBgaParams>,
    /// Key-bound BGA animation definitions (`#SWBGAxx`).
    pub sw_bga_defs: BTreeMap<BmsIndex<BmpTag>, OwnedSwBgaParams>,
    /// Per-layer colour / alpha overlay definitions (`#ARGBxx`).
    pub argb_defs: BTreeMap<BmsIndex<BmpTag>, ArgbParams>,
    /// Video file as BGA (`#VIDEOFILE`).
    pub video_file: Option<String>,
    /// Video file as BGA, no loop (`#MOVIE`).
    pub movie: Option<String>,
    /// External character animation file (`#EXTCHR`).
    pub ext_chr: Option<String>,
    /// Video frame rate override (`#VIDEOf/s`).
    pub video_fps: Option<f64>,
    /// Video colour count override (`#VIDEOCOLORS`).
    pub video_colors: Option<f64>,
    /// Video start delay (`#VIDEODLY`).
    pub video_dly: Option<f64>,
    /// Poor / miss BGA display mode (`#POORBGA`).
    pub poor_bga_mode: Option<PoorBgaMode>,
}

impl Visual {
    /// Apply a visual resource header to this struct.
    pub fn apply(&mut self, header: &BmsHeaderResDefVisual<'_>) {
        match header {
            BmsHeaderResDefVisual::Bmp { id, filename } => {
                self.bmp_files.insert(*id, (*filename).to_owned());
            }
            BmsHeaderResDefVisual::Seek { id, value } => {
                self.seek_defs.insert(*id, *value);
            }
            BmsHeaderResDefVisual::ExBmp { id, params } => {
                self.ex_bmp_defs.insert(*id, OwnedExBmpParams::from(params));
            }
            BmsHeaderResDefVisual::Bga { id, params } => {
                self.crop_defs.insert(*id, params.clone());
            }
            BmsHeaderResDefVisual::AtBga { id, params } => {
                self.alt_crop_defs.insert(*id, params.clone());
            }
            BmsHeaderResDefVisual::SwBga { id, params } => {
                self.sw_bga_defs.insert(*id, OwnedSwBgaParams::from(params));
            }
            BmsHeaderResDefVisual::Argb { id, params } => {
                self.argb_defs.insert(*id, params.clone());
            }
            BmsHeaderResDefVisual::VideoFile(s) => self.video_file = Some((*s).to_owned()),
            BmsHeaderResDefVisual::Movie(s) => self.movie = Some((*s).to_owned()),
            BmsHeaderResDefVisual::ExtChr(s) => self.ext_chr = Some((*s).to_owned()),
            BmsHeaderResDefVisual::VideoFps(v) => self.video_fps = Some(*v),
            BmsHeaderResDefVisual::VideoColors(c) => self.video_colors = Some(*c),
            BmsHeaderResDefVisual::VideoDly(d) => self.video_dly = Some(*d),
            BmsHeaderResDefVisual::PoorBga(m) => self.poor_bga_mode = Some(*m),
        }
    }
}
