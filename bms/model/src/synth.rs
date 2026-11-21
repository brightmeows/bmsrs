//! BMD synthesis parameters.
//!
//! `SynthParams` is a `KaleidBeat` extension: BMD synthesis is not part of the
//! BMS specification or beatoraja.

use std::fmt;

/// One BMD `#WAV` synthesis definition.
///
/// `KaleidBeat` extension: the parameter grammar is not part of the BMS
/// specification or beatoraja. Unknown parameters are kept in
/// [`extra`](Self::extra) so a future addition does not reject the line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SynthParams {
    /// GM program number (`P`).
    pub program: Option<u8>,
    /// Drum kit flag (`D`).
    pub drum: bool,
    /// Bank number (`B`).
    pub bank: Option<u16>,
    /// MIDI note number (`N`).
    pub note: Option<u8>,
    /// Velocity (`V`).
    pub velocity: Option<u8>,
    /// Gate length in 1/192 whole-note units (`G`).
    pub gate: Option<u32>,
    /// Pan value or curve (`A`).
    pub pan: Option<String>,
    /// Pitch-bend curve (`W`).
    pub pitch_bend: Option<String>,
    /// Volume curve (`E`).
    pub volume: Option<String>,
    /// Vibrato curve (`M`).
    pub vibrato: Option<String>,
    /// Controller values (`C`).
    pub cc: Option<String>,
    /// Unknown parameters, preserved in source order.
    pub extra: Vec<String>,
    /// Original parameter text.
    raw: String,
}

impl SynthParams {
    /// Parse a BMD `#WAV` value, or return `None` for an ordinary file name.
    ///
    /// The first token must be `P<number>` or `D`; every other prefix is
    /// ignored into [`extra`](Self::extra).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut tokens = text.split_whitespace();
        let first = tokens.next()?;
        let is_synth = first
            .strip_prefix('P')
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
            || first == "D";
        if !is_synth {
            return None;
        }
        let mut params = Self {
            raw: text.trim().to_owned(),
            ..Self::default()
        };
        for token in std::iter::once(first).chain(tokens) {
            let Some((prefix, value)) = token.split_at_checked(1) else {
                params.extra.push(token.to_owned());
                continue;
            };
            match prefix {
                "P" => params.program = value.parse().ok(),
                "D" => params.drum = true,
                "B" => params.bank = value.parse().ok(),
                "N" => params.note = value.parse().ok(),
                "V" => params.velocity = value.parse().ok(),
                "G" => params.gate = value.parse().ok(),
                "A" => params.pan = Some(value.to_owned()),
                "W" => params.pitch_bend = Some(value.to_owned()),
                "E" => params.volume = Some(value.to_owned()),
                "M" => params.vibrato = Some(value.to_owned()),
                "C" => params.cc = Some(value.to_owned()),
                _ => params.extra.push(token.to_owned()),
            }
        }
        Some(params)
    }

    /// Return the original parameter text.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for SynthParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}
