//! osu!mania 的每音符扩展数据。

use std::num::NonZeroU32;

use bmsrs_chart::NoteExt;

/// osu! 的 hit sample 名称。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OsuHitSampleName {
    /// 默认 normal 音。
    Normal,
    /// 默认 whistle 音。
    Whistle,
    /// 默认 finish 音。
    Finish,
    /// 默认 clap 音。
    Clap,
    /// 谱面目录中的自定义文件名。
    File(String),
}

/// osu! 的 sample bank。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OsuSampleBank {
    /// 未指定 bank。
    #[default]
    None,
    /// normal bank。
    Normal,
    /// soft bank。
    Soft,
    /// drum bank。
    Drum,
}

/// osu! 音符携带的一个 hit sample。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OsuHitSample {
    /// sample 名称；自定义文件使用 [`OsuHitSampleName::File`]。
    pub name: OsuHitSampleName,
    /// sample bank。
    pub bank: OsuSampleBank,
    /// 自定义 sample 后缀；`None` 表示无后缀。
    pub suffix: Option<NonZeroU32>,
    /// 音量（0 到 100）。
    pub volume: u8,
    /// 自定义 sample bank 编号。
    pub custom_sample_bank: u16,
    /// 是否显式指定了 bank。
    pub bank_specified: bool,
    /// 是否需要与 normal 音叠加播放。
    pub is_layered: bool,
}

/// osu!mania 的每音符扩展数据。
///
/// 当前只携带 hit sample；列位置由 [`NoteSide`](bmsrs_chart::NoteSide)
/// 与 [`Lane`](bmsrs_chart::Lane) 表达，长音时长由
/// [`NoteKind::Long`](bmsrs_chart::NoteKind::Long) 表达。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OsuNoteExt {
    /// 此音符触发的全部 hit sample。
    pub hit_samples: Vec<OsuHitSample>,
}

impl NoteExt for OsuNoteExt {}
