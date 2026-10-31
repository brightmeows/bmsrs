//! Audio resource definition headers: `#WAV`, `#EXWAV`, `#WAVCMD`, `#CDDA`,
//! `#MIDIFILE`, and `#PATH_WAV`.

use std::fmt;

use crate::index::WavIndex;
use crate::{BmsTokenAttr, BmsValue};

/// Extended audio effect parameters for `#EXWAV{id}` (nanasi extension).
///
/// Unspecified parameters are `None`; players apply their own defaults.
///
/// Parameter ranges (the BMS specification defines integers):
/// - **pan** (`p`): `[-10000, 10000]`, default `0`. Far left is -10000, far
///   right is 10000.
/// - **volume** (`v`): `[-10000, 0]`, default `0` (original volume). Reduces
///   the volume of one channel instead of boosting the other.
/// - **frequency** (`f`): `[100, 100000]` Hz (pitch control).
///
/// `#EXWAV` indices share the same namespace as `#WAV`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExWavParams {
    /// Pan (`p` flag). Range: `[-10000, 10000]`; `0` means center.
    pub pan: Option<i32>,
    /// Volume attenuation (`v` flag). Range: `[-10000, 0]`; `0` means original
    /// volume.
    pub volume: Option<i32>,
    /// Frequency (`f` flag). Range: `[100, 100000]` Hz.
    pub frequency: Option<u32>,
    /// Resource file path or name.
    pub filename: String,
}

impl fmt::Display for ExWavParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Emit present flags in the fixed order pan, volume, frequency.
        let mut flags = String::new();
        let mut values: Vec<String> = Vec::new();
        if let Some(p) = self.pan {
            flags.push('p');
            values.push(p.to_string());
        }
        if let Some(v) = self.volume {
            flags.push('v');
            values.push(v.to_string());
        }
        if let Some(fr) = self.frequency {
            flags.push('f');
            values.push(fr.to_string());
        }
        if flags.is_empty() {
            write!(f, "{}", self.filename)
        } else {
            write!(f, "{flags}")?;
            for v in &values {
                write!(f, " {v}")?;
            }
            write!(f, " {}", self.filename)
        }
    }
}

impl BmsValue for ExWavParams {
    fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split_whitespace();
        let first = parts.next()?;

        let is_flags = !first.is_empty()
            && first.chars().all(|c| matches!(c, 'p' | 'v' | 'f'))
            && first.len() <= 3;

        if is_flags {
            let mut pan = None;
            let mut volume = None;
            let mut frequency = None;
            // Track consumed bytes to keep the zero-copy path (filename slices
            // borrow `s`).
            let mut consumed = first.len();
            for flag in first.chars() {
                // Skip leading whitespace to locate the next numeric value.
                while s
                    .as_bytes()
                    .get(consumed)
                    .is_some_and(u8::is_ascii_whitespace)
                {
                    consumed += 1;
                }
                let val_start = consumed;
                while s
                    .as_bytes()
                    .get(consumed)
                    .is_some_and(|b| !b.is_ascii_whitespace())
                {
                    consumed += 1;
                }
                let val_str = s.get(val_start..consumed)?;
                let val: i64 = val_str.parse().ok()?;
                match flag {
                    'p' => {
                        let v: i32 = val.try_into().ok()?;
                        if !(-10000..=10000).contains(&v) {
                            return None;
                        }
                        pan = Some(v);
                    }
                    'v' => {
                        let v: i32 = val.try_into().ok()?;
                        if !(-10000..=0).contains(&v) {
                            return None;
                        }
                        volume = Some(v);
                    }
                    'f' => {
                        let v: u32 = val.try_into().ok()?;
                        if !(100..=100_000).contains(&v) {
                            return None;
                        }
                        frequency = Some(v);
                    }
                    _ => return None,
                }
            }
            // filename is the remainder after the last value (may contain
            // spaces).
            while s
                .as_bytes()
                .get(consumed)
                .is_some_and(u8::is_ascii_whitespace)
            {
                consumed += 1;
            }
            let filename = s.get(consumed..).filter(|f| !f.is_empty())?;
            Some(Self {
                pan,
                volume,
                frequency,
                filename: filename.to_owned(),
            })
        } else {
            // No flags: the whole string is the filename.
            Some(Self {
                pan: None,
                volume: None,
                frequency: None,
                filename: s.trim().to_owned(),
            })
        }
    }
}

/// Command kind for `#WAVCMD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavCmdKind {
    /// `00`: pitch (MIDI note number, where 60 is middle C). Range: `[0, 127]`.
    Pitch,
    /// `01`: volume (percentage; 100 means original).
    Volume,
    /// `02`: playback duration (half-millisecond units).
    Time,
}

impl std::fmt::Display for WavCmdKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pitch => "00",
            Self::Volume => "01",
            Self::Time => "02",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for WavCmdKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "00" => Ok(Self::Pitch),
            "01" => Ok(Self::Volume),
            "02" => Ok(Self::Time),
            _ => Err(()),
        }
    }
}

/// Parsed parameters for `#WAVCMD`: pitch, volume, or duration overrides
/// (`MacBeat` extension).
///
/// Format: `commandID wavIndex value`. Commands: `00` = pitch, `01` = volume,
/// `02` = duration. Only `MacBeat` handles these; Sonorous parses but ignores
/// them.
///
/// A parse failure falls back to
/// [`BmsHeaderFallback`](crate::BmsHeaderFallback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavCmdParams {
    /// Command kind.
    pub command: WavCmdKind,
    /// Target WAV index.
    pub wav_index: WavIndex,
    /// Parameter value (non-negative integer; semantics depend on `command`).
    ///
    /// - `Pitch`: MIDI note number, range `[0, 127]`.
    /// - `Volume`: percentage; `100` means original. Values above `100` may
    ///   clip.
    /// - `Time`: half-millisecond units. Values below `50` may be unreliable.
    pub value: u32,
}

impl std::fmt::Display for WavCmdParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.command, self.wav_index, self.value)
    }
}

impl std::str::FromStr for WavCmdParams {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = (|| {
            let mut parts = s.split_whitespace();
            let command: WavCmdKind = parts.next()?.parse().ok()?;
            let wav_index: WavIndex = parts.next()?.parse().ok()?;
            let value: u32 = parts.next()?.parse().ok()?;
            if parts.next().is_some() {
                return None;
            }
            // Pitch command (00) range: MIDI note number [0, 127].
            if command == WavCmdKind::Pitch && value > 127 {
                return None;
            }
            Some(Self {
                command,
                wav_index,
                value,
            })
        })();
        parsed.ok_or(())
    }
}

/// Audio resource definition headers.
///
/// These commands define the audio files used by a chart. WAV and OGG are the
/// most widely supported formats; MP3 introduces noticeable latency in most
/// players and is usually avoided.
#[derive(Debug, Clone, PartialEq, Eq, BmsTokenAttr)]
pub enum BmsHeaderResDefAudio {
    /// `#WAV{id}`: sound-effect or BGM file definition.
    ///
    /// Referenced by channels `#xxx01` (BGM), `#xxx11-19` / `#xxx21-29` (visible
    /// notes), `#xxx31-39` / `#xxx41-49` (invisible notes), `#xxx51-69` (long
    /// notes), `#xxxD1-E9` (mines), and so on.
    ///
    /// **Multiple-definition trick**: assigning the same file to several `#WAV`
    /// indices increases polyphony. For example, `#WAV01 kick.wav` and
    /// `#WAV02 kick.wav` let the same sound play twice at once without cutting
    /// each other off.
    ///
    /// `#WAV00` is special: it defines the mine explosion sound (referenced by
    /// channels `#xxxD1-E9`).
    #[bms_token("#WAV{id} {filename}")]
    Wav {
        /// 2-character index (for example, `"01"`, `"2A"`).
        id: WavIndex,
        /// Resource file path or name.
        filename: String,
    },
    /// `#EXWAV{id}`: extended WAV with pan, volume, and frequency control
    /// (nanasi).
    ///
    /// Shares the `#WAV` index namespace. Only nanasi applies the effects;
    /// other players fall back to playing the file as-is.
    #[bms_token("#EXWAV{id} {params}")]
    #[bms_fallback]
    ExWav {
        /// 2-character index.
        id: WavIndex,
        /// Parsed EXWAV parameters.
        params: ExWavParams,
    },
    /// `#WAVCMD`: pitch, volume, or duration overrides (`MacBeat` extension).
    ///
    /// Format: `commandID wavIndex value`. Commands: `00` = pitch, `01` =
    /// volume, `02` = duration. Only `MacBeat` handles these; Sonorous parses
    /// but ignores them.
    ///
    /// A parse failure falls back to
    /// [`BmsHeaderFallback`](crate::BmsHeaderFallback).
    #[bms_token("#WAVCMD {params}")]
    #[bms_fallback]
    WavCmd {
        /// Parsed WAVCMD parameters.
        params: WavCmdParams,
    },
    /// `#CDDA`: CD-DA audio track as BGM (DDR only).
    ///
    /// Specifies a CD track number to play as background music.
    #[bms_token("#CDDA {}")]
    Cdda(String),
    /// `#MIDIFILE`: MIDI file as BGM (BM98 origin).
    ///
    /// Hardware-dependent and noticeably latent. Not recommended for new
    /// charts. Supported by BM98, DDR, `IIDXv`, HDX, and Sonorous.
    #[bms_token("#MIDIFILE {}")]
    Midifile(String),
    /// `#PATH_WAV`: directory prefix for audio file lookup (BMEV origin).
    ///
    /// When present, `#WAV` filenames are resolved relative to this directory.
    /// **Comment it out before distribution** to avoid path problems on other
    /// systems.
    #[bms_token("#PATH_WAV {}")]
    PathWav(String),
}
