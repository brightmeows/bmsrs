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

use bms_tokenizer::{BmpIndex, BmsChannel, BpmIndex, ScrollIndex, StopIndex, WavIndex};
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineEvent {
    /// Position within the measure.
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1–9).
    pub lane: u8,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureLength {
    /// Measure number.
    pub measure: u16,
    /// Length as a percentage (e.g. 200 = 2× the default length).
    pub length_percent: u64,
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
    #[expect(clippy::too_many_lines, reason = "finalize handles all channel types")]
    pub fn finalize(&mut self) {
        // Clear existing parse results so finalize is safe to call once.
        self.bgm_events.clear();
        self.note_events.clear();
        self.long_note_events.clear();
        self.mine_events.clear();
        self.bpm_changes.clear();
        self.stop_events.clear();
        self.scroll_events.clear();
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
                            self.push_bgm_full(line, measure, total_objects);
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
                                self.push_bpm_reference_full(&merged, measure, total_objects);
                            }
                            BmsChannel::BgaBase => {
                                self.push_bga_full(&merged, measure, BgaLayer::Base, total_objects);
                            }
                            BmsChannel::BgaPoor => {
                                self.push_bga_full(&merged, measure, BgaLayer::Poor, total_objects);
                            }
                            BmsChannel::BgaLayer => {
                                self.push_bga_full(
                                    &merged,
                                    measure,
                                    BgaLayer::Layer,
                                    total_objects,
                                );
                            }
                            BmsChannel::BgaLayer2 => {
                                self.push_bga_full(
                                    &merged,
                                    measure,
                                    BgaLayer::Layer2,
                                    total_objects,
                                );
                            }
                            BmsChannel::Stop => {
                                self.push_stop_full(&merged, measure, total_objects);
                            }
                            BmsChannel::Scroll => {
                                self.push_scroll_full(&merged, measure, total_objects);
                            }
                            BmsChannel::Note(note_ch) => {
                                if let Some(ch) = note_ch.as_u8_hex() {
                                    self.dispatch_note_channel(&merged, measure, ch, total_objects);
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
                            | BmsChannel::Speed
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

impl Messages {
    /// Parse BGM events (ch 01) from full concatenated values.
    fn push_bgm_full(&mut self, values: &str, measure: u16, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.bgm_events.push(BgmEvent {
                position: Position::new(measure, i as u32, total_objects),
                wav_id,
            });
        }
    }

    /// Parse a measure length change (ch 02) from raw value.
    fn push_measure_length(&mut self, values: &str, measure: u16) {
        // The entire value is the length percentage (e.g. "200" = 200%)
        if let Ok(pct) = values.parse::<u64>() {
            self.measure_lengths.push(MeasureLength {
                measure,
                length_percent: pct,
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
    fn push_bpm_reference_full(&mut self, values: &str, measure: u16, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(bpm_id) = val.parse::<BpmIndex>() else {
                continue;
            };
            self.bpm_changes.push(BpmChange {
                position: Position::new(measure, i as u32, total_objects),
                value: BpmValue::Reference(bpm_id),
            });
        }
    }

    /// Parse stop events (ch 09) from full concatenated values.
    fn push_stop_full(&mut self, values: &str, measure: u16, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(stop_id) = val.parse::<StopIndex>() else {
                continue;
            };
            self.stop_events.push(StopEvent {
                position: Position::new(measure, i as u32, total_objects),
                stop_id,
            });
        }
    }

    /// Parse scroll events (ch SC) from full concatenated values.
    fn push_scroll_full(&mut self, values: &str, measure: u16, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(scroll_id) = val.parse::<ScrollIndex>() else {
                continue;
            };
            self.scroll_events.push(ScrollEvent {
                position: Position::new(measure, i as u32, total_objects),
                scroll_id,
            });
        }
    }

    /// Parse BGA display events (ch 04–07) from full concatenated values.
    fn push_bga_full(&mut self, values: &str, measure: u16, layer: BgaLayer, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(bmp_id) = val.parse::<BmpIndex>() else {
                continue;
            };
            self.bga_events.push(BgaEvent {
                position: Position::new(measure, i as u32, total_objects),
                layer,
                bmp_id,
            });
        }
    }

    /// Parse playable note events (ch 11–49) from full concatenated values.
    ///
    /// Entries with `"00"` WAV index are filtered out — they represent
    /// "no note" (silent step) positions and must not produce events.
    fn push_playable_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        key_type: KeyType,
        total_objects: u32,
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
                wav_id,
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
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.long_note_events.push(LongNoteEvent {
                position: Position::new(measure, i as u32, total_objects),
                player,
                lane,
                wav_id,
            });
        }
    }

    /// Parse mine events (ch D1–E9) from full concatenated values.
    fn push_mine_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        total_objects: u32,
    ) {
        for (i, _val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // For mines, the value is the damage amount (not stored as WAV ref here)
            self.mine_events.push(MineEvent {
                position: Position::new(measure, i as u32, total_objects),
                player,
                lane,
            });
        }
    }

    /// Dispatch a note channel's raw hex value to the appropriate handler.
    ///
    /// Called from [`finalize`](Self::finalize) when a [`BmsChannel::Note`]
    /// variant has a decodable hex channel value.
    fn dispatch_note_channel(&mut self, values: &str, measure: u16, ch: u8, total_objects: u32) {
        match ch {
            0x11..=0x19 => {
                self.push_playable_full(
                    values,
                    measure,
                    1,
                    ch - 0x10,
                    KeyType::Visible,
                    total_objects,
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
                );
            }
            0x51..=0x59 => {
                self.push_long_note_full(values, measure, 1, ch - 0x50, total_objects);
            }
            0x61..=0x69 => {
                self.push_long_note_full(values, measure, 2, ch - 0x60, total_objects);
            }
            0xD1..=0xD9 => {
                self.push_mine_full(values, measure, 1, ch - 0xD0, total_objects);
            }
            0xE1..=0xE9 => {
                self.push_mine_full(values, measure, 2, ch - 0xE0, total_objects);
            }
            _ => { /* non-note hex in Note variant — kept in raw */ }
        }
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bms_tokenizer::BmsToken;
    use bms_tokenizer::BmsTokenizer;

    #[test]
    fn position_new_and_fraction() {
        let pos = Position::new(1, 1, 4);
        assert_eq!(pos.measure, 1);
        assert_eq!(pos.numer, 1);
        assert_eq!(pos.denom, 4);
        assert!((pos.fraction() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn position_denom_zero_returns_zero() {
        let pos = Position::new(0, 5, 0);
        assert!(pos.fraction().abs() < f64::EPSILON);
    }

    #[test]
    fn position_at_measure_start() {
        let pos = Position::new(2, 0, 3);
        assert!(pos.fraction().abs() < f64::EPSILON);
    }

    #[test]
    fn position_at_measure_end() {
        // 3/3 is at the measure boundary; fraction is 1.0
        let pos = Position::new(1, 3, 3);
        assert!((pos.fraction() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bgm_event_fields() {
        let pos = Position::new(1, 0, 1);
        let id: WavIndex = "01".try_into().unwrap();
        let ev = BgmEvent {
            position: pos,
            wav_id: id,
        };
        assert_eq!(ev.position, pos);
        assert_eq!(ev.wav_id, id);
    }

    #[test]
    fn note_event_visible() {
        let pos = Position::new(1, 0, 1);
        let id: WavIndex = "AA".try_into().unwrap();
        let ev = NoteEvent {
            position: pos,
            player: 1,
            lane: 3,
            key_type: KeyType::Visible,
            wav_id: id,
        };
        assert_eq!(ev.player, 1);
        assert_eq!(ev.lane, 3);
        assert!(matches!(ev.key_type, KeyType::Visible));
    }

    #[test]
    fn bpm_change_absolute() {
        let pos = Position::new(0, 0, 1);
        let ev = BpmChange {
            position: pos,
            value: BpmValue::Absolute(180.0),
        };
        assert!(
            (match ev.value {
                BpmValue::Absolute(v) => v,
                BpmValue::Reference(_) => 0.0,
            } - 180.0)
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn bpm_change_reference() {
        let pos = Position::new(0, 0, 1);
        let id: BpmIndex = "05".try_into().unwrap();
        let ev = BpmChange {
            position: pos,
            value: BpmValue::Reference(id),
        };
        assert!(matches!(ev.value, BpmValue::Reference(_)));
    }

    #[test]
    fn bga_layer_variants() {
        assert!(matches!(BgaLayer::Base, BgaLayer::Base));
        assert!(matches!(BgaLayer::Poor, BgaLayer::Poor));
        assert!(matches!(BgaLayer::Layer, BgaLayer::Layer));
        assert!(matches!(BgaLayer::Layer2, BgaLayer::Layer2));
    }

    #[test]
    fn key_type_variants() {
        assert!(matches!(KeyType::Visible, KeyType::Visible));
        assert!(matches!(KeyType::Invisible, KeyType::Invisible));
    }

    #[test]
    fn mine_event_fields() {
        let pos = Position::new(2, 1, 4);
        let ev = MineEvent {
            position: pos,
            player: 2,
            lane: 5,
        };
        assert_eq!(ev.player, 2);
        assert_eq!(ev.lane, 5);
    }

    #[test]
    fn stp_event_fields() {
        let pos = Position::new(1, 128, 1000);
        let ev = StpEvent {
            position: pos,
            duration_ms: 500.0,
        };
        assert!((ev.duration_ms - 500.0).abs() < f64::EPSILON);
        assert_eq!(ev.position.numer, 128);
        assert_eq!(ev.position.denom, 1000);
    }

    #[test]
    fn measure_length_fields() {
        let ml = MeasureLength {
            measure: 1,
            length_percent: 200,
        };
        assert_eq!(ml.measure, 1);
        assert_eq!(ml.length_percent, 200);
    }

    // Event parsing integration tests

    /// Helper: create a `BmsChannel` from a raw string.
    fn ch(s: &str) -> BmsChannel {
        BmsChannel::from_raw(s).unwrap()
    }

    /// Helper: parse a single standard message line (`#xxxYY:body`) via Messages.
    fn parse_one(line: &str) -> Messages {
        let tokens: Vec<_> = BmsTokenizer::new()
            .tokenize::<Vec<_>, &str>(line)
            .into_iter()
            .filter_map(|(_, res)| res.ok())
            .collect();
        let mut msgs = Messages::default();
        for token in &tokens {
            if let BmsToken::Message(msg) = token {
                msgs.concat_raw(msg);
            }
        }
        msgs.finalize();
        msgs
    }

    #[test]
    fn bgm_events_parsed() {
        let msgs = parse_one("#00101:AABBCC");
        assert_eq!(msgs.bgm_events.len(), 3);
        assert_eq!(msgs.bgm_events[0].wav_id, "AA".try_into().unwrap());
        assert_eq!(msgs.bgm_events[1].wav_id, "BB".try_into().unwrap());
        assert_eq!(msgs.bgm_events[2].wav_id, "CC".try_into().unwrap());
        assert_eq!(msgs.bgm_events[0].position.numer, 0);
        assert_eq!(msgs.bgm_events[0].position.denom, 3);
    }

    #[test]
    fn note_events_parsed() {
        let msgs = parse_one("#00111:1122");
        assert_eq!(msgs.note_events.len(), 2);
        assert_eq!(msgs.note_events[0].player, 1);
        assert_eq!(msgs.note_events[0].lane, 1);
        assert_eq!(msgs.note_events[0].key_type, KeyType::Visible);
        assert_eq!(msgs.note_events[1].player, 1);
        assert_eq!(msgs.note_events[1].lane, 1);
    }

    #[test]
    fn long_note_events_parsed() {
        let msgs = parse_one("#00151:0102");
        assert_eq!(msgs.long_note_events.len(), 2);
        assert_eq!(msgs.long_note_events[0].player, 1);
        assert_eq!(msgs.long_note_events[0].lane, 1);
        assert_eq!(msgs.long_note_events[1].lane, 1); // same lane, different position
        assert_eq!(msgs.long_note_events[0].position.numer, 0);
        assert_eq!(msgs.long_note_events[1].position.numer, 1);
    }

    #[test]
    fn mine_events_parsed() {
        let msgs = parse_one("#001D1:01");
        assert_eq!(msgs.mine_events.len(), 1);
        assert_eq!(msgs.mine_events[0].player, 1);
        assert_eq!(msgs.mine_events[0].lane, 1);
    }

    #[test]
    fn bpm_absolute_parsed() {
        let msgs = parse_one("#00103:7F");
        assert_eq!(msgs.bpm_changes.len(), 1);
        assert_eq!(msgs.bpm_changes[0].value, BpmValue::Absolute(127.0));
    }

    #[test]
    fn bpm_reference_parsed() {
        let msgs = parse_one("#00108:05");
        assert_eq!(msgs.bpm_changes.len(), 1);
        assert_eq!(
            msgs.bpm_changes[0].value,
            BpmValue::Reference("05".try_into().unwrap())
        );
    }

    #[test]
    fn stop_event_parsed() {
        let msgs = parse_one("#00109:01");
        assert_eq!(msgs.stop_events.len(), 1);
        assert_eq!(msgs.stop_events[0].stop_id, "01".try_into().unwrap());
    }

    #[test]
    fn scroll_event_parsed() {
        let msgs = parse_one("#000SC:ZZ");
        assert_eq!(msgs.scroll_events.len(), 1);
        assert_eq!(msgs.scroll_events[0].scroll_id, "ZZ".try_into().unwrap());
    }

    #[test]
    fn bga_event_layer2_parsed() {
        let msgs = parse_one("#0010A:03");
        assert_eq!(msgs.bga_events.len(), 1);
        assert_eq!(msgs.bga_events[0].layer, BgaLayer::Layer2);
        assert_eq!(msgs.bga_events[0].bmp_id, "03".try_into().unwrap());
    }

    #[test]
    fn bga_event_base_parsed() {
        let msgs = parse_one("#00104:03");
        assert_eq!(msgs.bga_events.len(), 1);
        assert_eq!(msgs.bga_events[0].layer, BgaLayer::Base);
        assert_eq!(msgs.bga_events[0].bmp_id, "03".try_into().unwrap());
    }

    #[test]
    fn bga_event_poor_parsed() {
        let msgs = parse_one("#00106:AA");
        assert_eq!(msgs.bga_events.len(), 1);
        assert_eq!(msgs.bga_events[0].layer, BgaLayer::Poor);
    }

    #[test]
    fn bga_event_layer_parsed() {
        let msgs = parse_one("#00107:BB");
        assert_eq!(msgs.bga_events.len(), 1);
        assert_eq!(msgs.bga_events[0].layer, BgaLayer::Layer);
    }

    #[test]
    fn measure_length_parsed() {
        let msgs = parse_one("#00102:200");
        assert_eq!(msgs.measure_lengths.len(), 1);
        assert_eq!(msgs.measure_lengths[0].measure, 1);
        assert_eq!(msgs.measure_lengths[0].length_percent, 200);
    }

    #[test]
    fn empty_values_no_events() {
        let msgs = parse_one("#00111:");
        assert_eq!(msgs.note_events.len(), 0);
        assert!(msgs.raw.contains_key(&1));
    }

    #[test]
    fn unknown_channel_raw_only() {
        let msgs = parse_one("#0010F:AA");
        assert_eq!(msgs.bgm_events.len(), 0);
        assert_eq!(msgs.note_events.len(), 0);
        let ch = BmsChannel::from_raw("0F").unwrap();
        assert_eq!(
            msgs.raw
                .get(&1)
                .and_then(|m| m.get(&ch))
                .and_then(|v| v.first().map(String::as_str)),
            Some("AA")
        );
        // Single line stored.
        assert_eq!(
            msgs.raw.get(&1).and_then(|m| m.get(&ch).map(Vec::len)),
            Some(1)
        );
    }

    #[test]
    fn note_events_filter_zero_entries() {
        // "00" entries should not produce NoteEvent.
        let msgs = parse_one("#00111:AA00BB");
        assert_eq!(msgs.note_events.len(), 2);
        assert_eq!(msgs.note_events[0].wav_id, "AA".try_into().unwrap());
        assert_eq!(msgs.note_events[1].wav_id, "BB".try_into().unwrap());
    }

    #[test]
    fn note_events_only_zero_entries_produces_nothing() {
        let msgs = parse_one("#00111:0000");
        assert_eq!(msgs.note_events.len(), 0);
    }

    #[test]
    fn two_player_notes_parsed() {
        // Channel 21 = 2P visible key 1
        let msgs = parse_one("#00121:1122");
        assert_eq!(msgs.note_events.len(), 2);
        assert_eq!(msgs.note_events[0].player, 2);
        assert_eq!(msgs.note_events[0].lane, 1);
        assert_eq!(msgs.note_events[0].key_type, KeyType::Visible);

        // Channel 41 = 2P invisible key 1
        let msgs_41 = parse_one("#00141:3344");
        assert_eq!(msgs_41.note_events.len(), 2);
        assert_eq!(msgs_41.note_events[0].player, 2);
        assert_eq!(msgs_41.note_events[0].lane, 1);
        assert_eq!(msgs_41.note_events[0].key_type, KeyType::Invisible);
    }

    // BPM 00 filtering (ch 03)

    #[test]
    fn bpm_00_rest_skipped() {
        // "00" in channel 03 = rest — no BPM change emitted.
        let msgs = parse_one("#00103:00");
        assert_eq!(msgs.bpm_changes.len(), 0);
    }

    #[test]
    fn bpm_00_only_all_filtered() {
        // All "00" values → no BPM changes.
        let msgs = parse_one("#00103:00000000");
        assert_eq!(msgs.bpm_changes.len(), 0);
    }

    #[test]
    fn bpm_00_mixed_with_real() {
        // "00" in between: 7F (127) 00 AA (170) → only 2 BPM changes.
        let msgs = parse_one("#00103:7F00AA");
        assert_eq!(msgs.bpm_changes.len(), 2);
        assert_eq!(msgs.bpm_changes[0].value, BpmValue::Absolute(127.0));
        assert_eq!(msgs.bpm_changes[1].value, BpmValue::Absolute(170.0));
    }
}
