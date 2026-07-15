//! 处理警告类型定义。

/// BMS 处理过程中可恢复的警告。
///
/// 这些警告表示谱面中的异常情况，但仍可生成有效的 `Chart`。
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessWarning {
    /// 音符引用了 `#WAV` 中未定义的键。
    MissingWavDefinition {
        /// 未定义的 WAV 键（base36 数值）。
        key: u16,
    },

    /// 长音起点未配对的终点。
    UnterminatedLongNote {
        /// 玩家编号。
        player: u8,
        /// 轨道编号。
        lane: u8,
    },

    /// 值为 0 的 BPM 变更被忽略。
    ZeroBpmChangeIgnored {
        /// 事件所在脉冲。
        tick: u64,
    },

    /// 负值 STOP 时长被钳位到 0。
    StopDurationClipped {
        /// 事件所在脉冲。
        tick: u64,
        /// 原始的负值时长。
        original: f64,
    },

    /// BPM 变更引用 `#BPMxx` 中未定义的键。
    MissingBpmDefinition {
        /// 未定义的 BPM 键（base36 数值）。
        key: u16,
    },

    /// 停止事件引用 `#STOPxx` 中未定义的键。
    MissingStopDefinition {
        /// 未定义的 STOP 键（base36 数值）。
        key: u16,
    },
}
