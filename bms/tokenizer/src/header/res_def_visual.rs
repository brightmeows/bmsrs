//! 视觉资源定义头部：`#BMP`、`#EXBMP`、`#BGA`、
//! `#@BGA`、`#POORBGA`、`#SWBGA`、`#ARGB`、`#VIDEOFILE`、`#MOVIE`、
//! `#SEEK`、`#ExtChr`。

// 此模块大量使用 a、r、g、b 等单字符变量名表示 ARGB 颜色分量，
// 这是该领域的事实标准命名。
#![expect(clippy::many_single_char_names, reason = "ARGB color components: a=alpha, r=red, g=green, b=blue")]

use std::fmt;

use crate::header::display::PoorBgaMode;
use crate::index::{BmpIndex, SeekIndex};
use crate::{BmsHeader, BmsTokenAttr, BmsTryFromError, BmsValue};

/// `#BGA{id}` 的参数——图片裁剪与放置定义。
///
/// 从 `#BMP` 图片裁剪一个矩形区域并放置到
/// BGA 画布上。所有坐标均为像素。
///
/// 若同一索引同时在 `#BMP` 与 `#BGA` 中定义，`#BGA` 优先。
/// `BM98de` 将 `x1 y1 x2 y2 = 0 0 1 1` 视为 2×2 像素
/// 裁剪；大多数其他播放器视为 1×1。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgaParams {
    /// 指向 `#BMP` 表的索引（十进制）。
    pub bmp_index: u16,
    /// 左上角 X 坐标。
    pub x1: i32,
    /// 左上角 Y 坐标。
    pub y1: i32,
    /// 右下角 X 坐标。
    pub x2: i32,
    /// 右下角 Y 坐标。
    pub y2: i32,
    /// 显示偏移 X。
    pub dx: i32,
    /// 显示偏移 Y。
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

/// `#@BGA{id}` 的参数——图片裁剪与放置（宽/高形式）。
///
/// `#BGA` 的语法糖，改为指定宽/高而非
/// 右下角。内部等价于 `#BGA`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtBgaParams {
    /// 指向 `#BMP` 表的索引（十进制）。
    pub bmp_index: u16,
    /// 源 X。
    pub sx: i32,
    /// 源 Y。
    pub sy: i32,
    /// 宽度。
    pub w: i32,
    /// 高度。
    pub h: i32,
    /// 显示偏移 X。
    pub dx: i32,
    /// 显示偏移 Y。
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

/// `#EXBMP{id}` 的参数——带自定义透明色的图片
/// （nanasi 扩展）。
///
/// 类似 `#BMP`，但将指定的 ARGB 颜色视为透明，
/// 而非默认的纯黑（`RGB:00:00:00`）。索引
/// 与 `#BMP` 共享命名空间。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExBmpParams<C> {
    /// Alpha 分量（0–255）。
    pub a: u8,
    /// Red 分量（0–255）。
    pub r: u8,
    /// Green 分量（0–255）。
    pub g: u8,
    /// Blue 分量（0–255）。
    pub b: u8,
    /// 资源文件路径或名称。
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

/// `#SWBGA{id}` 的参数——按键绑定 BGA 动画（nanasi，
/// 实验性）。
///
/// 在指定通道上由按键输入触发的图片序列播放。
/// `pattern` 字段使用 BMS 消息表示法（例如 `"01020304"`），
/// 每对 2 字符引用一个 `#BMP`/`#EXBMP`/`#BGA`/`#@BGA`
/// 索引。与普通 BMS 消息不同，此处的 `00` **显示** `#BMP00`
/// 而非休止。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwBgaParams<C> {
    /// 帧率。
    pub fr: u32,
    /// 过渡时长（帧数）。
    pub time: u32,
    /// 扫描线方向。
    pub line: u8,
    /// 是否循环过渡。
    pub r#loop: bool,
    /// Alpha 分量。
    pub a: u8,
    /// Red 分量。
    pub r: u8,
    /// Green 分量。
    pub g: u8,
    /// Blue 分量。
    pub b: u8,
    /// 过渡图样名称或路径。
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

/// `#ARGB{id}` 的参数——逐图层颜色/alpha 叠加（nanasi）。
///
/// 将一个 ARGB 倍率应用到整个 BGA 图层（BASE / LAYER /
/// LAYER2 / POOR）。与 `#EXBMP`（逐图片）不同，这影响
/// 整个图层。索引被通道 `#xxxA1-A4` 引用。与不透明度通道
/// `#xxx0B-0E` 共享 alpha 通道。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgbParams {
    /// Alpha 分量（0–255）。
    pub a: u8,
    /// Red 分量（0–255）。
    pub r: u8,
    /// Green 分量（0–255）。
    pub g: u8,
    /// Blue 分量（0–255）。
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

/// 将空白分隔的字符串解析为恰好七个 `i32` 值。
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

/// 将逗号分隔的 `a,r,g,b` 字符串解析为四个 `u8` 值。
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

/// 视觉资源定义头部。
///
/// 这些命令定义谱面使用的图片、视频与 BGA（背景
/// 动画）图层。
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefVisual<C> {
    /// `#BMP{id}`——图片文件定义。
    ///
    /// 被 BGA 通道 `#xxx04`（BASE）、`#xxx06`（POOR）、
    /// `#xxx07`（LAYER）、`#xxx0A`（LAYER2）以及不透明度通道
    /// `#xxx0B-0E` 引用。通常为 256×256 像素；更大的图片
    /// 各播放器处理方式不同。
    ///
    /// 在 LAYER 通道（`#xxx07`）中，纯黑（`RGB:00:00:00`）
    /// 为透明，显示下方的 BASE 图层。
    ///
    /// `#BMP00` 是默认的 miss/poor 图片（当无 `#xxx06`
    /// 对象时显示）。
    ///
    /// **视频文件**也可分配给 `#BMP`（在 LR2、ruvit、
    /// Angolmois、Sonorous 等中）——MPG 是兼容性最好的格式。
    #[bms_token("#BMP{id} {filename}")]
    Bmp {
        /// 2 字符索引。
        id: BmpIndex,
        /// 资源文件路径或名称。
        filename: C,
    },
    /// `#EXBMP{id}`——带自定义透明色的图片（nanasi）。
    #[bms_token("#EXBMP{id} {params}")]
    #[bms_fallback]
    ExBmp {
        /// 2 字符索引。
        id: BmpIndex,
        /// 解析出的参数。
        params: ExBmpParams<C>,
    },
    /// `#BGA{id}`——图片裁剪与放置定义。
    #[bms_token("#BGA{id} {params}")]
    #[bms_fallback]
    Bga {
        /// 2 字符索引。
        id: BmpIndex,
        /// 解析出的放置参数。
        params: BgaParams,
    },
    /// `#@BGA{id}`——图片裁剪与放置（宽/高形式）。
    #[bms_token("#@BGA{id} {params}")]
    #[bms_fallback]
    AtBga {
        /// 2 字符索引。
        id: BmpIndex,
        /// 解析出的放置参数。
        params: AtBgaParams,
    },
    /// `#POORBGA`——poor/miss BGA 显示模式（nanasi）。
    #[bms_token("#POORBGA {}")]
    #[bms_fallback]
    PoorBga(PoorBgaMode),
    /// `#SWBGA{id}`——按键绑定 BGA 动画（nanasi，实验性）。
    #[bms_token("#SWBGA{id} {params}")]
    #[bms_fallback]
    SwBga {
        /// 2 字符索引。
        id: BmpIndex,
        /// 解析出的过渡参数。
        params: SwBgaParams<C>,
    },
    /// `#ARGB{id}`——逐图层 ARGB 颜色/alpha 叠加（nanasi）。
    #[bms_token("#ARGB{id} {params}")]
    #[bms_fallback]
    Argb {
        /// 2 字符索引。
        id: BmpIndex,
        /// 解析出的 ARGB 值。
        params: ArgbParams,
    },
    /// `#VIDEOFILE`——作为 BGA 的视频文件（bemaniaDX 起源）。
    ///
    /// 从 `#000` 播放；若谱面比视频长则循环。
    /// 视频音频被静音（nazoZZ 除外）。兼容格式：MPG
    /// （兼容性最好）、AVI、`WebM`、MP4 等（取决于播放器）。
    #[bms_token("#VIDEOFILE {}")]
    VideoFile(C),
    /// `#MOVIE`——作为 BGA 的视频文件，不循环（`DXEmu` 起源）。
    ///
    /// 从 `#000` 播放一次；结束时保持最后一帧。
    /// 与 `#xxx04` 冲突：`#xxx04` 中的图片文件输给
    /// `#MOVIE`，但 `#xxx04` 中的视频文件优先。
    #[bms_token("#MOVIE {}")]
    Movie(C),
    /// `#SEEK{id}`——以毫秒为单位的视频定位位置（LR 起源）。
    ///
    /// 被通道 `#xxx05` 引用。改变视频播放
    /// 位置。文档有限——可能不被广泛支持。
    #[bms_token("#SEEK{id} {value}")]
    Seek {
        /// 2 字符索引。
        id: SeekIndex,
        /// 定位时间（毫秒）。
        value: f64,
    },
    /// `#ExtChr`——自定义 UI 皮肤元素（`BM98k` 起源）。
    ///
    /// 允许 BMS 文件用自定义图片替换 BM98 的屏幕角色精灵。
    /// 仅 `BM98k` 和 DDR（部分）支持。
    /// 语法非常复杂；现代谱面很少使用。
    ///
    /// **历史注记**：`Project2DX` 格式曾用 `#ExtChr`
    /// 将 5K 视觉重映射为 7K 布局，这在专用 7K 通道
    /// （`#xxx18-19`）标准化之前。DDR 检测特定的 `#ExtChr`
    /// 模式以激活 `Project2DX` 模式。
    #[bms_token("#ExtChr {}")]
    ExtChr(C),
    /// `#VIDEOf/s`——视频帧率覆盖（仅 `bemaniaDX`）。
    ///
    /// 覆盖由 `#VIDEOFILE` 指定的视频的播放帧率。
    /// 省略则使用视频文件的原始帧率。
    #[bms_token("#VIDEOf/s {}")]
    VideoFps(f64),
    /// `#VIDEOCOLORS`——视频调色板深度（仅 `bemaniaDX`）。
    ///
    /// 设置视频播放的颜色深度（位）。
    /// 默认：`16`（16 位色）。
    #[bms_token("#VIDEOCOLORS {}")]
    VideoColors(f64),
    /// `#VIDEODLY`——视频起始帧延迟（仅 `bemaniaDX`）。
    ///
    /// 指定视频应从哪一帧开始播放。
    /// 默认：`0`（从头开始）。
    #[bms_token("#VIDEODLY {}")]
    VideoDly(f64),
}

// From / TryFrom 转换

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderResDefVisual<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::ResDefVisual(v) => Ok(v),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}
