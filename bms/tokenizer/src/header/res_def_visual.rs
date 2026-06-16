//! Visual resource definition headers: `#BMP`, `#EXBMP`, `#BGA`,
//! `#@BGA`, `#POORBGA`, `#SWBGA`, `#ARGB`, `#VIDEOFILE`, `#MOVIE`,
//! `#SEEK`, `#ExtChr`.

use std::fmt;

use crate::header::display::PoorBgaMode;
use crate::index::{BmpTag, BmsIndex, SeekTag};
use crate::{BmsHeader, BmsTokenAttr, BmsTryFromError, BmsValue};

/// Parameters for `#BGA{id}` — image crop-and-place definition.
///
/// Crops a rectangular region from a `#BMP` image and places it on the
/// BGA canvas.  All coordinates are in pixels.
///
/// If the same index is defined in both `#BMP` and `#BGA`, `#BGA` takes
/// priority.  `BM98de` treats `x1 y1 x2 y2 = 0 0 1 1` as a 2×2 pixel
/// crop; most others treat it as 1×1.
#[derive(Debug, Clone, PartialEq)]
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

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C> for BgaParams {
    fn parse(s: &'a str) -> Option<Self> {
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

/// Parameters for `#@BGA{id}` — image crop-and-place (width/height form).
///
/// Syntactic sugar for `#BGA` where you specify width/height instead of
/// bottom-right corner.  Internally equivalent to `#BGA`.
#[derive(Debug, Clone, PartialEq)]
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

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C>
    for AtBgaParams
{
    fn parse(s: &'a str) -> Option<Self> {
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

/// Parameters for `#EXBMP{id}` — image with custom transparency colour
/// (nanasi extension).
///
/// Like `#BMP`, but the specified ARGB colour is treated as transparent
/// instead of the default pure-black (`RGB:00:00:00`).  The index
/// shares the `#BMP` namespace.
#[derive(Debug, Clone, PartialEq)]
pub struct ExBmpParams<C> {
    /// Alpha component (0–255).
    pub a: u8,
    /// Red component (0–255).
    pub r: u8,
    /// Green component (0–255).
    pub g: u8,
    /// Blue component (0–255).
    pub b: u8,
    /// Path or name of the resource file.
    pub filename: C,
}

impl<C: AsRef<str> + fmt::Display> fmt::Display for ExBmpParams<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{} {}",
            self.a, self.r, self.g, self.b, self.filename
        )
    }
}

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C>
    for ExBmpParams<C>
{
    fn parse(s: &'a str) -> Option<Self> {
        let (argb_part, rest) = s.split_once(' ')?;
        let (alpha, red, green, blue) = parse_argb(argb_part)?;
        Some(Self {
            a: alpha,
            r: red,
            g: green,
            b: blue,
            filename: C::from(rest.trim()),
        })
    }
}

/// Parameters for `#SWBGA{id}` — key-bound BGA animation (nanasi,
/// experimental).
///
/// Plays an image sequence triggered by key input on a specified channel.
/// The `pattern` field uses BMS message notation (e.g., `"01020304"`)
/// where each 2-char pair references a `#BMP`/`#EXBMP`/`#BGA`/`#@BGA`
/// index.  Unlike normal BMS messages, `00` here **shows** `#BMP00`
/// rather than being a rest.
#[derive(Debug, Clone, PartialEq)]
pub struct SwBgaParams<C> {
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
    pub pattern: C,
}

impl<C: AsRef<str> + fmt::Display> fmt::Display for SwBgaParams<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let loop_val = if self.r#loop { "1" } else { "0" };
        write!(
            f,
            "{}:{}:{}:{}:{},{},{},{} {}",
            self.fr, self.time, self.line, loop_val, self.a, self.r, self.g, self.b, self.pattern
        )
    }
}

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C>
    for SwBgaParams<C>
{
    #[expect(clippy::many_single_char_names, reason = "ARGB component names")]
    fn parse(s: &'a str) -> Option<Self> {
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
            pattern: C::from(pattern.trim()),
        })
    }
}

/// Parameters for `#ARGB{id}` — per-layer colour/alpha overlay (nanasi).
///
/// Applies an ARGB multiplier to an entire BGA layer (BASE / LAYER /
/// LAYER2 / POOR).  Unlike `#EXBMP` (per-image), this affects the whole
/// layer.  The index is referenced by channels `#xxxA1-A4`.  Shares the
/// alpha channel with opacity channels `#xxx0B-0E`.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgbParams {
    /// Alpha component (0–255).
    pub a: u8,
    /// Red component (0–255).
    pub r: u8,
    /// Green component (0–255).
    pub g: u8,
    /// Blue component (0–255).
    pub b: u8,
}

impl fmt::Display for ArgbParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{},{}", self.a, self.r, self.g, self.b)
    }
}

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C> for ArgbParams {
    fn parse(s: &'a str) -> Option<Self> {
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
/// These commands define the images, videos, and BGA (background
/// animation) layers used by the chart.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefVisual<C> {
    /// `#BMP{id}` — image file definition.
    ///
    /// Referenced by BGA channels `#xxx04` (BASE), `#xxx06` (POOR),
    /// `#xxx07` (LAYER), `#xxx0A` (LAYER2), and opacity channels
    /// `#xxx0B-0E`.  Usually 256×256 pixels; larger images are handled
    /// differently by each player.
    ///
    /// In the LAYER channel (`#xxx07`), pure black (`RGB:00:00:00`) is
    /// transparent, showing the BASE layer underneath.
    ///
    /// `#BMP00` is the default miss/poor image (shown when no `#xxx06`
    /// objects are present).
    ///
    /// **Video files** can also be assigned to `#BMP` (in LR2, ruvit,
    /// Angolmois, Sonorous, etc.) — MPG is the most compatible format.
    #[bms_token("#BMP{id} {filename}")]
    Bmp {
        /// The 2-character index.
        id: BmsIndex<BmpTag>,
        /// Path or name of the resource file.
        filename: C,
    },
    /// `#EXBMP{id}` — image with custom transparency colour (nanasi).
    #[bms_token("#EXBMP{id} {params}")]
    #[bms_fallback]
    ExBmp {
        /// The 2-character index.
        id: BmsIndex<BmpTag>,
        /// Parsed parameters.
        params: ExBmpParams<C>,
    },
    /// `#BGA{id}` — image crop-and-place definition.
    #[bms_token("#BGA{id} {params}")]
    #[bms_fallback]
    Bga {
        /// The 2-character index.
        id: BmsIndex<BmpTag>,
        /// Parsed placement parameters.
        params: BgaParams,
    },
    /// `#@BGA{id}` — image crop-and-place (width/height form).
    #[bms_token("#@BGA{id} {params}")]
    #[bms_fallback]
    AtBga {
        /// The 2-character index.
        id: BmsIndex<BmpTag>,
        /// Parsed placement parameters.
        params: AtBgaParams,
    },
    /// `#POORBGA` — poor/miss BGA display mode (nanasi).
    #[bms_token("#POORBGA {}")]
    #[bms_fallback]
    PoorBga(PoorBgaMode),
    /// `#SWBGA{id}` — key-bound BGA animation (nanasi, experimental).
    #[bms_token("#SWBGA{id} {params}")]
    #[bms_fallback]
    SwBga {
        /// The 2-character index.
        id: BmsIndex<BmpTag>,
        /// Parsed transition parameters.
        params: SwBgaParams<C>,
    },
    /// `#ARGB{id}` — per-layer ARGB colour/alpha overlay (nanasi).
    #[bms_token("#ARGB{id} {params}")]
    #[bms_fallback]
    Argb {
        /// The 2-character index.
        id: BmsIndex<BmpTag>,
        /// Parsed ARGB values.
        params: ArgbParams,
    },
    /// `#VIDEOFILE` — video file as BGA (bemaniaDX origin).
    ///
    /// Plays from `#000`; loops if the chart is longer than the video.
    /// Video audio is muted (except nazoZZ).  Compatible formats: MPG
    /// (most compatible), AVI, `WebM`, MP4, etc. (player-dependent).
    #[bms_token("#VIDEOFILE {}")]
    VideoFile(C),
    /// `#MOVIE` — video file as BGA, no loop (`DXEmu` origin).
    ///
    /// Plays once from `#000`; holds the last frame when finished.
    /// Conflicts with `#xxx04`: image files in `#xxx04` lose to
    /// `#MOVIE`, but video files in `#xxx04` take priority.
    #[bms_token("#MOVIE {}")]
    Movie(C),
    /// `#SEEK{id}` — video seek position in milliseconds (LR origin).
    ///
    /// Referenced by channel `#xxx05`.  Changes the video playback
    /// position.  Limited documentation — may not be widely supported.
    #[bms_token("#SEEK{id} {value}")]
    Seek {
        /// The 2-character index.
        id: BmsIndex<SeekTag>,
        /// Seek time in milliseconds.
        value: f64,
    },
    /// `#ExtChr` — custom UI skin elements (`BM98k` origin).
    ///
    /// Allows the BMS file to replace BM98's on-screen character sprites
    /// with custom images.  Only supported by `BM98k` and DDR (partial).
    /// Very complex syntax; rarely used in modern charts.
    ///
    /// **Historical note**: the `Project2DX` format used `#ExtChr` to
    /// remap 5K visuals into a 7K layout before dedicated 7K channels
    /// (`#xxx18-19`) were standardised.  DDR detects specific `#ExtChr`
    /// patterns to activate `Project2DX` mode.
    #[bms_token("#ExtChr {}")]
    ExtChr(C),
    /// `#VIDEOf/s` — video frame rate override (`bemaniaDX` only).
    ///
    /// Overrides the playback frame rate of the video specified by
    /// `#VIDEOFILE`.  Omit to use the video file's native frame rate.
    #[bms_token("#VIDEOf/s {}")]
    VideoFps(f64),
    /// `#VIDEOCOLORS` — video palette depth (`bemaniaDX` only).
    ///
    /// Sets the colour depth (in bits) for video playback.
    /// Default: `16` (16-bit colour).
    #[bms_token("#VIDEOCOLORS {}")]
    VideoColors(f64),
    /// `#VIDEODLY` — video start-frame delay (`bemaniaDX` only).
    ///
    /// Specifies which frame the video should start playing from.
    /// Default: `0` (start from the beginning).
    #[bms_token("#VIDEODLY {}")]
    VideoDly(f64),
}

// From / TryFrom conversions

impl<C> From<BmsHeaderResDefVisual<C>> for BmsHeader<C> {
    #[inline]
    fn from(visual: BmsHeaderResDefVisual<C>) -> Self {
        BmsHeader::ResDefVisual(visual)
    }
}

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderResDefVisual<C> {
    type Error = BmsTryFromError<'static>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::ResDefVisual(v) => Ok(v),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}
