//! Display and difficulty fields.
//!
//! Corresponds to [`BmsHeaderDisplay`] from the tokenizer.

use bms_tokenizer::{BmsHeaderDisplay, BmsStr, DifficultyLevel};

/// Display assets and difficulty markers.
///
/// All fields use last-wins semantics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Display {
    /// Splash-screen image (`#STAGEFILE`).
    pub stage_file: Option<String>,
    /// Banner image (`#BANNER`).
    pub banner: Option<String>,
    /// Background image (`#BACKBMP`).
    pub back_bmp: Option<String>,
    /// Character animation file (`#CHARFILE`).
    pub char_file: Option<String>,
    /// Difficulty number (`#PLAYLEVEL`).
    pub play_level: Option<f64>,
    /// Difficulty category 1–5 (`#DIFFICULTY`).
    pub difficulty: Option<DifficultyLevel>,
    /// Preview audio file (`#PREVIEW`).
    pub preview: Option<String>,
}

impl Display {
    /// Apply a display header to this struct.
    pub fn apply<'a, C: BmsStr<'a>>(&mut self, header: &BmsHeaderDisplay<'a, C>) {
        match header {
            BmsHeaderDisplay::StageFile(s) => self.stage_file = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::Banner(s) => self.banner = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::BackBmp(s) => self.back_bmp = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::CharFile(s) => self.char_file = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::PlayLevel(v) => self.play_level = Some(*v),
            BmsHeaderDisplay::Difficulty(d) => self.difficulty = Some(*d),
            BmsHeaderDisplay::Preview(s) => self.preview = Some(s.as_ref().to_owned()),
            BmsHeaderDisplay::_Phantom(_) => unreachable!(),
        }
    }
}
