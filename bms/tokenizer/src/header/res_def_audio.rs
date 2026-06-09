//! `#wav` and related audio resource definitions.

use std::fmt;

use crate::id::{BmsChannelId, WavTag};
use crate::{BmsTokenAttr, BmsValue};

/// Parameters for `#EXWAV{id}` — extended WAV with pan/volume/frequency control.
///
/// `flags` is a string of characters from `{p, v, f}` indicating which
/// sound parameters follow. The number of values equals the number of
/// flag characters.
#[derive(Debug, Clone, PartialEq)]
pub struct ExWavParams<'a> {
    /// Flag characters (e.g., `"pvf"`).
    pub flags: &'a str,
    /// Parsed numeric values, one per flag character.
    pub values: Vec<f64>,
    /// Path or name of the resource file.
    pub filename: &'a str,
}

impl fmt::Display for ExWavParams<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.flags.is_empty() {
            write!(f, "{}", self.filename)
        } else {
            write!(f, "{}", self.flags)?;
            for val in &self.values {
                write!(f, " {val}")?;
            }
            write!(f, " {}", self.filename)
        }
    }
}

impl<'a> BmsValue<'a> for ExWavParams<'a> {
    fn parse(s: &'a str) -> Option<Self> {
        let mut parts = s.split_whitespace();
        let first = parts.next()?;

        let is_flags = !first.is_empty() && first.chars().all(|c| matches!(c, 'p' | 'v' | 'f'));

        if is_flags {
            let flag_count = first.len();
            // Collect exactly `flag_count` numeric values after the flags.
            let values: Vec<f64> = parts
                .take(flag_count)
                .map(|p| p.parse().ok())
                .collect::<Option<_>>()?;
            if values.len() != flag_count {
                return None;
            }
            let filename = nth_whitespace_field_rest(s, flag_count + 1);
            Some(Self {
                flags: first,
                values,
                filename,
            })
        } else {
            Some(Self {
                flags: "",
                values: Vec::new(),
                filename: s.trim(),
            })
        }
    }
}

/// Return the substring of `s` starting at the Nth whitespace-separated field
/// (0-indexed), trimmed of leading whitespace.
fn nth_whitespace_field_rest(s: &str, n: usize) -> &str {
    let mut start = 0;
    let mut field = 0;
    let bytes = s.as_bytes();
    while start < bytes.len() && field < n {
        while start < bytes.len() && bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
            start += 1;
        }
        if start >= bytes.len() {
            break;
        }
        while start < bytes.len() && bytes.get(start).is_some_and(|b| !b.is_ascii_whitespace()) {
            start += 1;
        }
        field += 1;
    }
    while start < bytes.len() && bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    s.get(start..).unwrap_or("")
}

/// Audio resource definition headers.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefAudio<'a> {
    /// `#WAV{id}` — a sound effect definition.
    #[bms_token("#WAV{id} {filename}")]
    Wav {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        id: BmsChannelId<WavTag>,
        /// Path or name of the resource file.
        filename: &'a str,
    },
    /// `#EXWAV{id}` — extended WAV with pan/volume/frequency.
    #[bms_token("#EXWAV{id} {params}")]
    #[bms_fallback]
    ExWav {
        /// The 2-character index.
        id: BmsChannelId<WavTag>,
        /// Parsed EXWAV parameters.
        params: ExWavParams<'a>,
    },
    /// `#WAVCMD`
    #[bms_token("#WAVCMD {value}")]
    WavCmd(&'a str),
    /// `#CDDA`
    #[bms_token("#CDDA {value}")]
    Cdda(&'a str),
    /// `#MIDIFILE`
    #[bms_token("#MIDIFILE {value}")]
    Midifile(&'a str),
    /// `#PATH_WAV`
    #[bms_token("#PATH_WAV {value}")]
    PathWav(&'a str),
}
