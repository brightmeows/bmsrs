//! Audio resource definition headers: `#WAV`, `#EXWAV`, `#WAVCMD`,
//! `#CDDA`, `#MIDIFILE`, `#PATH_WAV`.

use std::fmt;
use std::marker::PhantomData;

use crate::BmsStr;
use crate::index::{BmsIndex, WavTag};
use crate::{BmsHeader, BmsTokenAttr, BmsTryFromError, BmsValue};

/// Parameters for `#EXWAV{id}` — extended WAV with pan/volume/frequency
/// control (nanasi extension).
///
/// `flags` is a string of characters from `{p, v, f}` indicating which
/// sound parameters follow. The number of values equals the number of
/// flag characters.  Flag order determines value order:
///
/// ```text
/// #EXWAV01 vfp -50 100 -10000 sound.wav
///           │      │   │    │        └─ filename
///           │      │   │    └─ frequency (f): 100 Hz
///           │      │   └─ volume (v): -50 (attenuation)
///           │      └─ pan (p): -10000 (hard left)
///           └─ flags: volume, frequency, pan
/// ```
///
/// Parameter ranges:
/// - **pan** (`p`): `-10000` to `10000` (left ↔ right; default `0`).
///   Decreases the volume of one channel rather than boosting the other.
/// - **volume** (`v`): `-10000` to `0` (attenuation; `0` = original).
/// - **frequency** (`f`): `100` to `100000` Hz (pitch control).
///
/// The `#EXWAV` index shares the same namespace as `#WAV`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExWavParams<'a, C = &'a str> {
    /// Flag characters (e.g., `"pvf"`).
    pub flags: C,
    /// Parsed numeric values, one per flag character.
    pub values: Vec<f64>,
    /// Path or name of the resource file.
    pub filename: C,
    /// Phantom data to satisfy E0392 (unused lifetime parameter).
    pub _phantom: PhantomData<&'a C>,
}

impl<C: AsRef<str> + fmt::Display> fmt::Display for ExWavParams<'_, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.flags.as_ref().is_empty() {
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

impl<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a> BmsValue<'a, C>
    for ExWavParams<'a, C>
{
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
                flags: C::from(first),
                values,
                filename: C::from(filename),
                _phantom: PhantomData,
            })
        } else {
            Some(Self {
                flags: C::from(""),
                values: Vec::new(),
                filename: C::from(s.trim()),
                _phantom: PhantomData,
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
///
/// These commands define the sound files used by the chart.  WAV and OGG
/// are the most widely supported formats; MP3 introduces audible latency
/// in most players and is generally avoided.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderResDefAudio<'a, C = &'a str> {
    /// `#WAV{id}` — sound effect or BGM file definition.
    ///
    /// Referenced by channels `#xxx01` (BGM), `#xxx11-19` / `#xxx21-29`
    /// (visible notes), `#xxx31-39` / `#xxx41-49` (invisible notes),
    /// `#xxx51-69` (long notes), `#xxxD1-E9` (landmines), and others.
    ///
    /// **Multi-definition trick**: assigning the same file to multiple
    /// `#WAV` indices increases polyphony.  E.g., `#WAV01 kick.wav` and
    /// `#WAV02 kick.wav` allows two simultaneous playbacks of the same
    /// sound without one cutting off the other.
    ///
    /// `#WAV00` is special: it defines the landmine explosion sound
    /// (referenced by channels `#xxxD1-E9`).
    #[bms_token("#WAV{id} {filename}")]
    Wav {
        /// The 2-character index (e.g., `"01"`, `"2A"`).
        id: BmsIndex<WavTag>,
        /// Path or name of the resource file.
        filename: C,
    },
    /// `#EXWAV{id}` — extended WAV with pan/volume/frequency (nanasi).
    ///
    /// Shares the `#WAV` index namespace.  Only nanasi processes the
    /// effects; other players fall back to playing the file as-is.
    #[bms_token("#EXWAV{id} {params}")]
    #[bms_fallback]
    ExWav {
        /// The 2-character index.
        id: BmsIndex<WavTag>,
        /// Parsed EXWAV parameters.
        params: ExWavParams<'a, C>,
    },
    /// `#WAVCMD` — pitch/volume/duration overrides (`MacBeat` extension).
    ///
    /// Format: `commandID wavIndex value`.  Commands: `00` = pitch,
    /// `01` = volume, `02` = duration.  Only `MacBeat` processes these;
    /// Sonorous parses but ignores them.
    #[bms_token("#WAVCMD {}")]
    WavCmd(C),
    /// `#CDDA` — CD-DA track as BGM (DDR only).
    ///
    /// Specifies a CD track number to play as background music.
    #[bms_token("#CDDA {}")]
    Cdda(C),
    /// `#MIDIFILE` — MIDI file as BGM (BM98 origin).
    ///
    /// Hardware-dependent with audible latency.  Not recommended for
    /// new charts.  Supported by BM98, DDR, `IIDXv`, HDX, Sonorous.
    #[bms_token("#MIDIFILE {}")]
    Midifile(C),
    /// `#PATH_WAV` — directory prefix for audio file lookup (BMEV origin).
    ///
    /// When present, `#WAV` filenames are resolved relative to this
    /// directory.  **Should be commented out before distribution** to
    /// avoid path issues on other systems.
    #[bms_token("#PATH_WAV {}")]
    PathWav(C),
    /// Phantom data to satisfy E0392 (unused lifetime parameter).
    #[doc(hidden)]
    _Phantom(std::marker::PhantomData<&'a C>),
}

// From / TryFrom conversions

impl<'a, C: BmsStr<'a>> From<BmsHeaderResDefAudio<'a, C>> for BmsHeader<'a, C> {
    #[inline]
    fn from(audio: BmsHeaderResDefAudio<'a, C>) -> Self {
        BmsHeader::ResDefAudio(audio)
    }
}

impl<'a, C: BmsStr<'a>> TryFrom<BmsHeader<'a, C>> for BmsHeaderResDefAudio<'a, C> {
    type Error = BmsTryFromError<'a>;

    #[inline]
    fn try_from(header: BmsHeader<'a, C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::ResDefAudio(a) => Ok(a),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}
