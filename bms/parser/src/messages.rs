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

use bms_tokenizer::{Base62, BmpTag, BmsIndex, BpmTag, ChannelTag, ScrollTag, StopTag, WavTag};

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
#[derive(Debug, Clone, PartialEq)]
pub struct BgmEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference into the `#WAV` table.
    pub wav_id: BmsIndex<WavTag>,
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
#[derive(Debug, Clone, PartialEq)]
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
    pub wav_id: BmsIndex<WavTag>,
}

// Long notes

/// A long-note / charge-note (channels `51`–`59`, `61`–`69`).
///
/// The LN type (LN / CN / HCN) is determined by the chart-level
/// [`LnType`](bms_tokenizer::LnType) and [`LnMode`](bms_tokenizer::LnMode)
/// headers, not stored per-event.
#[derive(Debug, Clone, PartialEq)]
pub struct LongNoteEvent {
    /// Position within the measure (start of the hold).
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1–9).
    pub lane: u8,
    /// Reference into the `#WAV` table.
    pub wav_id: BmsIndex<WavTag>,
}

// Mines

/// A landmine note (channels `D1`–`D9`, `E1`–`E9`).
#[derive(Debug, Clone, PartialEq)]
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
    Reference(BmsIndex<BpmTag>),
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
#[derive(Debug, Clone, PartialEq)]
pub struct StopEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference into the `#STOP` table.
    pub stop_id: BmsIndex<StopTag>,
}

// Scroll

/// A scroll-speed multiplier event (channel `0A`).
///
/// References a `#SCROLLxx` definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference into the `#SCROLL` table.
    pub scroll_id: BmsIndex<ScrollTag>,
}

// BGA events

/// The BGA layer targeted by a [`BgaEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgaLayer {
    /// Base layer (channel `04`).
    Base,
    /// Poor / miss layer (channels `05`, `06`).
    Poor,
    /// Overlay layer (channel `07`).
    Layer,
}

/// A BGA display event (channels `04`, `05`, `06`, `07`).
#[derive(Debug, Clone, PartialEq)]
pub struct BgaEvent {
    /// Position within the measure.
    pub position: Position,
    /// Which BGA layer this event targets.
    pub layer: BgaLayer,
    /// Reference into the `#BMP` table.
    pub bmp_id: BmsIndex<BmpTag>,
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
    /// Raw concatenated value strings per (measure → channel).
    ///
    /// Multiple lines for the same `(measure, channel)` are **concatenated**
    /// in file order — unlike the old last-wins behaviour.
    pub raw: BTreeMap<u16, BTreeMap<BmsIndex<ChannelTag, Base62>, String>>,

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
    /// Scroll-speed events from channel `0A`.
    pub scroll_events: Vec<ScrollEvent>,
    /// BGA display events from channels `04`–`07`.
    pub bga_events: Vec<BgaEvent>,
    /// Measure length changes from channel `02`.
    pub measure_lengths: Vec<MeasureLength>,
    /// Header-based position stops (`#STP`).
    pub stp_events: Vec<StpEvent>,
}

impl Messages {
    /// Concatenate a message's values into `raw` and parse them into typed
    /// events.
    ///
    /// Unlike the old last-wins semantics, values from consecutive lines
    /// with the same `(measure, channel)` are **appended**.
    pub fn concat_and_parse(&mut self, msg: &bms_tokenizer::BmsMessage<'_>) {
        // Step 1: append value to raw storage
        self.raw
            .entry(msg.track)
            .or_default()
            .entry(msg.channel)
            .and_modify(|existing| existing.push_str(msg.body))
            .or_insert_with(|| msg.body.to_owned());

        // Step 2: read back the concatenated value for event parsing.
        // This ensures multi-line concat produces correct position numer/denom.
        let Some(ch) = msg.channel.as_u8_hex() else {
            return;
        };
        let track = msg.track;

        // Clone to release the borrow on self.raw before calling push_* methods.
        let Some(concat_values) = self
            .raw
            .get(&track)
            .and_then(|m| m.get(&msg.channel))
            .cloned()
        else {
            return;
        };

        // Early exit for empty values
        if concat_values.is_empty() {
            return;
        }

        // Compute object offset and total count for correct position numer/denom.
        // We parse the current line's body (msg.body) but use the concatenated
        // total count so that multi-line channel data produces non-overlapping
        // positions.  E.g. two lines "AABB" + "CCDD" produce positions
        // (0/4, 1/4) and (2/4, 3/4) rather than (0/2, 1/2) and (0/2, 1/2).
        let current_count = split_2char_values_lenient(msg.body).len() as u32;
        let total_count = split_2char_values_lenient(&concat_values).len() as u32;
        let offset = total_count - current_count;

        match ch {
            // BGM
            0x01 => self.push_bgm(msg.body, track, offset, total_count),
            // Measure length
            0x02 => self.push_measure_length(msg.body, track),
            // BPM absolute (hex)
            0x03 => self.push_bpm_absolute(msg.body, track, offset, total_count),
            // BGA events
            0x04 => self.push_bga(msg.body, track, BgaLayer::Base, offset, total_count),
            0x05 | 0x06 => self.push_bga(msg.body, track, BgaLayer::Poor, offset, total_count),
            0x07 => self.push_bga(msg.body, track, BgaLayer::Layer, offset, total_count),
            // BPM reference
            0x08 => self.push_bpm_reference(msg.body, track, offset, total_count),
            // Stop
            0x09 => self.push_stop(msg.body, track, offset, total_count),
            // Scroll
            0x0A => self.push_scroll(msg.body, track, offset, total_count),
            // 1P visible notes
            0x11..=0x19 => self.push_playable(
                msg.body,
                track,
                1,
                ch - 0x10,
                KeyType::Visible,
                offset,
                total_count,
            ),
            // 2P visible notes
            0x21..=0x29 => self.push_playable(
                msg.body,
                track,
                2,
                ch - 0x20,
                KeyType::Visible,
                offset,
                total_count,
            ),
            // 1P invisible notes
            0x31..=0x39 => self.push_playable(
                msg.body,
                track,
                1,
                ch - 0x30,
                KeyType::Invisible,
                offset,
                total_count,
            ),
            // 2P invisible notes
            0x41..=0x49 => self.push_playable(
                msg.body,
                track,
                2,
                ch - 0x40,
                KeyType::Invisible,
                offset,
                total_count,
            ),
            // 1P long notes
            0x51..=0x59 => self.push_long_note(msg.body, track, 1, ch - 0x50, offset, total_count),
            // 2P long notes
            0x61..=0x69 => self.push_long_note(msg.body, track, 2, ch - 0x60, offset, total_count),
            // 1P mines
            0xD1..=0xD9 => self.push_mine(msg.body, track, 1, ch - 0xD0, offset, total_count),
            // 2P mines
            0xE1..=0xE9 => self.push_mine(msg.body, track, 2, ch - 0xE0, offset, total_count),
            _ => { /* unknown channel — kept only in raw */ }
        }
    }
}

// Internal parsing helpers

/// Check if a byte is a valid Base62 character (0-9, A-Z, a-z).
fn is_base62(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
}

/// Split a BMS message value string into 2-character chunks with lenient
/// parsing: invalid characters are silently skipped, and every 2 consecutive
/// valid Base62 characters form a chunk.  A trailing single valid character
/// is discarded.
///
/// This matches the behaviour of the tokenizer's `parse_body_objects`, so
/// event positions are consistent regardless of whether the split happens
/// at the tokenizer or parser level.
#[expect(
    clippy::indexing_slicing,
    reason = "while-loop guard ensures i < len and i+1 < len before indexing"
)]
fn split_2char_values_lenient(values: &str) -> Vec<&str> {
    let bytes = values.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    let len = bytes.len();
    while i < len {
        if is_base62(bytes[i]) && i + 1 < len && is_base62(bytes[i + 1]) {
            // SAFETY: two valid Base62 chars are always valid ASCII.
            let chunk = unsafe { std::str::from_utf8_unchecked(&bytes[i..i + 2]) };
            result.push(chunk);
            i += 2;
        } else {
            i += 1;
        }
    }
    result
}

impl Messages {
    /// Parse BGM events (ch 01) from raw values.
    fn push_bgm(&mut self, values: &str, measure: u16, object_offset: u32, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = BmsIndex::try_from(val) else {
                continue;
            };
            self.bgm_events.push(BgmEvent {
                position: Position::new(measure, object_offset + i as u32, total_objects),
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
    fn push_bpm_absolute(
        &mut self,
        values: &str,
        measure: u16,
        object_offset: u32,
        total_objects: u32,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // Channel 03 values are hex integers (00-FF)
            let Ok(bpm_val) = u8::from_str_radix(val, 16) else {
                continue;
            };
            self.bpm_changes.push(BpmChange {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                value: BpmValue::Absolute(f64::from(bpm_val)),
            });
        }
    }

    /// Parse BPM reference changes (ch 08) from raw values.
    fn push_bpm_reference(
        &mut self,
        values: &str,
        measure: u16,
        object_offset: u32,
        total_objects: u32,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(bpm_id) = BmsIndex::try_from(val) else {
                continue;
            };
            self.bpm_changes.push(BpmChange {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                value: BpmValue::Reference(bpm_id),
            });
        }
    }

    /// Parse stop events (ch 09) from raw values.
    fn push_stop(&mut self, values: &str, measure: u16, object_offset: u32, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(stop_id) = BmsIndex::try_from(val) else {
                continue;
            };
            self.stop_events.push(StopEvent {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                stop_id,
            });
        }
    }

    /// Parse scroll events (ch 0A) from raw values.
    fn push_scroll(&mut self, values: &str, measure: u16, object_offset: u32, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(scroll_id) = BmsIndex::try_from(val) else {
                continue;
            };
            self.scroll_events.push(ScrollEvent {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                scroll_id,
            });
        }
    }

    /// Parse BGA display events (ch 04–07) from raw values.
    fn push_bga(
        &mut self,
        values: &str,
        measure: u16,
        layer: BgaLayer,
        object_offset: u32,
        total_objects: u32,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(bmp_id) = BmsIndex::try_from(val) else {
                continue;
            };
            self.bga_events.push(BgaEvent {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                layer,
                bmp_id,
            });
        }
    }

    /// Parse playable note events (ch 11–49) from raw values.
    #[expect(clippy::too_many_arguments, reason = "BMS event fields")]
    fn push_playable(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        key_type: KeyType,
        object_offset: u32,
        total_objects: u32,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = BmsIndex::try_from(val) else {
                continue;
            };
            self.note_events.push(NoteEvent {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                player,
                lane,
                key_type,
                wav_id,
            });
        }
    }

    /// Parse long-note events (ch 51–69) from raw values.
    fn push_long_note(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        object_offset: u32,
        total_objects: u32,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = BmsIndex::try_from(val) else {
                continue;
            };
            self.long_note_events.push(LongNoteEvent {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                player,
                lane,
                wav_id,
            });
        }
    }

    /// Parse mine events (ch D1–E9) from raw values.
    fn push_mine(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        object_offset: u32,
        total_objects: u32,
    ) {
        for (i, _val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // For mines, the value is the damage amount (not stored as WAV ref here)
            self.mine_events.push(MineEvent {
                position: Position::new(measure, object_offset + i as u32, total_objects),
                player,
                lane,
            });
        }
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bms_tokenizer::{BmsMessage, BmsObjectId};

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
        let id: BmsIndex<WavTag> = "01".try_into().unwrap();
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
        let id: BmsIndex<WavTag> = "AA".try_into().unwrap();
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
        let id: BmsIndex<BpmTag> = "05".try_into().unwrap();
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

    /// Helper: create a channel ID from a Base62 string.
    fn ch(s: &str) -> BmsIndex<ChannelTag, Base62> {
        s.try_into().unwrap()
    }

    /// Helper: parse a single standard message line (`#xxxYY:body`) via Messages.
    fn parse_one(line: &str) -> Messages {
        let mut msgs = Messages::default();
        let content = line.strip_prefix('#').unwrap_or(line);
        let colon_pos = content.find(':').unwrap_or(content.len());
        let addr = &content[..colon_pos];
        let body = &content[colon_pos.saturating_add(1)..];

        // For standard test lines like #00101:AABB, addr is "00101".
        // Channel = last 2 chars, track = chars before that.
        let channel_str = &addr[addr.len().saturating_sub(2)..];
        let channel: BmsIndex<ChannelTag, Base62> = channel_str.try_into().unwrap();
        let track: u16 = addr[..addr.len().saturating_sub(2)]
            .chars()
            .filter(char::is_ascii_digit)
            .fold(0u16, |acc, c| {
                acc.saturating_mul(10)
                    .saturating_add(u16::from(c as u8 - b'0'))
            });
        let objects: Vec<BmsObjectId> = body
            .as_bytes()
            .chunks(2)
            .filter(|c| c.len() == 2)
            .filter_map(|c| std::str::from_utf8(c).ok().and_then(|s| s.try_into().ok()))
            .collect();
        let msg = BmsMessage {
            addr,
            body,
            track,
            channel,
            objects,
        };
        msgs.concat_and_parse(&msg);
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
        let msgs = parse_one("#0010A:ZZ");
        assert_eq!(msgs.scroll_events.len(), 1);
        assert_eq!(msgs.scroll_events[0].scroll_id, "ZZ".try_into().unwrap());
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
        let ch: BmsIndex<ChannelTag, Base62> = "0F".try_into().unwrap();
        assert_eq!(
            msgs.raw
                .get(&1)
                .and_then(|m| m.get(&ch))
                .map(String::as_str),
            Some("AA")
        );
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
        let msgs = parse_one("#00141:3344");
        assert_eq!(msgs.note_events.len(), 2);
        assert_eq!(msgs.note_events[0].player, 2);
        assert_eq!(msgs.note_events[0].lane, 1);
        assert_eq!(msgs.note_events[0].key_type, KeyType::Invisible);
    }
}
