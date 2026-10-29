//! 处理警告类型定义。

use bmsrs_bms_parser::ChannelValueWarning;

/// BMS 处理过程中可恢复的警告。
///
/// 这些警告表示谱面中的异常情况，但仍可生成有效的 `Chart`。
///
/// 引用缺失类（`Missing*`）与值损坏类（[`ProcessWarning::UnparsableValue`]）
/// 共同构成 processor 层的完整可观测面；控制流结构问题由
/// `bmsrs-bms-syntax` 诊断负责。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProcessWarning {
    /// 音符引用了 `#WAV` 中未定义的键。
    #[error(
        "note at tick {tick} (player {player}, lane {lane}) references undefined WAV key {key}"
    )]
    MissingWavDefinition {
        /// 未定义的 WAV 键（base36 数值）。
        key: u16,
        /// 引用方音符所在玩家编号。
        player: u8,
        /// 引用方音符所在轨道编号。
        lane: u8,
        /// 事件所在脉冲。
        tick: u64,
    },

    /// 长音起点未配对的终点。
    #[error("long note start at tick {tick} (player {player}, lane {lane}) has no matching end")]
    UnterminatedLongNote {
        /// 玩家编号。
        player: u8,
        /// 轨道编号。
        lane: u8,
        /// 长音起点所在脉冲。
        tick: u64,
    },

    /// 值无效（零、NaN 或无穷大）的 BPM 变更被忽略。
    ///
    /// 过滤谓词与 [`TimingTrack`](bmsrs_chart::TimingTrack) 构造一致
    /// （[`BpmChange::is_valid`](bmsrs_chart::BpmChange::is_valid)），
    /// 无效值不进入计时轨，也不进入统一事件流。
    #[error("invalid BPM change at tick {tick} ignored (value {bpm})")]
    InvalidBpmChangeIgnored {
        /// 事件所在脉冲。
        tick: u64,
        /// 被忽略的 BPM 值。
        bpm: f64,
    },

    /// 负值 STOP 时长被钳位到 0。
    #[error("negative stop duration at tick {tick} clipped to 0 (computed {original})")]
    StopDurationClipped {
        /// 事件所在脉冲。
        tick: u64,
        /// 计算出的负值时长（被钳位前）。
        original: f64,
    },

    /// BPM 变更引用 `#BPMxx` 中未定义的键。
    #[error("BPM change at tick {tick} references undefined BPM key {key}")]
    MissingBpmDefinition {
        /// 未定义的 BPM 键（base36 数值）。
        key: u16,
        /// 事件所在脉冲。
        tick: u64,
    },

    /// 停止事件引用 `#STOPxx` 中未定义的键。
    #[error("stop event at tick {tick} references undefined STOP key {key}")]
    MissingStopDefinition {
        /// 未定义的 STOP 键（base36 数值）。
        key: u16,
        /// 事件所在脉冲。
        tick: u64,
    },

    /// 通道值无法解析为该通道要求的类型，事件被跳过。
    ///
    /// 透传 parser 层的 [`ChannelValueWarning`]（含 measure/channel/value
    /// 定位信息）。
    #[error(transparent)]
    UnparsableValue(#[from] ChannelValueWarning),
}
