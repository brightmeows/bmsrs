//! osu!mania → `Chart` 转换处理器。
//!
//! [`OsuProcessor`] 使用 [`rosu_map`] 解析 `.osu` 文本，并将 osu!mania
//! 谱面转换为格式无关的 [`Chart`]：
//!
//! ```text
//! .osu bytes → rosu_map::Beatmap → OsuProcessor::process → Chart<OsuNoteExt, OsuCustomEvent>
//! ```
//!
//! # 支持范围
//!
//! - 只接受 `Mode: 3`（osu!mania）。
//! - circle 映射为 [`NoteKind::Normal`](bmsrs_chart::NoteKind::Normal)，
//!   hold 映射为 [`NoteKind::Long`](bmsrs_chart::NoteKind::Long)。
//! - 列位置由 `x` 坐标与 `CircleSize` 键数计算，使用
//!   [`NoteSide::P1`](bmsrs_chart::NoteSide::P1)。
//! - hit sample 放入 [`OsuNoteExt`]；谱面级元数据、难度、Kiai、Break
//!   放入 [`OsuCustomEvent`]。
//! - slider、spinner、standard/taiko/catch、故事板与视频不在本轮范围。
//!
//! # Example
//!
//! ```
//! use osu_processor::OsuProcessor;
//!
//! let chart = OsuProcessor::from_str(
//!     "osu file format v14\n\
//!      [General]\n\
//!      AudioFilename: audio.ogg\n\
//!      Mode: 3\n\
//!      [Difficulty]\n\
//!      CircleSize: 4\n\
//!      [TimingPoints]\n\
//!      0,500,4,2,0,100,1,0\n\
//!      [HitObjects]\n\
//!      64,192,1000,1,0,0:0:0:0:\n",
//! )
//! .unwrap();
//! assert_eq!(chart.data.audio_assets.len(), 1);
//! ```

mod convert;
mod custom_event;
mod error;
mod note_ext;

use std::path::Path;

use bmsrs_chart::Chart;

pub use crate::custom_event::{OsuChartMeta, OsuCustomEvent, OsuMode};
pub use crate::error::{OsuError, OsuProcessError};
pub use crate::note_ext::{OsuHitSample, OsuHitSampleName, OsuNoteExt, OsuSampleBank};

/// osu!mania 谱面处理器。
pub struct OsuProcessor;

impl OsuProcessor {
    /// 将已解析的 osu! 谱面转换为 [`Chart`]。
    ///
    /// # Errors
    ///
    /// 非 mania、键数非法、缺少 timing point 或音频、出现 slider/spinner、
    /// 时间溢出等情况返回 [`OsuProcessError`]。
    pub fn process(
        map: &rosu_map::Beatmap,
    ) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuProcessError> {
        convert::process(map)
    }

    /// 从 `.osu` 字节内容解析并转换。
    ///
    /// # Errors
    ///
    /// `.osu` 读取或解码失败、转换失败时返回 [`OsuError`]。
    pub fn from_bytes(bytes: &[u8]) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuError> {
        let map: rosu_map::Beatmap = rosu_map::from_bytes(bytes)?;
        Self::process(&map).map_err(OsuError::from)
    }

    /// 从 `.osu` 文本解析并转换。
    ///
    /// # Errors
    ///
    /// `.osu` 解码失败或转换失败时返回 [`OsuError`]。
    #[expect(
        clippy::should_implement_trait,
        reason = "returns Chart, not OsuProcessor; FromStr cannot express the conversion"
    )]
    pub fn from_str(content: &str) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuError> {
        let map: rosu_map::Beatmap = rosu_map::from_str(content)?;
        Self::process(&map).map_err(OsuError::from)
    }

    /// 从 `.osu` 文件路径解析并转换。
    ///
    /// # Errors
    ///
    /// 文件读取、`.osu` 解码或转换失败时返回 [`OsuError`]。
    pub fn from_path(
        path: impl AsRef<Path>,
    ) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuError> {
        let map: rosu_map::Beatmap = rosu_map::from_path(path)?;
        Self::process(&map).map_err(OsuError::from)
    }
}
