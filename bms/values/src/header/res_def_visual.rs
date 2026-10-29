//! Visual resource definition headers: `#BMP`, `#EXBMP`, `#BGA`, `#@BGA`,
//! `#POORBGA`, `#SWBGA`, `#ARGB`, `#VIDEOFILE`, `#MOVIE`, `#SEEK`, and
//! `#ExtChr`.

use std::fmt;

use crate::header::display::PoorBgaMode;
use crate::index::{BmpIndex, SeekIndex};
use crate::{BmsTokenAttr, BmsValue};

/// Parameters for `#BGA{id}`: image crop and placement definition.
///
/// Crops a rectangular region from a `#BMP` image and places it on the BGA
/// canvas. All coordinates are in pixels.
///
/// If the same index is defined in both `#BMP` and `#BGA`, `#BGA` wins.
/// `BM98de` treats `x1 y1 x2 y2 = 0 0 1 1` as a 2x2 pixel crop; most other
/// players treat it as 1x1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgaParams {
    /// Index into the `#BMP` table (decimal).
    pub bmp_index: u16,
    /// Top-left X coordinate.
    pub x1: i32,
    /// Top-left Y coordinate.
    pub y1: i32,
    /// Bottom-right X coordinate.
    pub x2: i32,
    /// Bottom-right Y coordinate.
    pub y2: i32,
    /// Display offset X.
    pub dx: i32,
    /// Display offset Y.
    pub dy: i32,
}

impl fmt::Display for BgaParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} {}",
            self.bmp_index, self.x1, self.y1, self.x2, self.y2, self.dx, self.dy
        )
    }
}

impl BmsValue for BgaParams {
    fn parse(s: &str) -> Option<Self> {
        let [bmp_index_raw, x1, y1, x2, y2, dx, dy] = parse_seven_ints(s)?;
        let bmp_index = u16::try_from(bmp_index_raw).ok()?;
        Some(Self {
            bmp_index,
            x1,
            y1,
            x2,
            y2,
            dx,
            dy,
        })
    }
}

/// Parameters for `#@BGA{id}`: image crop and placement (width/height form).
///
/// Syntactic sugar for `#BGA` that specifies width/height instead of the
/// bottom-right corner. Internally equivalent to `#BGA`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtBgaParams {
    /// Index into the `#BMP` table (decimal).
    pub bmp_index: u16,
    /// Source X.
    pub sx: i32,
    /// Source Y.
    pub sy: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
    /// Display offset X.
    pub dx: i32,
    /// Display offset Y.
    pub dy: i32,
}

impl fmt::Display for AtBgaParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} {}",
            self.bmp_index, self.sx, self.sy, self.w, self.h, self.dx, self.dy
        )
    }
}

impl BmsValue for AtBgaParams {
    fn parse(s: &str) -> Option<Self> {
        let [bmp_index_raw, sx, sy, w, h, dx, dy] = parse_seven_ints(s)?;
        let bmp_index = u16::try_from(bmp_index_raw).ok()?;
        Some(Self {
            bmp_index,
            sx,
            sy,
            w,
            h,
            dx,
            dy,
        })
    }
}

/// Parameters for `#EXBMP{id}`: image with a custom transparent color (nanasi
/// extension).
///
/// Like `#BMP`, but treats the specified ARGB color as transparent instead of
/// the default pure black (`RGB:00:00:00`). The index shares the `#BMP`
/// namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExBmpParams {
    /// Alpha component (0-255).
    pub a: u8,
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
    /// Resource file path or name.
    pub filename: String,
}

impl fmt::Display for ExBmpParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{} {}",
            self.a, self.r, self.g, self.b, self.filename
        )
    }
}

impl BmsValue for ExBmpParams {
    fn parse(s: &str) -> Option<Self> {
        let (argb_part, rest) = s.split_once(' ')?;
        let (alpha, red, green, blue) = parse_argb(argb_part)?;
        Some(Self {
            a: alpha,
            r: red,
            g: green,
            b: blue,
            filename: rest.trim().to_owned(),
        })
    }
}

/// Parameters for `#SWBGA{id}`: key-bound BGA animation (nanasi, experimental).
///
/// Plays an image sequence triggered by key input on the specified channel. The
/// `pattern` field uses BMS message notation (for example, `"01020304"`), where
/// each 2-character pair references a `#BMP`/`#EXBMP`/`#BGA`/`#@BGA` index.
/// Unlike ordinary BMS messages, `00` here **displays** `#BMP00` instead of
/// acting as a rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwBgaParams {
    /// Frame rate.
    pub fr: u32,
    /// Transition duration in frames.
    pub time: u32,
    /// Scanline direction.
    pub line: u8,
    /// Whether the transition loops.
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

impl fmt::Display for SwBgaParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let loop_val = if self.r#loop { "1" } else { "0" };
        write!(
            f,
            "{}:{}:{}:{}:{},{},{},{} {}",
            self.fr, self.time, self.line, loop_val, self.a, self.r, self.g, self.b, self.pattern
        )
    }
}

impl BmsValue for SwBgaParams {
    #[expect(clippy::many_single_char_names, reason = "ARGB component names")]
    fn parse(s: &str) -> Option<Self> {
        let (param_part, pattern) = s.split_once(' ')?;
        let mut groups = param_part.split(':');
        let fr: u32 = groups.next()?.parse().ok()?;
        let time: u32 = groups.next()?.parse().ok()?;
        let line: u8 = groups.next()?.parse().ok()?;
        let r#loop = groups.next()? == "1";
        let (a, r, g, b) = parse_argb(groups.next()?)?;
        if groups.next().is_some() {
            return None;
        }
        Some(Self {
            fr,
            time,
            line,
            r#loop,
            a,
            r,
            g,
            b,
            pattern: pattern.trim().to_owned(),
        })
    }
}

/// Parameters for `#ARGB{id}`: per-layer color and alpha overlay (nanasi).
///
/// Applies an ARGB multiplier to an entire BGA layer (BASE / LAYER / LAYER2 /
/// POOR). Unlike `#EXBMP` (per image), this affects the whole layer. The index
/// is referenced by channels `#xxxA1-A4`. It shares the alpha channel with the
/// opacity channels `#xxx0B-0E`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgbParams {
    /// Alpha component (0-255).
    pub a: u8,
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
}

impl fmt::Display for ArgbParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{},{}", self.a, self.r, self.g, self.b)
    }
}

impl BmsValue for ArgbParams {
    fn parse(s: &str) -> Option<Self> {
        let (alpha, red, green, blue) = parse_argb(s.trim())?;
        Some(Self {
            a: alpha,
            r: red,
            g: green,
            b: blue,
        })
    }
}

/// Parse a whitespace-separated string into exactly seven `i32` values.
fn parse_seven_ints(s: &str) -> Option<[i32; 7]> {
    let mut iter = s.split_whitespace();
    let v0 = iter.next()?.parse().ok()?;
    let v1 = iter.next()?.parse().ok()?;
    let v2 = iter.next()?.parse().ok()?;
    let v3 = iter.next()?.parse().ok()?;
    let v4 = iter.next()?.parse().ok()?;
    let v5 = iter.next()?.parse().ok()?;
    let v6 = iter.next()?.parse().ok()?;
    if iter.next().is_some() {
        return None;
    }
    Some([v0, v1, v2, v3, v4, v5, v6])
}

/// Parse a comma-separated `a,r,g,b` string into four `u8` values.
#[expect(clippy::many_single_char_names, reason = "ARGB component names")]
fn parse_argb(s: &str) -> Option<(u8, u8, u8, u8)> {
    let mut parts = s.split(',');
    let a = parts.next()?.trim().parse().ok()?;
    let r = parts.next()?.trim().parse().ok()?;
    let g = parts.next()?.trim().parse().ok()?;
    let b = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((a, r, g, b))
}

/// Visual resource definition headers.
///
/// These commands define the images, videos, and BGA (background animation)
/// layers used by a chart.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefVisual {
    /// `#BMP{id}`: image file definition.
    ///
    /// Referenced by BGA channels `#xxx04` (BASE), `#xxx06` (POOR), `#xxx07`
    /// (LAYER), `#xxx0A` (LAYER2), and the opacity channels `#xxx0B-0E`.
    /// Usually 256x256 pixels; players handle larger images differently.
    ///
    /// On LAYER channels (`#xxx07`), pure black (`RGB:00:00:00`) is transparent,
    /// revealing the BASE layer underneath.
    ///
    /// `#BMP00` is the default miss/poor image (shown when there are no
    /// `#xxx06` objects).
    ///
    /// **Video files** may also be assigned to `#BMP` (in LR2, ruvit,
    /// Angolmois, Sonorous, and others); MPG has the best compatibility.
    #[bms_token("#BMP{id} {filename}")]
    Bmp {
        /// 2-character index.
        id: BmpIndex,
        /// Resource file path or name.
        filename: String,
    },
    /// `#EXBMP{id}`: image with a custom transparent color (nanasi).
    #[bms_token("#EXBMP{id} {params}")]
    #[bms_fallback]
    ExBmp {
        /// 2-character index.
        id: BmpIndex,
        /// Parsed parameters.
        params: ExBmpParams,
    },
    /// `#BGA{id}`: image crop and placement definition.
    #[bms_token("#BGA{id} {params}")]
    #[bms_fallback]
    Bga {
        /// 2-character index.
        id: BmpIndex,
        /// Parsed placement parameters.
        params: BgaParams,
    },
    /// `#@BGA{id}`: image crop and placement (width/height form).
    #[bms_token("#@BGA{id} {params}")]
    #[bms_fallback]
    AtBga {
        /// 2-character index.
        id: BmpIndex,
        /// Parsed placement parameters.
        params: AtBgaParams,
    },
    /// `#POORBGA`: poor/miss BGA display mode (nanasi).
    #[bms_token("#POORBGA {}")]
    #[bms_fallback]
    PoorBga(PoorBgaMode),
    /// `#SWBGA{id}`: key-bound BGA animation (nanasi, experimental).
    #[bms_token("#SWBGA{id} {params}")]
    #[bms_fallback]
    SwBga {
        /// 2-character index.
        id: BmpIndex,
        /// Parsed transition parameters.
        params: SwBgaParams,
    },
    /// `#ARGB{id}`: per-layer ARGB color and alpha overlay (nanasi).
    #[bms_token("#ARGB{id} {params}")]
    #[bms_fallback]
    Argb {
        /// 2-character index.
        id: BmpIndex,
        /// Parsed ARGB values.
        params: ArgbParams,
    },
    /// `#VIDEOFILE`: video file as BGA (bemaniaDX origin).
    ///
    /// Plays from `#000`; loops if the chart is longer than the video. Video
    /// audio is muted (except in nazoZZ). Compatible formats: MPG (best
    /// compatibility), AVI, `WebM`, MP4, and others depending on the player.
    #[bms_token("#VIDEOFILE {}")]
    VideoFile(String),
    /// `#MOVIE`: video file as BGA, without looping (`DXEmu` origin).
    ///
    /// Plays once from `#000` and holds the last frame when it ends. Conflicts
    /// with `#xxx04`: an image file in `#xxx04` loses to `#MOVIE`, but a video
    /// file in `#xxx04` takes priority.
    #[bms_token("#MOVIE {}")]
    Movie(String),
    /// `#SEEK{id}`: video seek position in milliseconds (LR origin).
    ///
    /// Referenced by channel `#xxx05`. Changes the video playback position.
    /// Documentation is limited; support may not be widespread.
    #[bms_token("#SEEK{id} {value}")]
    Seek {
        /// 2-character index.
        id: SeekIndex,
        /// Seek time in milliseconds.
        value: f64,
    },
    /// `#ExtChr`: custom UI skin elements (`BM98k` origin).
    ///
    /// Lets a BMS file replace BM98's on-screen character sprites with custom
    /// images. Only `BM98k` and DDR (partially) support it. The syntax is very
    /// complex; modern charts rarely use it.
    ///
    /// **Historical note**: the `Project2DX` format once used `#ExtChr` to remap
    /// a 5K visual layout to 7K, before dedicated 7K channels (`#xxx18-19`) were
    /// standardized. DDR detects specific `#ExtChr` patterns to activate
    /// `Project2DX` mode.
    #[bms_token("#ExtChr {}")]
    ExtChr(String),
    /// `#VIDEOf/s`: video frame-rate override (`bemaniaDX` only).
    ///
    /// Overrides the playback frame rate of the video specified by
    /// `#VIDEOFILE`. When omitted, the video file's original frame rate is
    /// used.
    #[bms_token("#VIDEOf/s {}")]
    VideoFps(f64),
    /// `#VIDEOCOLORS`: video color depth (`bemaniaDX` only).
    ///
    /// Sets the color depth in bits for video playback. Default: `16` (16-bit
    /// color).
    #[bms_token("#VIDEOCOLORS {}")]
    VideoColors(f64),
    /// `#VIDEODLY`: video starting-frame delay (`bemaniaDX` only).
    ///
    /// Specifies which frame the video should start from. Default: `0` (start
    /// from the beginning).
    #[bms_token("#VIDEODLY {}")]
    VideoDly(f64),
}
