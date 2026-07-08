//! BMS 引擎特定自定义事件类型。
//!
//! 这些事件由 [`BmsProcessor`](crate::BmsProcessor) 从 BMS 通道（如 BGA 不透明度、
//! ARGB 颜色、TEXT、OPTION 等）转换而来，存储在
//! [`EventKind::Custom`](bmsrs_chart::EventKind::Custom) 变体中。
//! tick 统一由 [`Event`](bmsrs_chart::Event) 存储，
//! 本类型的变体不含 tick。

use bmsrs_chart::{BgaLayer, CustomEvent};

/// BMS 格式特有的谱面事件。
///
/// # 设计原则
///
/// - 所有变体**不含** `tick` 字段——tick 由 [`Event`](bmsrs_chart::Event) 统一存储。
/// - 仅包含引擎特定的事件；通用概念（音符、BPM、BGA 等）由
///   [`Event`](bmsrs_chart::Event) 的原生变体承载。
///
/// # 标记 trait
///
/// 本类型实现 [`CustomEvent`]（无方法标记 trait），
/// 作为 [`EventKind::Custom`](bmsrs_chart::EventKind::Custom) 的 `C` 类型参数使用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BmsCustomEvent {
    /// BGA 图层不透明度变更（通道 `0B`–`0E`）。
    ///
    /// 值范围 `01`–`FF`（十六进制），`00` = 完全透明，`FF` = 完全不透明。
    BgaOpacity {
        /// 目标 BGA 图层。
        layer: BgaLayer,
        /// 不透明度（0–255）。
        opacity: u8,
    },
    /// BGA 图层 ARGB 颜色变更（通道 `A1`–`A4`）。
    ///
    /// 每个颜色分量范围 `00`–`FF`。
    BgaArgb {
        /// 目标 BGA 图层。
        layer: BgaLayer,
        /// Alpha 通道。
        a: u8,
        /// 红色分量。
        r: u8,
        /// 绿色分量。
        g: u8,
        /// 蓝色分量。
        b: u8,
    },
    /// BGA 按键绑定（通道 `A5` / `#SWBGA`）。
    ///
    /// 将 BGA 资源绑定到按键操作，按下时显示对应帧。
    BgaKeyBound {
        /// BGA 资源索引（对应 BMP 定义中的资源）。
        resource_id: u32,
    },
    /// 定时文本显示（通道 `99` / `#TEXTxx` / `#SONGxx`）。
    TextDisplay {
        /// TEXT 定义索引。
        text_index: u32,
    },
    /// 逐位置判定覆盖（通道 `A0` / `#EXRANK`）。
    ///
    /// 覆盖此脉冲后的判定窗口（与 `#DEFEXRANK` 乘算生效）。
    JudgeOverride {
        /// 判定窗口倍率（`100` = NORMAL）。
        rank: u64,
    },
    /// 玩家选项变更（通道 `A6` / `#CHANGEOPTION`）。
    OptionChange {
        /// 选项 ID。
        option_id: u64,
        /// 选项值。
        value: String,
    },
    /// BGM 通道音量（通道 `97`）。
    BgmVolume {
        /// 音量（`01`–`FF`，`FF` = 原声）。
        volume: u8,
    },
    /// KEY 通道音量（通道 `98`）。
    KeyVolume {
        /// 音量（`01`–`FF`，`FF` = 原声）。
        volume: u8,
    },
    /// 视频定位（通道 `05` / `#SEEK`）。
    VideoSeek {
        /// 视频目标位置。
        position: u64,
    },
}

impl CustomEvent for BmsCustomEvent {}
