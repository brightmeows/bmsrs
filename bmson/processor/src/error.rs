//! 处理警告类型定义。

/// BMSON 处理过程中可恢复的警告。
#[derive(Debug, Clone, PartialEq)]
pub enum BmsonProcessWarning {
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
}
