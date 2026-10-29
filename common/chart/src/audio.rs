//! 音频素材类型。
//!
//! BGM 事件现归入统一的 [`Event`](crate::Event) 枚举 ——
//! 参见 [`crate::EventKind::Bgm`]。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// 指向 [`ChartData::audio_assets`](crate::ChartData::audio_assets) 的索引。
///
/// 独立的 newtype 避免与 BGA 资源索引等裸 `u32` 互相赋值。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AudioIndex(u32);

impl AudioIndex {
    /// 从裸索引创建音频索引。
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// 返回裸索引值。
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<u32> for AudioIndex {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<AudioIndex> for u32 {
    fn from(value: AudioIndex) -> Self {
        value.0
    }
}

impl std::fmt::Display for AudioIndex {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// 音频素材 —— 一个音频文件中的一段。
///
/// 对于 BMS 谱面，每个素材是一整个 WAV 文件（`start: Duration::ZERO`、
/// `duration: None`）。
///
/// 对于 BMSON 谱面，处理器使用音频通道切片算法预先计算出切片，
/// 在同一音频文件中产生具有特定 `start` 和 `duration` 值的素材。
///
/// 路径使用 [`Arc<Path>`] 存储，使同一文件的多个切片可以共享路径
/// 而不产生重复的堆分配。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioAsset {
    /// 相对于谱面文件所在目录的文件路径。
    pub path: Arc<Path>,
    /// 从文件起始处计算的切片起始偏移。
    /// 对于 BMS（无切片）始终为 [`Duration::ZERO`]。
    pub start: Duration,
    /// 切片时长。
    /// `None` 表示播放至文件末尾。
    pub duration: Option<Duration>,
}

impl AudioAsset {
    /// 从 `PathBuf` 创建一个 `AudioAsset`，自动包装为 `Arc<Path>`。
    ///
    /// 便捷构造，用于 BMS 处理器中每个素材独立对应一个文件的情况。
    #[must_use]
    pub fn from_path_buf(path: PathBuf, start: Duration, duration: Option<Duration>) -> Self {
        Self {
            path: Arc::from(path),
            start,
            duration,
        }
    }
}
