//! 通道值解析警告。

use bms_values::BmsChannel;

/// 通道值无法解析为该通道要求的类型。
///
/// 由 [`Messages::finalize`](crate::Messages::finalize) 在解析通道事件时
/// 收集：值形状（2-char Base62 块）合法但不满足该通道的解析规则
/// （如 ch03 要求十六进制而值含非十六进制字符）。事件被跳过，警告
/// 携带定位信息供谱面作者排查。
///
/// 引用缺失类警告（WAV/BPM/STOP 键未定义）由 processor 层的
/// `ProcessWarning` 承载；值损坏类由本类型承载，processor 经
/// `ProcessWarning::UnparsableValue` 透传。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unparsable channel value \"{value}\" (channel {channel:?}) at measure {measure}")]
pub struct ChannelValueWarning {
    /// 小节号。
    pub measure: u16,
    /// 通道类型。
    pub channel: BmsChannel,
    /// 无法解析的原始 2-char 值。
    pub value: String,
}
