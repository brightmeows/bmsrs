//! Visual and BGA resource definitions.
//!
//! Corresponds to the tokenizer's [`BmsHeaderResDefVisual`].

use std::collections::BTreeMap;

use bmsrs_bms_values::{
    ArgbParams, AtBgaParams, BgaParams, BmpIndex, BmsBase, BmsHeaderResDefVisual, ExBmpParams,
    PoorBgaMode, SeekIndex, SwBgaParams,
};

// Visual resources

/// Visual and BGA resource definitions.
///
/// Corresponds to the tokenizer's [`BmsHeaderResDefVisual`]. Indexed definitions
/// use `BTreeMap`; scalar fields use `Option`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Visual {
    /// Image and BGA file definitions (`#BMP`).
    pub bmp_files: BTreeMap<BmpIndex, String>,
    /// Video seek position definitions (`#SEEKxx`).
    pub seek_defs: BTreeMap<SeekIndex, f64>,
    /// Extended BMP definitions with ARGB tinting (`#EXBMPxx`).
    pub ex_bmp_defs: BTreeMap<BmpIndex, ExBmpParams>,
    /// BGA crop-and-place definitions (`#BGAxx`).
    pub crop_defs: BTreeMap<BmpIndex, BgaParams>,
    /// BGA crop-and-place, width/height form (`#@BGAxx`).
    pub alt_crop_defs: BTreeMap<BmpIndex, AtBgaParams>,
    /// Key-bound BGA animation definitions (`#SWBGAxx`).
    pub sw_bga_defs: BTreeMap<BmpIndex, SwBgaParams>,
    /// Per-layer color and alpha overlay definitions (`#ARGBxx`).
    pub argb_defs: BTreeMap<BmpIndex, ArgbParams>,
    /// Video file used as BGA (`#VIDEOFILE`).
    pub video_file: Option<String>,
    /// Video file used as BGA without looping (`#MOVIE`).
    pub movie: Option<String>,
    /// External character animation file (`#EXTCHR`).
    pub ext_chr: Option<String>,
    /// Video frame-rate override (`#VIDEOf/s`).
    pub video_fps: Option<f64>,
    /// Video color-count override (`#VIDEOCOLORS`).
    pub video_colors: Option<f64>,
    /// Video start delay (`#VIDEODLY`).
    pub video_dly: Option<f64>,
    /// BGA display mode on POOR or miss (`#POORBGA`).
    pub poor_bga_mode: Option<PoorBgaMode>,
}

impl Visual {
    /// Apply one visual resource header command to this structure.
    ///
    /// Index keys (`BmpIndex`, `SeekIndex`, and so on) are normalized with
    /// `base` for case-insensitive comparison in standard BMS.
    pub fn apply(&mut self, header: &BmsHeaderResDefVisual, base: BmsBase) {
        // Only #BMPxx (and its #EXBMPxx alias) are in the Base62 command list.
        // The #SEEKxx, #BGAxx/#@BGAxx, #SWBGAxx, and #ARGBxx namespaces keep
        // Base36 case-insensitivity even in Base62 files.
        let bmp_base = crate::family_base(base, true);
        let ext_base = crate::family_base(base, false);
        match header {
            BmsHeaderResDefVisual::Bmp { id, filename } => {
                self.bmp_files
                    .insert(BmpIndex::from(id.normalize(bmp_base)), filename.clone());
            }
            BmsHeaderResDefVisual::Seek { id, value } => {
                self.seek_defs
                    .insert(SeekIndex::from(id.normalize(ext_base)), *value);
            }
            BmsHeaderResDefVisual::ExBmp { id, params } => {
                self.ex_bmp_defs
                    .insert(BmpIndex::from(id.normalize(bmp_base)), params.clone());
            }
            BmsHeaderResDefVisual::Bga { id, params } => {
                self.crop_defs
                    .insert(BmpIndex::from(id.normalize(ext_base)), params.clone());
            }
            BmsHeaderResDefVisual::AtBga { id, params } => {
                self.alt_crop_defs
                    .insert(BmpIndex::from(id.normalize(ext_base)), params.clone());
            }
            BmsHeaderResDefVisual::SwBga { id, params } => {
                self.sw_bga_defs
                    .insert(BmpIndex::from(id.normalize(ext_base)), params.clone());
            }
            BmsHeaderResDefVisual::Argb { id, params } => {
                self.argb_defs
                    .insert(BmpIndex::from(id.normalize(ext_base)), params.clone());
            }
            BmsHeaderResDefVisual::VideoFile(s) => self.video_file = Some(s.clone()),
            BmsHeaderResDefVisual::Movie(s) => self.movie = Some(s.clone()),
            BmsHeaderResDefVisual::ExtChr(s) => self.ext_chr = Some(s.clone()),
            BmsHeaderResDefVisual::VideoFps(v) => self.video_fps = Some(*v),
            BmsHeaderResDefVisual::VideoColors(c) => self.video_colors = Some(*c),
            BmsHeaderResDefVisual::VideoDly(d) => self.video_dly = Some(*d),
            BmsHeaderResDefVisual::PoorBga(m) => self.poor_bga_mode = Some(*m),
        }
    }
}
