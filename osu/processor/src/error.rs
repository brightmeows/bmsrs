//! 错误类型定义。

use bmsrs_chart::TimingTrackError;
use thiserror::Error;

use crate::custom_event::OsuMode;

/// osu! 谱面解析与转换过程中可能出现的错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OsuError {
    /// 读取或解析 `.osu` 内容时发生 I/O 错误。
    ///
    /// `rosu-map` 的便捷解析入口把读取与解码错误统一为
    /// [`std::io::Error`]；单行语义错误在默认配置下被静默忽略。
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// 解析成功，但无法转换为 `Chart`。
    #[error(transparent)]
    Process(#[from] OsuProcessError),
}

/// osu!mania 转换为 `Chart` 时可能出现的错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OsuProcessError {
    /// 谱面模式不是 osu!mania。
    #[error("unsupported osu! mode {mode:?}; only mania is supported")]
    UnsupportedMode {
        /// 实际模式。
        mode: OsuMode,
    },
    /// `CircleSize` 不是 1 到 18 的整数。
    #[error(
        "invalid mania key count derived from CircleSize {circle_size}; expected an integer in 1..=18"
    )]
    InvalidKeyCount {
        /// 原始的 `CircleSize` 值。
        circle_size: f32,
    },
    /// 没有非继承 timing point，无法构造计时轨。
    #[error("beatmap has no uninherited timing point")]
    MissingTimingPoint,
    /// `AudioFilename` 为空。
    #[error("beatmap has no audio filename")]
    MissingAudioFilename,
    /// 音符时间为负。
    #[error("hit object {index} has negative time {time} ms")]
    NegativeHitObjectTime {
        /// 音符在 `[HitObjects]` 中的下标。
        index: usize,
        /// 负的时间值（毫秒）。
        time: f64,
    },
    /// hold 的结束时间不晚于开始时间。
    #[error("hit object {index} is an invalid hold: start {start} ms, end {end} ms")]
    InvalidHold {
        /// 音符在 `[HitObjects]` 中的下标。
        index: usize,
        /// hold 开始时间（毫秒）。
        start: f64,
        /// hold 结束时间（毫秒）。
        end: f64,
    },
    /// mania 谱面中出现 slider 或 spinner。
    #[error("hit object {index} is a {kind}, which is not valid for osu!mania")]
    UnsupportedObject {
        /// 音符在 `[HitObjects]` 中的下标。
        index: usize,
        /// 物件类型名称。
        kind: &'static str,
    },
    /// 毫秒换算到 tick 时溢出。
    #[error("time {time} ms cannot be represented as a tick")]
    TimeOverflow {
        /// 触发溢出的时间值（毫秒）。
        time: f64,
    },
    /// 计时轨构造失败。
    #[error(transparent)]
    Timing(#[from] TimingTrackError),
}
