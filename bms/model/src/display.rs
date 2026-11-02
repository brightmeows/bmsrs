//! Display and difficulty fields.
//!
//! Corresponds to the tokenizer's [`BmsHeaderDisplay`].

use bmsrs_bms_values::{BmsHeaderDisplay, DifficultyLevel};

/// Display resources and difficulty markers.
///
/// Every field uses last-wins semantics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Display {
    /// Loading screen image (`#STAGEFILE`).
    pub stage_file: Option<String>,
    /// Banner image (`#BANNER`).
    pub banner: Option<String>,
    /// Background image (`#BACKBMP`).
    pub back_bmp: Option<String>,
    /// Character animation file (`#CHARFILE`).
    pub char_file: Option<String>,
    /// Difficulty value (`#PLAYLEVEL`).
    pub play_level: Option<f64>,
    /// Difficulty category 1-5 (`#DIFFICULTY`).
    pub difficulty: Option<DifficultyLevel>,
    /// Preview audio file (`#PREVIEW`).
    pub preview: Option<String>,
}

impl Display {
    /// Apply one display header command to this structure.
    pub fn apply(&mut self, header: &BmsHeaderDisplay) {
        match header {
            BmsHeaderDisplay::StageFile(s) => self.stage_file = Some(s.clone()),
            BmsHeaderDisplay::Banner(s) => self.banner = Some(s.clone()),
            BmsHeaderDisplay::BackBmp(s) => self.back_bmp = Some(s.clone()),
            BmsHeaderDisplay::CharFile(s) => self.char_file = Some(s.clone()),
            BmsHeaderDisplay::PlayLevel(v) => self.play_level = Some(*v),
            BmsHeaderDisplay::Difficulty(d) => self.difficulty = Some(*d),
            BmsHeaderDisplay::Preview(s) => self.preview = Some(s.clone()),
        }
    }
}
