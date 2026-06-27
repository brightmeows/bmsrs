//! Structured types for BMS channel message data.
//!
//! This module defines event types extracted from `#xxxYY:values` lines
//! and the [`Messages`] container that holds both raw concatenated values
//! and parsed event vectors.

// Types are wired into Bms in Task 4; dead_code lint is suppressed
// until then.  Event types are referenced by Messages fields.
// usize→u32 truncation is safe: BMS measures have <4B values per channel.
#![allow(dead_code, reason = "wired into Bms in Task 4")]
#![allow(
    clippy::cast_possible_truncation,
    reason = "BMS measure value count fits in u32"
)]

use std::collections::BTreeMap;

use bms_tokenizer::{
    BmpIndex, BmsBase, BmsChannel, BpmIndex, ScrollIndex, SpeedIndex, StopIndex, WavIndex,
};
use bmsrs_chart::BgaLayer;

// Position

/// Position within a BMS measure, expressed as a fraction `numer / denom`.
///
/// For a channel with N values in a measure, the i-th value (0-indexed) has
/// `numer = i`, `denom = N`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Measure number (0–999).
    pub measure: u16,
    /// Numerator (index of this object within the measure).
    pub numer: u32,
    /// Denominator (total objects for this channel in this measure).
    pub denom: u32,
}

impl Position {
    /// Create a new position.
    #[inline]
    #[must_use]
    pub const fn new(measure: u16, numer: u32, denom: u32) -> Self {
        Self {
            measure,
            numer,
            denom,
        }
    }

    /// Return the fractional position within the measure as `numer / denom`.
    #[inline]
    #[must_use]
    pub fn fraction(self) -> f64 {
        if self.denom == 0 {
            0.0
        } else {
            f64::from(self.numer) / f64::from(self.denom)
        }
    }
}

// BGM

/// A BGM (background music) note — channel `01`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgmEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference into the `#WAV` table.
    pub wav_id: WavIndex,
}

// Playable notes

/// Whether a playable note is visible on the gameplay field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// Visible note (channels `11`–`19`, `21`–`29`).
    Visible,
    /// Invisible / "key" note (channels `31`–`39`, `41`–`49`).
    Invisible,
}

/// A playable note (channels `11`–`19`, `21`–`29`, `31`–`39`, `41`–`49`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEvent {
    /// Position within the measure.
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1–9).
    pub lane: u8,
    /// Whether the note is visible or invisible.
    pub key_type: KeyType,
    /// Reference into the `#WAV` table.
    pub wav_id: WavIndex,
}

// Long notes

/// A long-note / charge-note (channels `51`–`59`, `61`–`69`).
///
/// The LN type (LN / CN / HCN) is determined by the chart-level
/// [`LnType`](bms_tokenizer::LnType) and [`LnMode`](bms_tokenizer::LnMode)
/// headers, not stored per-event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongNoteEvent {
    /// Position within the measure (start of the hold).
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1–9).
    pub lane: u8,
    /// Reference into the `#WAV` table.
    pub wav_id: WavIndex,
}

// Mines

/// A landmine note (channels `D1`–`D9`, `E1`–`E9`).
///
/// The damage value is derived from the BMS 2-character index:
/// `damage = base36_value / 2.0`, with `ZZ` = instant kill.
#[derive(Debug, Clone, PartialEq)]
pub struct MineEvent {
    /// Position within the measure.
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1–9).
    pub lane: u8,
    /// Damage dealt on miss (0.0 = none, `f64::INFINITY` = instant kill).
    pub damage: f64,
}

// BPM changes

/// The value of a BPM change event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BpmValue {
    /// Absolute BPM value (channel `03`).
    Absolute(f64),
    /// Reference to a `#BPMxx` definition (channel `08`).
    Reference(BpmIndex),
}

/// A BPM change event (channels `03`, `08`).
#[derive(Debug, Clone, PartialEq)]
pub struct BpmChange {
    /// Position within the measure.
    pub position: Position,
    /// The new BPM value or reference.
    pub value: BpmValue,
}

// Stops

/// A stop / pause event (channel `09`) that references a `#STOPxx` definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference into the `#STOP` table.
    pub stop_id: StopIndex,
}

// Scroll

/// A scroll-speed multiplier event (channel `SC`).
///
/// References a `#SCROLLxx` definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference into the `#SCROLL` table.
    pub scroll_id: ScrollIndex,
}

/// A visual note-spacing keyframe event (channel `SP`).
///
/// References a `#SPEEDxx` definition.  Between keyframes the spacing
/// factor is linearly interpolated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference into the `#SPEED` table.
    pub speed_id: SpeedIndex,
}

// BGA events

/// A BGA display event (channels `04`, `05`, `06`, `07`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgaEvent {
    /// Position within the measure.
    pub position: Position,
    /// Which BGA layer this event targets.
    pub layer: BgaLayer,
    /// Reference into the `#BMP` table.
    pub bmp_id: BmpIndex,
}

// Measure length

/// A measure length change (channel `02`).
///
/// The value is a ratio relative to a standard 4/4 measure:
/// - `1.0` = 4/4 (standard)
/// - `0.75` = 3/4
/// - `2.0` = 8/4
/// - `0.015625` = 1/64
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasureLength {
    /// Measure number.
    pub measure: u16,
    /// Length ratio (1.0 = standard 4/4 measure).
    pub length_ratio: f64,
}

// Header-based position stop

/// A position-based stop event (`#STP` header).
///
/// Unlike channel `09` stops (which reference `#STOPxx` definitions),
/// `#STP` specifies an exact position and duration inline.
/// The position uses a denominator of 1000 (1/1000 of a measure).
#[derive(Debug, Clone, PartialEq)]
pub struct StpEvent {
    /// Position within the measure.
    pub position: Position,
    /// Stop duration in milliseconds.
    pub duration_ms: f64,
}

// Messages container

/// Container for raw and parsed channel message data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Messages {
    /// Raw value strings per (measure → channel), preserving individual lines.
    ///
    /// Multiple lines for the same `(measure, channel)` are stored as
    /// separate entries in the `Vec` in file order.  During [`finalize`](Self::finalize),
    /// non-BGM channels are **position-merged** (later lines overwrite earlier
    /// non-`00` positions), while BGM lines are processed independently to
    /// support polyphony.
    pub raw: BTreeMap<u16, BTreeMap<BmsChannel, Vec<String>>>,

    /// BGM events parsed from channel `01`.
    pub bgm_events: Vec<BgmEvent>,
    /// Playable note events from channels `11`–`19`, `21`–`29`, `31`–`39`, `41`–`49`.
    pub note_events: Vec<NoteEvent>,
    /// Long-note events from channels `51`–`59`, `61`–`69`.
    pub long_note_events: Vec<LongNoteEvent>,
    /// Mine events from channels `D1`–`D9`, `E1`–`E9`.
    pub mine_events: Vec<MineEvent>,
    /// BPM change events from channels `03`, `08`.
    pub bpm_changes: Vec<BpmChange>,
    /// Stop events from channel `09`.
    pub stop_events: Vec<StopEvent>,
    /// Scroll-speed events from channel `SC`.
    pub scroll_events: Vec<ScrollEvent>,
    /// Visual note-spacing keyframe events from channel `SP`.
    pub speed_events: Vec<SpeedEvent>,
    /// BGA display events from channels `04`–`07`.
    pub bga_events: Vec<BgaEvent>,
    /// Measure length changes from channel `02`.
    pub measure_lengths: Vec<MeasureLength>,
    /// Header-based position stops (`#STP`).
    pub stp_events: Vec<StpEvent>,
}

impl Messages {
    /// Append a message's values to `raw` storage.
    ///
    /// Each line for the same `(measure, channel)` is stored as a separate
    /// entry in the inner `Vec`, preserving file order.  Call
    /// [`finalize`](Self::finalize) after all messages have been received;
    /// it will position-merge non-BGM channels and then parse events.
    pub fn concat_raw<C: AsRef<str>>(&mut self, msg: &bms_tokenizer::BmsMessage<C>) {
        self.raw
            .entry(msg.track)
            .or_default()
            .entry(msg.channel)
            .or_default()
            .push(msg.body.as_ref().to_owned());
    }

    /// Finalize event parsing from raw multi-line storage.
    ///
    /// Must be called after **all** channel messages have been added via
    /// [`concat_raw`](Self::concat_raw).  For each `(measure, channel)`:
    ///
    /// - **BGM** (`ch01`): each line is processed independently, producing
    ///   events with per-line denominators (polyphony support).
    /// - **`ch02` / `chA6`** (measure length / option): only the **last**
    ///   line is used (last-wins for these scalar-like channels).
    /// - **`ch03`** (BPM change via hex): like other playable channels,
    ///   position-merged.  `"00"` entries are filtered out (rest = no BPM
    ///   change).
    /// - **All other channels**: all lines are **position-merged** via
    ///   `merge_channel` before parsing, following the spec rule that
    ///   later lines overwrite non-`00` positions while `"00"` preserves.
    ///
    /// Calling this multiple times appends duplicate events; call it exactly
    /// once after all data is loaded.
    ///
    /// `base` controls index normalisation: in standard Base36 mode indices
    /// are uppercased for case-insensitive lookup; in Base62 mode the
    /// original case is preserved.
    #[expect(clippy::too_many_lines, reason = "finalize handles all channel types")]
    pub fn finalize(&mut self, base: BmsBase) {
        // Clear existing parse results so finalize is safe to call once.
        self.bgm_events.clear();
        self.note_events.clear();
        self.long_note_events.clear();
        self.mine_events.clear();
        self.bpm_changes.clear();
        self.stop_events.clear();
        self.scroll_events.clear();
        self.speed_events.clear();
        self.bga_events.clear();
        self.measure_lengths.clear();

        let raw = std::mem::take(&mut self.raw);

        for (&measure, channels) in &raw {
            for (&channel, lines) in channels {
                match channel {
                    // BGM: each line independent (polyphony).
                    BmsChannel::Bgm => {
                        for line in lines {
                            let objects = split_2char_values_lenient(line);
                            let total_objects = objects.len() as u32;
                            self.push_bgm_full(line, measure, total_objects, base);
                        }
                    }
                    // Measure length / Option: last line wins.
                    BmsChannel::MeasureLength => {
                        if let Some(last) = lines.last() {
                            self.push_measure_length(last, measure);
                        }
                    }
                    // All other channels: position-merge, then parse events.
                    _ => {
                        let merged = if lines.len() <= 1 {
                            lines.first().cloned().unwrap_or_default()
                        } else {
                            merge_channel(lines)
                        };
                        let objects = split_2char_values_lenient(&merged);
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "BMS measure value count fits in u32"
                        )]
                        let total_objects = objects.len() as u32;

                        match channel {
                            BmsChannel::BpmChange => {
                                self.push_bpm_absolute_full(&merged, measure, total_objects);
                            }
                            BmsChannel::ExtendedBpm => {
                                self.push_bpm_reference_full(&merged, measure, total_objects, base);
                            }
                            BmsChannel::BgaBase => {
                                self.push_bga_full(
                                    &merged,
                                    measure,
                                    BgaLayer::Base,
                                    total_objects,
                                    base,
                                );
                            }
                            BmsChannel::BgaPoor => {
                                self.push_bga_full(
                                    &merged,
                                    measure,
                                    BgaLayer::Poor,
                                    total_objects,
                                    base,
                                );
                            }
                            BmsChannel::BgaLayer => {
                                self.push_bga_full(
                                    &merged,
                                    measure,
                                    BgaLayer::Layer,
                                    total_objects,
                                    base,
                                );
                            }
                            BmsChannel::BgaLayer2 => {
                                self.push_bga_full(
                                    &merged,
                                    measure,
                                    BgaLayer::Layer2,
                                    total_objects,
                                    base,
                                );
                            }
                            BmsChannel::Stop => {
                                self.push_stop_full(&merged, measure, total_objects, base);
                            }
                            BmsChannel::Scroll => {
                                self.push_scroll_full(&merged, measure, total_objects, base);
                            }
                            BmsChannel::Speed => {
                                self.push_speed_full(&merged, measure, total_objects, base);
                            }
                            BmsChannel::Note(note_ch) => {
                                if let Some(ch) = note_ch.as_u8_hex() {
                                    self.dispatch_note_channel(
                                        &merged,
                                        measure,
                                        ch,
                                        total_objects,
                                        base,
                                    );
                                }
                            }
                            // Non-event channels — kept in raw only.
                            BmsChannel::BgaBaseOpacity
                            | BmsChannel::BgaLayerOpacity
                            | BmsChannel::BgaLayer2Opacity
                            | BmsChannel::BgaPoorOpacity
                            | BmsChannel::BgmVolume
                            | BmsChannel::KeyVolume
                            | BmsChannel::Text
                            | BmsChannel::Judge
                            | BmsChannel::BgaArgbBase
                            | BmsChannel::BgaArgbLayer
                            | BmsChannel::BgaArgbLayer2
                            | BmsChannel::BgaArgbPoor
                            | BmsChannel::BgaKeyBound
                            | BmsChannel::Seek
                            | BmsChannel::Bgm
                            | BmsChannel::MeasureLength
                            | BmsChannel::Option
                            | BmsChannel::Unknown(_) => { /* kept only in raw */ }
                        }
                    }
                }
            }
        }

        self.raw = raw;
    }
}

// Channel merge (position-based, per spec)

/// Merge multiple BMS message lines for the same `(measure, channel)` using
/// position-based merge semantics:
///
/// - Lines are processed **in file order** (later line = higher priority).
/// - A non-`"00"` value at a position **overwrites** whatever was there.
/// - A `"00"` value **preserves** the existing value (no-op).
///
/// The final resolution (total object count) is the **maximum** count across
/// all lines.  Each line's values are mapped onto this grid proportionally:
/// `dest_position = src_position × max_count / line_count`.
///
/// # Panics
///
/// Panics if `lines` is empty (caller must guard).
#[expect(
    clippy::indexing_slicing,
    reason = "loop guard ensures access is within bounds"
)]
pub fn merge_channel(lines: &[String]) -> String {
    debug_assert!(!lines.is_empty(), "merge_channel called with empty lines");

    // Parse each line into 2-char value chunks.
    let parsed: Vec<Vec<String>> = lines
        .iter()
        .map(|line| {
            split_2char_values_lenient(line)
                .into_iter()
                .map(String::from)
                .collect()
        })
        .collect();

    // Find the maximum count (final resolution).
    let max_count = parsed.iter().map(Vec::len).max().unwrap_or(1);
    if max_count == 0 {
        return String::new();
    }

    // Initialise result buffer with all "00" at max resolution.
    let mut result: Vec<&str> = vec!["00"; max_count];

    // Process lines in file order.  Later lines have higher priority,
    // but "00" is a no-op (preserves existing value).
    for line_vals in &parsed {
        let line_count = line_vals.len();
        if line_count == 0 {
            continue;
        }
        // Map each position from this line's grid onto the max grid.
        for (j, val) in line_vals.iter().enumerate() {
            let dest = j * max_count / line_count;
            if val != "00" {
                // dest < max_count by construction
                result[dest] = val;
            }
            // "00" → skip (preserve existing)
        }
    }

    result.concat()
}

// Internal parsing helpers

/// Check if a byte is a valid Base62 character (0-9, A-Z, a-z).
const fn is_base62(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
}

/// Split a BMS message value string into 2-character chunks with lenient
/// parsing: invalid characters are silently skipped, and every 2 consecutive
/// valid Base62 characters form a chunk.  A trailing single valid character
/// is discarded.
///
/// This mirrors the tokenizer's former `parse_body_objects` logic, so
/// event parsing consistency is maintained regardless of which stage
/// performs the split.
#[expect(
    clippy::indexing_slicing,
    reason = "while-loop guard ensures i < len and i+1 < len before indexing"
)]
#[expect(
    clippy::expect_used,
    reason = "two base62 chars are valid ASCII by the is_base62 guard"
)]
fn split_2char_values_lenient(values: &str) -> Vec<&str> {
    let bytes = values.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    let len = bytes.len();
    while i < len {
        if is_base62(bytes[i]) && i + 1 < len && is_base62(bytes[i + 1]) {
            let chunk =
                std::str::from_utf8(&bytes[i..i + 2]).expect("two base62 chars are valid ASCII");
            result.push(chunk);
            i += 2;
        } else {
            i += 1;
        }
    }
    result
}

/// Decode a 2-character BMS landmine index into a damage value.
///
/// The index is interpreted as a Base36 value (unless the base overrides):
/// - `"00"` → `0.0` (no mine, caller should pre-filter)
/// - `"01"` → `0.5`
/// - `"ZZ"` → [`f64::INFINITY`] (instant kill per BMS spec)
/// - All other values → `base36_value / 2.0`
fn decode_mine_damage(val: &str, base: BmsBase) -> f64 {
    let parsed = if base == BmsBase::Base62 {
        val.parse::<u16>().or_else(|_| base36_decode(val))
    } else {
        let upper = val.to_ascii_uppercase();
        base36_decode(&upper)
    };
    match parsed {
        Ok(1295) => f64::INFINITY,
        Ok(n) => f64::from(n) / 2.0,
        Err(()) => 1.0,
    }
}

/// Decode a Base36 (0-9A-Z) string to a u16 value.
#[expect(
    clippy::indexing_slicing,
    reason = "guarded by bytes.len() != 2 check above"
)]
fn base36_decode(s: &str) -> Result<u16, ()> {
    let bytes = s.as_bytes();
    if bytes.len() != 2 {
        return Err(());
    }
    let hi = base36_digit(bytes[0]).ok_or(())?;
    let lo = base36_digit(bytes[1]).ok_or(())?;
    Ok(hi * 36 + lo)
}

/// Decode a single Base36 digit (0-9, A-Z, case-insensitive).
fn base36_digit(b: u8) -> Option<u16> {
    match b {
        b'0'..=b'9' => Some(u16::from(b - b'0')),
        b'A'..=b'Z' => Some(u16::from(b - b'A') + 10),
        b'a'..=b'z' => Some(u16::from(b - b'a') + 10),
        _ => None,
    }
}

impl Messages {
    /// Parse BGM events (ch 01) from full concatenated values.
    fn push_bgm_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.bgm_events.push(BgmEvent {
                position: Position::new(measure, i as u32, total_objects),
                wav_id: WavIndex::from(wav_id.normalize(base)),
            });
        }
    }

    /// Parse a measure length change (ch 02) from raw value.
    ///
    /// The value is a ratio relative to 4/4: `1.0` = 4/4, `0.75` = 3/4,
    /// `2.0` = 8/4.  Non-numeric or zero values are silently ignored
    /// (the measure defaults to 4/4).
    fn push_measure_length(&mut self, values: &str, measure: u16) {
        if let Ok(ratio) = values.trim().parse::<f64>()
            && ratio > 0.0
            && ratio.is_finite()
        {
            self.measure_lengths.push(MeasureLength {
                measure,
                length_ratio: ratio,
            });
        }
    }

    /// Parse absolute BPM changes (ch 03) from hex values.
    ///
    /// Values of `"00"` represent a rest (no BPM change) and are **skipped**
    /// per BMS spec — passing `BpmValue::Absolute(0.0)` downstream would
    /// cause division-by-zero in the timing track.
    fn push_bpm_absolute_full(&mut self, values: &str, measure: u16, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // "00" = rest / no BPM change — skip entirely.
            if val == "00" {
                continue;
            }
            // Channel 03 values are hex integers (01-FF)
            let Ok(bpm_val) = u8::from_str_radix(val, 16) else {
                continue;
            };
            self.bpm_changes.push(BpmChange {
                position: Position::new(measure, i as u32, total_objects),
                value: BpmValue::Absolute(f64::from(bpm_val)),
            });
        }
    }

    /// Parse BPM reference changes (ch 08) from full concatenated values.
    fn push_bpm_reference_full(
        &mut self,
        values: &str,
        measure: u16,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(bpm_id) = val.parse::<BpmIndex>() else {
                continue;
            };
            self.bpm_changes.push(BpmChange {
                position: Position::new(measure, i as u32, total_objects),
                value: BpmValue::Reference(BpmIndex::from(bpm_id.normalize(base))),
            });
        }
    }

    /// Parse stop events (ch 09) from full concatenated values.
    fn push_stop_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(stop_id) = val.parse::<StopIndex>() else {
                continue;
            };
            self.stop_events.push(StopEvent {
                position: Position::new(measure, i as u32, total_objects),
                stop_id: StopIndex::from(stop_id.normalize(base)),
            });
        }
    }

    /// Parse scroll events (ch SC) from full concatenated values.
    fn push_scroll_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(scroll_id) = val.parse::<ScrollIndex>() else {
                continue;
            };
            self.scroll_events.push(ScrollEvent {
                position: Position::new(measure, i as u32, total_objects),
                scroll_id: ScrollIndex::from(scroll_id.normalize(base)),
            });
        }
    }

    /// Parse speed keyframe events (ch SP) from full concatenated values.
    fn push_speed_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(speed_id) = val.parse::<SpeedIndex>() else {
                continue;
            };
            self.speed_events.push(SpeedEvent {
                position: Position::new(measure, i as u32, total_objects),
                speed_id: SpeedIndex::from(speed_id.normalize(base)),
            });
        }
    }

    /// Parse BGA display events (ch 04–07) from full concatenated values.
    fn push_bga_full(
        &mut self,
        values: &str,
        measure: u16,
        layer: BgaLayer,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(bmp_id) = val.parse::<BmpIndex>() else {
                continue;
            };
            self.bga_events.push(BgaEvent {
                position: Position::new(measure, i as u32, total_objects),
                layer,
                bmp_id: BmpIndex::from(bmp_id.normalize(base)),
            });
        }
    }

    /// Parse playable note events (ch 11–49) from full concatenated values.
    ///
    /// Entries with `"00"` WAV index are filtered out — they represent
    /// "no note" (silent step) positions and must not produce events.
    #[expect(
        clippy::too_many_arguments,
        reason = "BMS event parsing requires channel context: measure, player, lane, type, plus base for normalization"
    )]
    fn push_playable_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        key_type: KeyType,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // "00" = no note — skip entirely. This must happen before
            // parsing since "00" is a valid index but semantically
            // means "no object at this position" in all BMS channels.
            if val == "00" {
                continue;
            }
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.note_events.push(NoteEvent {
                position: Position::new(measure, i as u32, total_objects),
                player,
                lane,
                key_type,
                wav_id: WavIndex::from(wav_id.normalize(base)),
            });
        }
    }

    /// Parse long-note events (ch 51–69) from full concatenated values.
    fn push_long_note_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.long_note_events.push(LongNoteEvent {
                position: Position::new(measure, i as u32, total_objects),
                player,
                lane,
                wav_id: WavIndex::from(wav_id.normalize(base)),
            });
        }
    }

    /// Parse mine events (ch D1–E9) from full concatenated values.
    ///
    /// The 2-character index encodes damage as a Base36 value:
    /// `damage = value / 2.0`, with `ZZ` (1295) = instant kill.
    /// Entries with `"00"` are filtered out (no mine at that position).
    fn push_mine_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // "00" = no mine — skip entirely.
            if val == "00" {
                continue;
            }
            let damage = decode_mine_damage(val, base);
            self.mine_events.push(MineEvent {
                position: Position::new(measure, i as u32, total_objects),
                player,
                lane,
                damage,
            });
        }
    }

    /// Dispatch a note channel's raw hex value to the appropriate handler.
    ///
    /// Called from [`finalize`](Self::finalize) when a [`BmsChannel::Note`]
    /// variant has a decodable hex channel value.
    fn dispatch_note_channel(
        &mut self,
        values: &str,
        measure: u16,
        ch: u8,
        total_objects: u32,
        base: BmsBase,
    ) {
        match ch {
            0x11..=0x19 => {
                self.push_playable_full(
                    values,
                    measure,
                    1,
                    ch - 0x10,
                    KeyType::Visible,
                    total_objects,
                    base,
                );
            }
            0x21..=0x29 => {
                self.push_playable_full(
                    values,
                    measure,
                    2,
                    ch - 0x20,
                    KeyType::Visible,
                    total_objects,
                    base,
                );
            }
            0x31..=0x39 => {
                self.push_playable_full(
                    values,
                    measure,
                    1,
                    ch - 0x30,
                    KeyType::Invisible,
                    total_objects,
                    base,
                );
            }
            0x41..=0x49 => {
                self.push_playable_full(
                    values,
                    measure,
                    2,
                    ch - 0x40,
                    KeyType::Invisible,
                    total_objects,
                    base,
                );
            }
            0x51..=0x59 => {
                self.push_long_note_full(values, measure, 1, ch - 0x50, total_objects, base);
            }
            0x61..=0x69 => {
                self.push_long_note_full(values, measure, 2, ch - 0x60, total_objects, base);
            }
            0xD1..=0xD9 => {
                self.push_mine_full(values, measure, 1, ch - 0xD0, total_objects, base);
            }
            0xE1..=0xE9 => {
                self.push_mine_full(values, measure, 2, ch - 0xE0, total_objects, base);
            }
            _ => { /* non-note hex in Note variant — kept in raw */ }
        }
    }
}

// Tests
