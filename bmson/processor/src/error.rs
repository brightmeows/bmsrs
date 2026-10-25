//! 处理警告类型定义。

/// BMSON 处理过程中可恢复的警告。
///
/// 与 BMS 侧的 `ProcessWarning` 对应；BMSON 数据经
/// serde 类型化解析，无值损坏类警告（解析失败在反序列化即报错）。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BmsonProcessWarning {
    /// 值为 0 的 BPM 变更被忽略。
    #[error("zero BPM change at tick {tick} ignored")]
    ZeroBpmChangeIgnored {
        /// 事件所在脉冲。
        tick: u64,
    },

    /// 负值 STOP 时长被钳位到 0。
    #[error("negative stop duration at tick {tick} clipped to 0 (original {original})")]
    StopDurationClipped {
        /// 事件所在脉冲。
        tick: u64,
        /// 原始的负值时长。
        original: f64,
    },

    /// 规范外的扩展 `mode_hint` 回退为 beat 布局。
    #[error("unknown mode hint \"{hint}\" fell back to beat layout")]
    UnknownModeHint {
        /// 原始的扩展模式提示字符串。
        hint: String,
    },
}
