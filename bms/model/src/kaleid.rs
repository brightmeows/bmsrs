//! `KaleidBeat` declaration fields.
//!
//! Every field in this module is a `KaleidBeat` extension: none of these
//! commands are defined by the BMS specification or beatoraja. The model keeps
//! them out of the standard header groups so ordinary charts stay unchanged.

use bmsrs_bms_values::{BgaOverflow, BmsHeaderKaleid, SongSoundMode};

/// `KaleidBeat` chart-local declarations.
///
/// All fields use last-wins semantics. The structure is empty for a chart that
/// does not use the dialect.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kaleid {
    /// Loading-screen image (`#LOADINGFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub loading_file: Option<String>,
    /// READY-screen image (`#READYFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub ready_file: Option<String>,
    /// BGA overflow mode (`#BGAOVERFLOW`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub bga_overflow: Option<BgaOverflow>,
    /// Differential-patch target hash (`#TARGETHASH`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub target_hash: Option<String>,
    /// `SoundFont` used by `BMD` synthesis (`#SOUNDFONT`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub sound_font: Option<String>,
    /// Song-select confirm sound (`#DECIDEFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub decide_file: Option<String>,
    /// Hard-gauge close sound (`#PLAYSTOPFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub play_stop_file: Option<String>,
    /// Generic result-screen sound (`#RESULTFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub result_file: Option<String>,
    /// Result sound played on clear (`#RESULTCLEARFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub result_clear_file: Option<String>,
    /// Result sound played on failure (`#RESULTFAILFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub result_fail_file: Option<String>,
    /// Result sound played only on the first play (`#RESULTFIRSTFILE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub result_first_file: Option<String>,
    /// Song-sound playback mode (`#SONGSOUNDMODE`).
    ///
    /// `KaleidBeat` extension: not defined by the BMS specification or
    /// beatoraja.
    pub song_sound_mode: Option<SongSoundMode>,
}

impl Kaleid {
    /// Apply one `KaleidBeat` declaration header.
    pub fn apply(&mut self, header: &BmsHeaderKaleid) {
        match header {
            BmsHeaderKaleid::LoadingFile(value) => self.loading_file = Some(value.clone()),
            BmsHeaderKaleid::ReadyFile(value) => self.ready_file = Some(value.clone()),
            BmsHeaderKaleid::BgaOverflow(value) => self.bga_overflow = Some(*value),
            BmsHeaderKaleid::TargetHash(value) => self.target_hash = Some(value.clone()),
            BmsHeaderKaleid::SoundFont(value) => self.sound_font = Some(value.clone()),
            BmsHeaderKaleid::DecideFile(value) => self.decide_file = Some(value.clone()),
            BmsHeaderKaleid::PlayStopFile(value) => self.play_stop_file = Some(value.clone()),
            BmsHeaderKaleid::ResultFile(value) => self.result_file = Some(value.clone()),
            BmsHeaderKaleid::ResultClearFile(value) => self.result_clear_file = Some(value.clone()),
            BmsHeaderKaleid::ResultFailFile(value) => self.result_fail_file = Some(value.clone()),
            BmsHeaderKaleid::ResultFirstFile(value) => {
                self.result_first_file = Some(value.clone());
            }
            BmsHeaderKaleid::SongSoundMode(value) => self.song_sound_mode = Some(*value),
        }
    }

    /// Return whether every declaration is absent.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}
