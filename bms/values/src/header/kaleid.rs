//! `KaleidBeat` declaration headers.
//!
//! These commands are `KaleidBeat` extensions: none of them are defined by the
//! BMS specification or beatoraja. They declare chart-local assets and playback
//! hints that stay in the format sidecar instead of the format-neutral `Chart`.

use crate::BmsTokenAttr;

/// `KaleidBeat` BGA overflow mode (`#BGAOVERFLOW`).
///
/// `KaleidBeat` extension: values are `0` (off), `1` (on), and `Y` (on, with
/// upward overflow). Omitting the header lets the player auto-detect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum BgaOverflow {
    /// `0`: keep the BGA inside its frame.
    #[bms_token("0")]
    Off,
    /// `1`: draw the BGA outside its frame.
    #[bms_token("1")]
    On,
    /// `Y`: draw outside the frame and allow upward overflow.
    #[bms_token("Y")]
    #[bms_token("y")]
    Up,
}

/// `KaleidBeat` song-sound playback mode (`#SONGSOUNDMODE`).
///
/// `KaleidBeat` extension: controls whether chart-defined song sounds replace
/// or mix with the player's own sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BmsTokenAttr)]
pub enum SongSoundMode {
    /// `EXCLUSIVE`: replace the player's sound (the default).
    #[bms_token("EXCLUSIVE")]
    #[bms_token("exclusive")]
    #[bms_token("Exclusive")]
    Exclusive,
    /// `MIX`: play both the chart sound and the player's sound.
    #[bms_token("MIX")]
    #[bms_token("mix")]
    #[bms_token("Mix")]
    Mix,
}

/// `KaleidBeat` declaration headers that do not fit an existing BMS domain.
///
/// `KaleidBeat` extension: every variant is chart-local metadata for the
/// `KaleidBeat` player and is not defined by the BMS specification or
/// beatoraja.
#[derive(Debug, Clone, PartialEq, Eq, BmsTokenAttr)]
pub enum BmsHeaderKaleid {
    /// `#LOADINGFILE`: replacement image shown while the chart loads.
    #[bms_token("#LOADINGFILE {}")]
    LoadingFile(String),
    /// `#READYFILE`: replacement image shown from READY onward.
    #[bms_token("#READYFILE {}")]
    ReadyFile(String),
    /// `#BGAOVERFLOW`: whether BGA may draw outside its frame.
    #[bms_token("#BGAOVERFLOW {}")]
    BgaOverflow(BgaOverflow),
    /// `#TARGETHASH`: SHA-256 of the base chart a differential patch targets.
    #[bms_token("#TARGETHASH {}")]
    TargetHash(String),
    /// `#SOUNDFONT`: `SoundFont` file used by `BMD` synthesis.
    #[bms_token("#SOUNDFONT {}")]
    SoundFont(String),
    /// `#DECIDEFILE`: replacement sound for the song-select confirm action.
    #[bms_token("#DECIDEFILE {}")]
    DecideFile(String),
    /// `#PLAYSTOPFILE`: replacement sound for a hard-gauge close.
    #[bms_token("#PLAYSTOPFILE {}")]
    PlayStopFile(String),
    /// `#RESULTFILE`: replacement result-screen sound.
    #[bms_token("#RESULTFILE {}")]
    ResultFile(String),
    /// `#RESULTCLEARFILE`: result sound played on clear.
    #[bms_token("#RESULTCLEARFILE {}")]
    ResultClearFile(String),
    /// `#RESULTFAILFILE`: result sound played on failure.
    #[bms_token("#RESULTFAILFILE {}")]
    ResultFailFile(String),
    /// `#RESULTFIRSTFILE`: result sound played only on the first play.
    #[bms_token("#RESULTFIRSTFILE {}")]
    ResultFirstFile(String),
    /// `#SONGSOUNDMODE`: replace or mix chart-defined song sounds.
    #[bms_token("#SONGSOUNDMODE {}")]
    SongSoundMode(SongSoundMode),
}
