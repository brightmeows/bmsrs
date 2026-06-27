//! 视觉 / BGA 资源定义。
//!
//! 对应分词器的 [`BmsHeaderResDefVisual`]。

use std::collections::BTreeMap;

use bms_tokenizer::{
    ArgbParams, AtBgaParams, BgaParams, BmpIndex, BmsBase, BmsHeaderResDefVisual, ExBmpParams,
    PoorBgaMode, SeekIndex, SwBgaParams,
};

// 拥有型参数类型
//
// 分词器的 ExBmpParams<'_> 与 SwBgaParams<'_> 借用自输入字符串。由于
// Bms 拥有全部数据，此处转换为对应的拥有型等价类型。

/// [`ExBmpParams`] 的拥有型版本，文件名为 `String`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedExBmpParams {
    /// Alpha 分量（0–255）。
    pub a: u8,
    /// 红色分量（0–255）。
    pub r: u8,
    /// 绿色分量（0–255）。
    pub g: u8,
    /// 蓝色分量（0–255）。
    pub b: u8,
    /// 资源文件的路径或名称。
    pub filename: String,
}

impl<C: AsRef<str>> From<&ExBmpParams<C>> for OwnedExBmpParams {
    fn from(p: &ExBmpParams<C>) -> Self {
        Self {
            a: p.a,
            r: p.r,
            g: p.g,
            b: p.b,
            filename: p.filename.as_ref().to_owned(),
        }
    }
}

/// [`SwBgaParams`] 的拥有型版本，模式（pattern）为 `String`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedSwBgaParams {
    /// 帧率。
    pub fr: u32,
    /// 过渡时间（以帧为单位）。
    pub time: u32,
    /// 扫描线方向。
    pub line: u8,
    /// 是否循环过渡。
    pub r#loop: bool,
    /// Alpha 分量。
    pub a: u8,
    /// 红色分量。
    pub r: u8,
    /// 绿色分量。
    pub g: u8,
    /// 蓝色分量。
    pub b: u8,
    /// 过渡模式的名称或路径。
    pub pattern: String,
}

impl<C: AsRef<str>> From<&SwBgaParams<C>> for OwnedSwBgaParams {
    fn from(p: &SwBgaParams<C>) -> Self {
        Self {
            fr: p.fr,
            time: p.time,
            line: p.line,
            r#loop: p.r#loop,
            a: p.a,
            r: p.r,
            g: p.g,
            b: p.b,
            pattern: p.pattern.as_ref().to_owned(),
        }
    }
}

// 视觉资源

/// 视觉 / BGA 资源定义。
///
/// 对应分词器的 [`BmsHeaderResDefVisual`]。索引定义使用 `BTreeMap`；
/// 标量字段使用 `Option`。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Visual {
    /// 图片 / BGA 文件定义（`#BMP`）。
    pub bmp_files: BTreeMap<BmpIndex, String>,
    /// 视频跳转位置定义（`#SEEKxx`）。
    pub seek_defs: BTreeMap<SeekIndex, f64>,
    /// 带 ARGB 着色的扩展 BMP 定义（`#EXBMPxx`）。
    pub ex_bmp_defs: BTreeMap<BmpIndex, OwnedExBmpParams>,
    /// BGA 裁剪并放置定义（`#BGAxx`）。
    pub crop_defs: BTreeMap<BmpIndex, BgaParams>,
    /// BGA 裁剪并放置（宽高形式）（`#@BGAxx`）。
    pub alt_crop_defs: BTreeMap<BmpIndex, AtBgaParams>,
    /// 按键绑定的 BGA 动画定义（`#SWBGAxx`）。
    pub sw_bga_defs: BTreeMap<BmpIndex, OwnedSwBgaParams>,
    /// 按图层的颜色 / Alpha 叠加定义（`#ARGBxx`）。
    pub argb_defs: BTreeMap<BmpIndex, ArgbParams>,
    /// 作为 BGA 的视频文件（`#VIDEOFILE`）。
    pub video_file: Option<String>,
    /// 作为 BGA 的视频文件，不循环（`#MOVIE`）。
    pub movie: Option<String>,
    /// 外部角色动画文件（`#EXTCHR`）。
    pub ext_chr: Option<String>,
    /// 视频帧率覆盖（`#VIDEOf/s`）。
    pub video_fps: Option<f64>,
    /// 视频颜色数覆盖（`#VIDEOCOLORS`）。
    pub video_colors: Option<f64>,
    /// 视频开始延迟（`#VIDEODLY`）。
    pub video_dly: Option<f64>,
    /// POOR / miss 时 BGA 显示模式（`#POORBGA`）。
    pub poor_bga_mode: Option<PoorBgaMode>,
}

impl Visual {
    /// 将一个视觉资源头部命令应用到此结构体。
    ///
    /// 索引键（`BmpIndex`、`SeekIndex` 等）使用 `base` 归一化，以便在
    /// 标准 BMS 中进行不区分大小写的比较。
    pub fn apply<C: AsRef<str>>(&mut self, header: &BmsHeaderResDefVisual<C>, base: BmsBase) {
        macro_rules! norm_as {
            ($id:expr, $ty:ident) => {
                $ty::from($id.normalize(base))
            };
        }

        match header {
            BmsHeaderResDefVisual::Bmp { id, filename } => {
                self.bmp_files
                    .insert(norm_as!(id, BmpIndex), filename.as_ref().to_owned());
            }
            BmsHeaderResDefVisual::Seek { id, value } => {
                self.seek_defs.insert(norm_as!(id, SeekIndex), *value);
            }
            BmsHeaderResDefVisual::ExBmp { id, params } => {
                self.ex_bmp_defs
                    .insert(norm_as!(id, BmpIndex), OwnedExBmpParams::from(params));
            }
            BmsHeaderResDefVisual::Bga { id, params } => {
                self.crop_defs
                    .insert(norm_as!(id, BmpIndex), params.clone());
            }
            BmsHeaderResDefVisual::AtBga { id, params } => {
                self.alt_crop_defs
                    .insert(norm_as!(id, BmpIndex), params.clone());
            }
            BmsHeaderResDefVisual::SwBga { id, params } => {
                self.sw_bga_defs
                    .insert(norm_as!(id, BmpIndex), OwnedSwBgaParams::from(params));
            }
            BmsHeaderResDefVisual::Argb { id, params } => {
                self.argb_defs
                    .insert(norm_as!(id, BmpIndex), params.clone());
            }
            BmsHeaderResDefVisual::VideoFile(s) => self.video_file = Some(s.as_ref().to_owned()),
            BmsHeaderResDefVisual::Movie(s) => self.movie = Some(s.as_ref().to_owned()),
            BmsHeaderResDefVisual::ExtChr(s) => self.ext_chr = Some(s.as_ref().to_owned()),
            BmsHeaderResDefVisual::VideoFps(v) => self.video_fps = Some(*v),
            BmsHeaderResDefVisual::VideoColors(c) => self.video_colors = Some(*c),
            BmsHeaderResDefVisual::VideoDly(d) => self.video_dly = Some(*d),
            BmsHeaderResDefVisual::PoorBga(m) => self.poor_bga_mode = Some(*m),
        }
    }
}
