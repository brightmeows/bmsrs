//! 显示与难度字段。
//!
//! 对应分词器的 [`BmsHeaderDisplay`]。

use bms_tokenizer::{BmsHeaderDisplay, DifficultyLevel};

/// 显示资源与难度标记。
///
/// 所有字段均使用最后胜出（last-wins）语义。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Display {
    /// 开场画面图片（`#STAGEFILE`）。
    pub stage_file: Option<String>,
    /// 横幅图片（`#BANNER`）。
    pub banner: Option<String>,
    /// 背景图片（`#BACKBMP`）。
    pub back_bmp: Option<String>,
    /// 角色动画文件（`#CHARFILE`）。
    pub char_file: Option<String>,
    /// 难度数值（`#PLAYLEVEL`）。
    pub play_level: Option<f64>,
    /// 难度类别 1–5（`#DIFFICULTY`）。
    pub difficulty: Option<DifficultyLevel>,
    /// 预览音频文件（`#PREVIEW`）。
    pub preview: Option<String>,
}

impl Display {
    /// 将一个显示头部命令应用到此结构体。
    pub fn apply<C: AsRef<str>>(&mut self, header: &BmsHeaderDisplay<C>) {
        match header {
            BmsHeaderDisplay::StageFile(s) => self.stage_file = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::Banner(s) => self.banner = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::BackBmp(s) => self.back_bmp = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::CharFile(s) => self.char_file = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::PlayLevel(v) => self.play_level = Some(*v),
            BmsHeaderDisplay::Difficulty(d) => self.difficulty = Some(*d),
            BmsHeaderDisplay::Preview(s) => self.preview = Some(s.as_ref().to_owned()),
        }
    }
}
