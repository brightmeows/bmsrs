//! 音频素材类型。
//!
//! BGM 事件现归入统一的 [`Event`](crate::Event) 枚举 ——
//! 参见 [`crate::Event::Bgm`]。

use std::path::PathBuf;
use std::time::Duration;

/// 音频素材 —— 一个音频文件中的一段。
///
/// 对于 BMS 谱面，每个素材是一整个 WAV 文件（`start: Duration::ZERO`、
/// `duration: None`）。
///
/// 对于 BMSON 谱面，处理器使用音频通道切片算法预先计算出切片，
/// 在同一音频文件中产生具有特定 `start` 和 `duration` 值的素材。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioAsset {
    /// 相对于谱面文件所在目录的文件路径。
    pub path: PathBuf,
    /// 从文件起始处计算的切片起始偏移。
    /// 对于 BMS（无切片）始终为 [`Duration::ZERO`]。
    pub start: Duration,
    /// 切片时长。
    /// `None` 表示播放至文件末尾。
    pub duration: Option<Duration>,
}
