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

/// BGA 资源（图片或视频文件）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgaResource {
    /// 谱面内唯一标识符。
    pub id: u32,
    /// 相对于谱面文件所在目录的文件路径。
    pub path: PathBuf,
}
