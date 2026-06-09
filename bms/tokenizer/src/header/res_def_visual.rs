//! `#bmp`, `#bga` and related visual resource definitions.

use std::fmt;

use crate::header::display::PoorBgaMode;
use crate::id::{BmpTag, BmsChannelId, SeekTag};
use crate::{BmsTokenAttr, BmsValue};

/// Parameters for `#BGA{id}` — base BGA layer placement.
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

impl<'a> BmsValue<'a> for BgaParams {
    fn parse(s: &'a str) -> Option<Self> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 7 {
            return None;
        }
        let ints: Vec<i32> = parts
            .iter()
            .map(|p| p.parse().ok())
            .collect::<Option<_>>()?;
        Some(Self {
            bmp_index: u16::try_from(*ints.first()?).ok()?,
            x1: *ints.get(1)?,
            y1: *ints.get(2)?,
            x2: *ints.get(3)?,
            y2: *ints.get(4)?,
            dx: *ints.get(5)?,
            dy: *ints.get(6)?,
        })
    }
}

/// Parameters for `#@BGA{id}` — overlay BGA layer placement.
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

impl<'a> BmsValue<'a> for AtBgaParams {
    fn parse(s: &'a str) -> Option<Self> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 7 {
            return None;
        }
        let ints: Vec<i32> = parts
            .iter()
            .map(|p| p.parse().ok())
            .collect::<Option<_>>()?;
        Some(Self {
            bmp_index: u16::try_from(*ints.first()?).ok()?,
            sx: *ints.get(1)?,
            sy: *ints.get(2)?,
            w: *ints.get(3)?,
            h: *ints.get(4)?,
            dx: *ints.get(5)?,
            dy: *ints.get(6)?,
        })
    }
}

/// Parameters for `#EXBMP{id}` — extended BMP with alpha channel.
#[derive(Debug, Clone, PartialEq)]
pub struct ExBmpParams<'a> {
    /// Alpha component (0–255).
    pub a: u8,
    /// Red component (0–255).
    pub r: u8,
    /// Green component (0–255).
    pub g: u8,
    /// Blue component (0–255).
    pub b: u8,
    /// Path or name of the resource file.
    pub filename: &'a str,
}

impl fmt::Display for ExBmpParams<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{} {}",
            self.a, self.r, self.g, self.b, self.filename
        )
    }
}

impl<'a> BmsValue<'a> for ExBmpParams<'a> {
    fn parse(s: &'a str) -> Option<Self> {
        let (argb_part, rest) = s.split_once(' ')?;
        let (alpha, red, green, blue) = parse_argb(argb_part)?;
        Some(Self {
            a: alpha,
            r: red,
            g: green,
            b: blue,
            filename: rest.trim(),
        })
    }
}

/// Parameters for `#SWBGA{id}` — switch BGA with transition.
#[derive(Debug, Clone, PartialEq)]
pub struct SwBgaParams<'a> {
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
    pub pattern: &'a str,
}

impl fmt::Display for SwBgaParams<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let loop_val = if self.r#loop { "1" } else { "0" };
        write!(
            f,
            "{}:{}:{}:{}:{},{},{},{} {}",
            self.fr, self.time, self.line, loop_val, self.a, self.r, self.g, self.b, self.pattern
        )
    }
}

impl<'a> BmsValue<'a> for SwBgaParams<'a> {
    fn parse(s: &'a str) -> Option<Self> {
        let (param_part, pattern) = s.split_once(' ')?;
        let groups: Vec<&str> = param_part.split(':').collect();
        if groups.len() != 5 {
            return None;
        }
        let fr: u32 = groups.first()?.parse().ok()?;
        let time: u32 = groups.get(1)?.parse().ok()?;
        let line: u8 = groups.get(2)?.parse().ok()?;
        let r#loop = *groups.get(3)? == "1";
        let (alpha, red, green, blue) = parse_argb(groups.get(4)?)?;
        Some(Self {
            fr,
            time,
            line,
            r#loop,
            a: alpha,
            r: red,
            g: green,
            b: blue,
            pattern: pattern.trim(),
        })
    }
}

/// Parameters for `#ARGB{id}` — colour overlay definition.
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

impl<'a> BmsValue<'a> for ArgbParams {
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

/// Parse a comma-separated `a,r,g,b` string into four `u8` values.
fn parse_argb(s: &str) -> Option<(u8, u8, u8, u8)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let vals: Vec<u8> = parts
        .iter()
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    Some((*vals.first()?, *vals.get(1)?, *vals.get(2)?, *vals.get(3)?))
}

/// Visual resource definition headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefVisual<'a> {
    /// `#BMP{id}`
    #[bms_token("#BMP{id} {filename}")]
    Bmp {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#EXBMP{id}` — extended BMP with alpha channel.
    #[bms_token("#EXBMP{id} {params}")]
    #[bms_fallback]
    ExBmp {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Parsed parameters.
        params: ExBmpParams<'a>,
    },
    /// `#BGA{id}` — base BGA layer.
    #[bms_token("#BGA{id} {params}")]
    #[bms_fallback]
    Bga {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Parsed placement parameters.
        params: BgaParams,
    },
    /// `#@BGA{id}` — overlay BGA layer.
    #[bms_token("#@BGA{id} {params}")]
    #[bms_fallback]
    AtBga {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Parsed placement parameters.
        params: AtBgaParams,
    },
    /// `#POORBGA` — poor BGA display mode.
    #[bms_token("#POORBGA {value}")]
    #[bms_fallback]
    PoorBga(PoorBgaMode),
    /// `#SWBGA{id}` — switch BGA with transition.
    #[bms_token("#SWBGA{id} {params}")]
    #[bms_fallback]
    SwBga {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Parsed transition parameters.
        params: SwBgaParams<'a>,
    },
    /// `#ARGB{id}` — colour overlay.
    #[bms_token("#ARGB{id} {params}")]
    #[bms_fallback]
    Argb {
        /// The 2-character index.
        id: BmsChannelId<BmpTag>,
        /// Parsed ARGB values.
        params: ArgbParams,
    },
    /// `#VIDEOFILE`
    #[bms_token("#VIDEOFILE {value}")]
    VideoFile(&'a str),
    /// `#MOVIE`
    #[bms_token("#MOVIE {value}")]
    Movie(&'a str),
    /// `#SEEK{id}`
    #[bms_token("#SEEK{id} {value}")]
    Seek {
        /// The 2-character index.
        id: BmsChannelId<SeekTag>,
        /// The raw value.
        value: f64,
    },
    /// `#ExtChr`
    #[bms_token("#ExtChr {value}")]
    ExtChr(&'a str),
}
