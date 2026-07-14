//! 视觉元素：BGA 图层类型与资源声明。
//!
//! 小节线、滚动变更与 BGA 事件现归入统一的 [`Event`](crate::Event)
//! 枚举 —— 参见 [`crate::EventKind::Bar`]、[`crate::EventKind::Scroll`] 与
//! [`crate::EventKind::Bga`]。

use std::path::PathBuf;

/// 显示事件所针对的 BGA 图层。
///
/// 图层由渲染器按以下顺序合成：
/// `Base`（底层）→ `Layer` → `Layer2` → `Poor`（顶层，未命中时显示）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BgaLayer {
    /// 基础 / 主背景（通道 `04`）。
    #[default]
    Base,
    /// 未命中 / POOR 表现图层（通道 `05`、`06`）。
    Poor,
    /// 叠加在主 BGA 之上的覆盖图层（通道 `07`）。
    Layer,
    /// 第二覆盖图层（通道 `0A`，nanasi 扩展）。
    ///
    /// LAYER2 叠加在 LAYER 之上。
    Layer2,
}

/// 图片裁剪与放置定义（BMS `#BGA` / `#@BGA`）。
///
/// 从源 `#BMP` 图片中取一个矩形区域，放置到 BGA 画布的指定偏移处。
/// 所有坐标均为像素。`#@BGA` 的宽/高形式由 processor 归一为本结构
/// 的右下角形式：`x2 = sx + w`、`y2 = sy + h`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropRect {
    /// 源矩形左上角 X 坐标（像素）。
    pub x1: i32,
    /// 源矩形左上角 Y 坐标（像素）。
    pub y1: i32,
    /// 源矩形右下角 X 坐标（像素）。
    pub x2: i32,
    /// 源矩形右下角 Y 坐标（像素）。
    pub y2: i32,
    /// 目标显示偏移 X（像素）。
    pub dx: i32,
    /// 目标显示偏移 Y（像素）。
    pub dy: i32,
}

/// BGA 资源（图片或视频文件）。
///
/// `crop` 为 [`None`] 时表示显示整张图片；为 [`Some`] 时按 [`CropRect`]
/// 裁剪并放置（源自 BMS `#BGA` / `#@BGA`）。BMSON 不区分裁剪，
/// 其 processor 产出均为 [`None`]。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgaResource {
    /// 谱面内唯一标识符。
    pub id: u32,
    /// 相对于谱面文件所在目录的文件路径。
    pub path: PathBuf,
    /// 源裁剪矩形与目标偏移。[`None`] = 整图。
    pub crop: Option<CropRect>,
}

/// 背景视频资源（BMS `#VIDEOFILE` / `#MOVIE`）。
///
/// BMSON 无对应概念，其 processor 不产出视频资源。
///
/// # `Eq` 保证
///
/// `fps` 为 `Option<f64>`，手动实现 [`Eq`]——`fps` 由 BMS 解析得到，
/// 保证不含 NaN（仿 [`Damage`](crate::Damage) 模式）。
/// `colors` / `delay_frames` 用 `u32` 规避浮点问题。
#[derive(Clone, Debug, PartialEq)]
pub struct VideoAsset {
    /// 视频文件路径（相对于谱面文件所在目录）。
    pub path: PathBuf,
    /// 是否循环播放（`#VIDEOFILE` = true，`#MOVIE` = false）。
    pub loop_playback: bool,
    /// 帧率覆盖（`#VIDEOf/s`）。帧率可非整，故保留 `f64`。
    pub fps: Option<f64>,
    /// 调色板深度位（`#VIDEOCOLORS`）。原 BMS 值为 `f64`，语义为整数。
    pub colors: Option<u32>,
    /// 起始帧延迟（`#VIDEODLY`）。原 BMS 值为 `f64`，语义为帧序号。
    pub delay_frames: Option<u32>,
}

impl VideoAsset {
    /// 创建一个循环或单次播放的 `VideoAsset`，其余参数留空。
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

    /// 设置帧率覆盖（`#VIDEOf/s`）。
    #[must_use]
    pub const fn with_fps(mut self, fps: f64) -> Self {
        self.fps = Some(fps);
        self
    }

    /// 设置调色板深度位（`#VIDEOCOLORS`）。
    #[must_use]
    pub const fn with_colors(mut self, colors: u32) -> Self {
        self.colors = Some(colors);
        self
    }

    /// 设置起始帧延迟（`#VIDEODLY`）。
    #[must_use]
    pub const fn with_delay_frames(mut self, delay_frames: u32) -> Self {
        self.delay_frames = Some(delay_frames);
        self
    }
}

// fps 保证不含 NaN（BMS 解析值非 NaN），故可安全实现 Eq。
impl Eq for VideoAsset {}
