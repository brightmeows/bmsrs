//! Structured types for BMS channel message data.
//!
//! This module defines the event types extracted from `#xxxYY:values` lines and
//! the [`Messages`] container that holds both the raw concatenated values and
//! the parsed event vectors.

// The usize to u32 truncation is safe: BMS has fewer than 4B values per channel
// per measure. See the event_pos helper.

use std::collections::BTreeMap;

use crate::warnings::ChannelValueWarning;
use bmsrs_bms_values::{
    BmpIndex, BmsBase, BmsChannel, BmsIndex, BodyCells, BpmIndex, ChannelIndex, ScrollIndex,
    SpeedIndex, StopIndex, WavIndex, base36_digit_value,
};

// Position

/// A position within a BMS measure, represented by the fraction
/// `numer / denom`.
///
/// For a channel with N values in a measure, the i-th value (zero-based)
/// corresponds to `numer = i` and `denom = N`.
///
/// The fields are private and can only be constructed through
/// [`Position::new`], which guarantees the `denom > 0` invariant at the type
/// level so external code cannot bypass it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Measure number (0-999).
    measure: u16,
    /// Numerator (the object's index within the measure).
    numer: u32,
    /// Denominator (the total number of objects for this channel in this
    /// measure).
    denom: u32,
}

impl Position {
    /// Create a new position.
    ///
    /// # Panics
    ///
    /// Panics when `denom == 0`; a position with a zero denominator is
    /// meaningless.
    #[inline]
    #[must_use]
    pub fn new(measure: u16, numer: u32, denom: u32) -> Self {
        assert!(denom > 0, "Position denom must be non-zero");
        Self {
            measure,
            numer,
            denom,
        }
    }

    /// Return the measure number (0-999).
    #[inline]
    #[must_use]
    pub const fn measure(self) -> u16 {
        self.measure
    }

    /// Return the numerator (the object's index within the measure).
    #[inline]
    #[must_use]
    pub const fn numer(self) -> u32 {
        self.numer
    }

    /// Return the denominator (the total number of objects for this channel in
    /// this measure).
    ///
    /// Guaranteed to be greater than 0; validated by [`Position::new`] at
    /// construction.
    #[inline]
    #[must_use]
    pub const fn denom(self) -> u32 {
        self.denom
    }

    /// Return the fractional position within the measure, `numer / denom`.
    #[inline]
    #[must_use]
    pub fn fraction(self) -> f64 {
        f64::from(self.numer) / f64::from(self.denom)
    }
}

// BGM

/// A BGM (background music) note on channel `01`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgmEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference to the `#WAV` table.
    pub wav_id: WavIndex,
}

// Playable notes

/// Whether a playable note is visible in the play area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// Visible note (channels `11`-`19`, `21`-`29`).
    Visible,
    /// Invisible or "key" note (channels `31`-`39`, `41`-`49`).
    Invisible,
}

/// A playable note (channels `11`-`1Z`, `21`-`2Z`, `31`-`3Z`, `41`-`4Z`).
///
/// Extended channels (`1A`-`1Z` and so on, pomu2 family) decode to `lane`
/// 10-35.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEvent {
    /// Position within the measure.
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1-9 standard; 10-35 for extended channels).
    pub lane: u8,
    /// Whether the note is visible or invisible.
    pub key_type: KeyType,
    /// Reference to the `#WAV` table.
    pub wav_id: WavIndex,
}

// Long notes

/// A long note or charge note (channels `51`-`5Z`, `61`-`6Z`).
///
/// Extended channels (`5A`-`5Z` and so on, pomu2 family) decode to `lane`
/// 10-35.
///
/// The long-note type (LN / CN / HCN) is determined by the chart-level
/// [`LnType`](bmsrs_bms_values::LnType) and [`LnMode`](bmsrs_bms_values::LnMode) header
/// commands, not stored per event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongNoteEvent {
    /// Position within the measure (the start of the hold).
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1-9 standard; 10-35 for extended channels).
    pub lane: u8,
    /// Reference to the `#WAV` table.
    pub wav_id: WavIndex,
}

// Mines

/// A mine note (channels `D1`-`D9`, `E1`-`E9`).
///
/// `raw_value` is the Base36 decoding of the mine channel's 2-character value.
/// Damage calculation (`base36 / 2.0`, `ZZ` means instant death) belongs to the
/// processor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineEvent {
    /// Position within the measure.
    pub position: Position,
    /// Player number (1 or 2).
    pub player: u8,
    /// Lane / key number (1-9).
    pub lane: u8,
    /// Base36-decoded value (`Some(1295)` means `"ZZ"`, `None` means decoding
    /// failed).
    pub raw_value: Option<u16>,
}

// BPM changes

/// Value of a BPM change event.
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
    /// New BPM value or reference.
    pub value: BpmValue,
}

// Stops

/// A stop or pause event referencing a `#STOPxx` definition (channel `09`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference to the `#STOP` table.
    pub stop_id: StopIndex,
}

// Scroll

/// A scroll speed multiplier event (channel `SC`).
///
/// References a `#SCROLLxx` definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference to the `#SCROLL` table.
    pub scroll_id: ScrollIndex,
}

/// A visual note spacing keyframe event (channel `SP`).
///
/// References a `#SPEEDxx` definition. Spacing factors between keyframes are
/// computed by linear interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedEvent {
    /// Position within the measure.
    pub position: Position,
    /// Reference to the `#SPEED` table.
    pub speed_id: SpeedIndex,
}

// BGA events

/// BMS BGA layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BmsBgaLayer {
    /// Base / main background (channel `04`).
    Base,
    /// Miss / POOR presentation layer (channel `06`).
    Poor,
    /// Overlay layer on top of the main BGA (channel `07`).
    Layer,
    /// Second overlay layer (channel `0A`).
    Layer2,
}

/// A BGA display event (channels `04`, `06`, `07`, `0A`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgaEvent {
    /// Position within the measure.
    pub position: Position,
    /// BGA layer targeted by this event.
    pub layer: BmsBgaLayer,
    /// Reference to the `#BMP` table.
    pub bmp_id: BmpIndex,
}

// Measure length

/// A measure length change (channel `02`).
///
/// The value is a ratio relative to a standard 4/4 measure:
/// - `1.0` means 4/4 (standard)
/// - `0.75` means 3/4
/// - `2.0` means 8/4
/// - `0.015625` means 1/64
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasureLength {
    /// Measure number.
    pub measure: u16,
    /// Length ratio (1.0 is a standard 4/4 measure).
    pub length_ratio: f64,
}

// Position-based stop from a header

/// A position-based stop event (the `#STP` header command).
///
/// Unlike channel `09` stops that reference `#STOPxx` definitions, `#STP`
/// specifies the exact position and duration inline. Positions use 1000 as the
/// denominator (1/1000 of a measure).
#[derive(Debug, Clone, PartialEq)]
pub struct StpEvent {
    /// Position within the measure.
    pub position: Position,
    /// Stop duration in milliseconds.
    pub duration_ms: f64,
}

/// Merged data for non-event channels (used only for deferred parsing of BMS
/// engine-specific events).
///
/// Filled during finalization when a non-event channel is matched; the
/// processor (`bmsrs-bms-processor`) reads this data and converts it to
/// `EventKind::Custom`.
///
/// **Normalization guarantee**: every 2-character index in `values` has already
/// been normalized with `base` (uppercase in Base36 mode, original case
/// preserved in Base62 mode). The processor does not need to normalize again
/// when looking up definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEventData {
    /// Measure number.
    pub measure: u16,
    /// Channel type.
    pub channel: BmsChannel,
    /// List of 2-character indices normalized with `base` (`"00"` is kept for
    /// position calculation).
    pub values: Vec<BmsIndex>,
}

// Message container

/// Container for raw and parsed channel message data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Messages {
    /// Raw value strings stored by (measure, channel), preserving each line
    /// separately.
    ///
    /// Multiple lines for the same `(measure, channel)` are stored as separate
    /// entries in a `Vec`, in file order. During [`finalize`](Self::finalize),
    /// non-BGM channels are **merged by position** (later lines override
    /// earlier non-`00` positions), while BGM lines are processed independently
    /// to support polyphony.
    pub raw: BTreeMap<u16, BTreeMap<BmsChannel, Vec<String>>>,

    /// BGM events parsed from channel `01`.
    pub bgm_events: Vec<BgmEvent>,
    /// Playable note events parsed from channels `11`-`1Z`, `21`-`2Z`,
    /// `31`-`3Z`, and `41`-`4Z` (including pomu2-family extended channels such
    /// as `1A`-`1Z`, with lanes 10-35).
    pub note_events: Vec<NoteEvent>,
    /// Long-note events parsed from channels `51`-`5Z` and `61`-`6Z` (including
    /// extended channels such as `5A`-`5Z`, with lanes 10-35).
    pub long_note_events: Vec<LongNoteEvent>,
    /// Mine events parsed from channels `D1`-`D9` and `E1`-`E9`.
    pub mine_events: Vec<MineEvent>,
    /// BPM change events parsed from channels `03` and `08`.
    pub bpm_changes: Vec<BpmChange>,
    /// Stop events parsed from channel `09`.
    pub stop_events: Vec<StopEvent>,
    /// Scroll speed events parsed from channel `SC`.
    pub scroll_events: Vec<ScrollEvent>,
    /// Visual note spacing keyframe events parsed from channel `SP`.
    pub speed_events: Vec<SpeedEvent>,
    /// BGA display events parsed from channels `04`-`07`.
    pub bga_events: Vec<BgaEvent>,
    /// Measure length changes parsed from channel `02`.
    pub measure_lengths: Vec<MeasureLength>,
    /// Position-based stops from the `#STP` header command.
    pub stp_events: Vec<StpEvent>,

    /// Merged data for non-event channels (used only for deferred parsing of
    /// BMS engine-specific events).
    ///
    /// Each entry contains the measure number, channel type, and a list of
    /// normalized single values; `finalize_merged` fills it when a non-event
    /// channel is matched. The processor (`bmsrs-bms-processor`) reads this data and
    /// converts it to `EventKind::Custom`. Every 2-character index has been
    /// normalized with `base`.
    ///
    pub non_event_data: Vec<NonEventData>,

    /// Channel value parse warnings (collected during finalize; cleared first on
    /// repeated calls).
    ///
    /// 2-character values whose shape is valid but do not satisfy that channel's
    /// parse rule (for example, ch03 requires hexadecimal and the value contains
    /// a non-hexadecimal character) are skipped and recorded here.
    /// Missing-reference warnings (undefined WAV/BPM/STOP keys) belong to the
    /// processor layer's `ProcessWarning`.
    pub warnings: Vec<ChannelValueWarning>,
}

/// Convert the enumerated index `i` into a [`Position`], centralizing the
/// `usize` to `u32` truncation expectation.
///
/// The number of values per channel per measure (object lines times values per
/// line) is far below `u32::MAX` in real charts, so truncation is safe.
#[expect(
    clippy::cast_possible_truncation,
    reason = "BMS per-channel per-measure value count fits in u32"
)]
fn event_pos(i: usize, measure: u16, denom: u32) -> Position {
    Position::new(measure, i as u32, denom)
}

impl Messages {
    /// Append one message's value to the `raw` store.
    ///
    /// Each line for the same `(measure, channel)` is stored as a separate entry
    /// in the inner `Vec`, preserving file order. Call
    /// [`finalize`](Self::finalize) after receiving every message; it merges
    /// non-BGM channels by position and then parses events.
    pub fn concat_raw(&mut self, msg: &bmsrs_bms_values::BmsMessage) {
        self.raw
            .entry(msg.track())
            .or_default()
            .entry(msg.channel())
            .or_default()
            .push(msg.body.clone());
    }

    /// Finalize event parsing from the raw multi-line store.
    ///
    /// Must be called after **all** channel messages have been added through
    /// [`concat_raw`](Self::concat_raw). For each `(measure, channel)`:
    ///
    /// - **BGM** (`ch01`): each line is processed independently, producing
    ///   events with that line's denominator (supports polyphony).
    /// - **`ch02`** (measure length): only the **last** line is used (last-wins
    ///   scalar-like channel).
    /// - **`ch03`** (BPM change via hexadecimal): merged by position like other
    ///   playable channels. `"00"` entries are filtered out (a rest means no BPM
    ///   change).
    /// - **Every other channel** (including `chA6` options): all lines are
    ///   **merged by position** through `merge_channel` before parsing,
    ///   following the specification rule that later lines override non-`"00"`
    ///   positions while `"00"` preserves them.
    ///
    /// Existing parse results are cleared before re-parsing, so repeated calls
    /// are safe.
    ///
    /// `base` controls index normalization: in standard Base36 mode indices are
    /// uppercased for case-insensitive lookup; in Base62 mode the original case
    /// is preserved.
    pub fn finalize(&mut self, base: BmsBase) {
        // Take raw and header-sourced stp_events, then reset message parse
        // results so finalize can be called repeatedly.
        let raw = std::mem::take(&mut self.raw);
        let stp_events = std::mem::take(&mut self.stp_events);
        *self = Self::default();

        for (&measure, channels) in &raw {
            for (&channel, lines) in channels {
                self.finalize_channel(channel, lines, measure, base);
            }
        }

        self.raw = raw;
        self.stp_events = stp_events;
    }

    /// Dispatch one channel's multi-line data to the corresponding finalizer.
    fn finalize_channel(
        &mut self,
        channel: BmsChannel,
        lines: &[String],
        measure: u16,
        base: BmsBase,
    ) {
        match channel {
            // BGM: each line independently (polyphony).
            BmsChannel::Bgm => self.finalize_bgm_lines(lines, measure, base),
            // Measure length: last line wins.
            BmsChannel::MeasureLength => self.finalize_measure_length_line(lines, measure),
            // Every other channel: merge by position, then parse events.
            _ => self.finalize_merged(channel, lines, measure, base),
        }
    }

    /// Finalize the BGM channel: process each line independently (polyphony).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "BMS measure value count fits in u32"
    )]
    fn finalize_bgm_lines(&mut self, lines: &[String], measure: u16, base: BmsBase) {
        for line in lines {
            let total_objects = BodyCells::including_zero(line).total() as u32;
            let context = ChannelContext::new(measure, total_objects, base, BmsChannel::Bgm);
            self.push_bgm_full(line, context);
        }
    }

    /// Finalize the measure length channel: use only the last line.
    fn finalize_measure_length_line(&mut self, lines: &[String], measure: u16) {
        if let Some(last) = lines.last() {
            self.push_measure_length(last, measure);
        }
    }

    /// Finalize other channels: merge by position, then dispatch by event type.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "BMS measure value count fits in u32"
    )]
    fn finalize_merged(
        &mut self,
        channel: BmsChannel,
        lines: &[String],
        measure: u16,
        base: BmsBase,
    ) {
        let merged = match lines {
            [] => String::new(),
            [line] => line.clone(),
            _ => Self::merge_bodies(lines),
        };
        let total_objects = BodyCells::including_zero(&merged).total() as u32;
        let context = ChannelContext::new(measure, total_objects, base, channel);

        match channel {
            BmsChannel::BpmChange => self.push_bpm_absolute_full(&merged, context),
            BmsChannel::ExtendedBpm => self.push_bpm_reference_full(&merged, context),
            BmsChannel::BgaBase => self.push_bga_full(&merged, BmsBgaLayer::Base, context),
            BmsChannel::BgaPoor => self.push_bga_full(&merged, BmsBgaLayer::Poor, context),
            BmsChannel::BgaLayer => self.push_bga_full(&merged, BmsBgaLayer::Layer, context),
            BmsChannel::BgaLayer2 => self.push_bga_full(&merged, BmsBgaLayer::Layer2, context),
            BmsChannel::Stop => self.push_stop_full(&merged, context),
            BmsChannel::Scroll => self.push_scroll_full(&merged, context),
            BmsChannel::Speed => self.push_speed_full(&merged, context),
            BmsChannel::Note(note_ch) => self.dispatch_note_channel(&merged, note_ch, context),
            // Non-event channels: keep normalized data for the processor to
            // convert into BmsCustomEvent. Every 2-character value is
            // normalized by base here, eliminating duplicate normalization in
            // the processor (F1).
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
            | BmsChannel::Option
            | BmsChannel::Unknown(_) => {
                let values = normalize_merged_values(&merged, base);
                self.non_event_data.push(NonEventData {
                    measure,
                    channel,
                    values,
                });
            }
            // BGM and MeasureLength are unreachable here (handled by early
            // branches in finalize_channel); the arms remain for exhaustiveness.
            BmsChannel::Bgm | BmsChannel::MeasureLength => {}
        }
    }
}

impl Messages {
    /// Merge multiple BMS message lines for the same `(measure, channel)` using
    /// position-merge semantics.
    ///
    /// - Lines are processed in **file order** (later lines have higher
    ///   priority).
    /// - A non-`"00"` value at a position **overrides** the existing value.
    /// - A `"00"` value **preserves** the existing value (no-op).
    ///
    /// The final resolution (total object count) is the **maximum** count across
    /// lines. Each line's values are mapped proportionally onto that grid:
    /// `dest_position = src_position * max_count / line_count`.
    ///
    /// Empty input returns an empty string.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "loop guard ensures access is within bounds"
    )]
    pub fn merge_bodies(lines: &[String]) -> String {
        if lines.is_empty() {
            return String::new();
        }

        // Parse each line into 2-character value chunks.
        let parsed: Vec<Vec<&str>> = lines
            .iter()
            .map(|line| {
                BodyCells::including_zero(line)
                    .map(|cell| cell.text)
                    .collect()
            })
            .collect();

        // Find the maximum count (final resolution).
        let max_count = parsed.iter().map(Vec::len).max().unwrap_or(1);
        if max_count == 0 {
            return String::new();
        }

        // Initialize the result buffer at the maximum resolution, all "00".
        let mut result: Vec<&str> = vec!["00"; max_count];

        // Process lines in file order. Later lines take priority, but "00" is a
        // no-op (keeps the existing value).
        for line_vals in &parsed {
            let line_count = line_vals.len();
            if line_count == 0 {
                continue;
            }
            // Map each position on this line's grid to the maximum grid.
            for (j, val) in line_vals.iter().enumerate() {
                let dest = j * max_count / line_count;
                if *val != "00" {
                    // dest < max_count is guaranteed by construction.
                    result[dest] = *val;
                }
                // "00" means skip (keep the existing value).
            }
        }

        result.concat()
    }
}

/// Normalize a merged channel value string into a list of 2-character indices
/// according to `base`.
///
/// Every 2-character chunk parsed by [`BodyCells::including_zero`] is
/// constructed through [`BmsIndex::try_from`] and then normalized with
/// [`BmsIndex::normalize`] for the given base.
///
/// `BodyCells` guarantees 2-character Base62 chunks, so
/// [`BmsIndex::try_from`] always succeeds for them; `filter_map` is a
/// conservative guard that should never filter a value in theory.
fn normalize_merged_values(merged: &str, base: BmsBase) -> Vec<BmsIndex> {
    BodyCells::including_zero(merged)
        .filter_map(|cell| {
            BmsIndex::try_from(cell.text)
                .ok()
                .map(|idx| idx.normalize(base))
        })
        .collect()
}

// Internal parse helpers

/// Decode a mine channel's 2-character value into a Base36 number.
///
/// Mine damage values are always Base36 and are not affected by `#BASE 62` (see
/// the BMS Base62 specification, `bms/ext/base62-format.md` lines 127-129).
///
/// `"00"` has already been filtered out by [`BodyCells::non_zero`] upstream and
/// never reaches this function. `None` means the raw value could not be decoded;
/// the processor uses the default damage value.
fn decode_mine_raw(val: &str) -> Option<u16> {
    let upper = val.to_ascii_uppercase();
    BmsBase::Base36.decode(&upper)
}

/// Shared context for parsing one (measure, channel) body.
#[derive(Clone, Copy, Debug)]
struct ChannelContext {
    /// Measure number.
    measure: u16,
    /// Total number of cells in the body.
    total_objects: u32,
    /// Radix base used for index normalization.
    base: BmsBase,
    /// Original channel type, used for warnings.
    channel: BmsChannel,
}

impl ChannelContext {
    /// Construct the context.
    const fn new(measure: u16, total_objects: u32, base: BmsBase, channel: BmsChannel) -> Self {
        Self {
            measure,
            total_objects,
            base,
            channel,
        }
    }
}

/// Feed body cells to `make` one by one; failures are recorded as warnings.
fn parse_indexed<T>(
    context: ChannelContext,
    cells: BodyCells<'_>,
    out: &mut Vec<T>,
    warnings: &mut Vec<ChannelValueWarning>,
    mut make: impl FnMut(Position, &str) -> Option<T>,
) {
    for cell in cells {
        let position = event_pos(cell.index, context.measure, context.total_objects);
        match make(position, cell.text) {
            Some(value) => out.push(value),
            None => warnings.push(ChannelValueWarning {
                measure: context.measure,
                channel: context.channel,
                value: cell.text.to_owned(),
            }),
        }
    }
}

impl Messages {
    /// Parse BGM events from the full concatenated values (ch 01).
    fn push_bgm_full(&mut self, values: &str, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::including_zero(values),
            &mut self.bgm_events,
            &mut self.warnings,
            |position, value| {
                let wav_id = value.parse::<WavIndex>().ok()?;
                Some(BgmEvent {
                    position,
                    wav_id: WavIndex::from(wav_id.normalize(context.base)),
                })
            },
        );
    }

    /// Parse a measure length change from the raw value (ch 02).
    ///
    /// The value is a ratio relative to 4/4: `1.0` means 4/4, `0.75` means 3/4,
    /// and `2.0` means 8/4. Non-numeric or zero values are silently ignored
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

    /// Parse absolute BPM changes from hexadecimal values (ch 03).
    ///
    /// A `"00"` value means a rest (no BPM change) and is **skipped** per the
    /// BMS specification: passing `BpmValue::Absolute(0.0)` downstream would
    /// make the timing track divide by zero.
    fn push_bpm_absolute_full(&mut self, values: &str, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.bpm_changes,
            &mut self.warnings,
            |position, value| {
                // Channel 03 values are hexadecimal integers (01-FF).
                let bpm_val = u8::from_str_radix(value, 16).ok()?;
                Some(BpmChange {
                    position,
                    value: BpmValue::Absolute(f64::from(bpm_val)),
                })
            },
        );
    }

    /// Parse BPM reference changes from the full concatenated values (ch 08).
    fn push_bpm_reference_full(&mut self, values: &str, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.bpm_changes,
            &mut self.warnings,
            |position, value| {
                let bpm_id = value.parse::<BpmIndex>().ok()?;
                Some(BpmChange {
                    position,
                    value: BpmValue::Reference(BpmIndex::from(bpm_id.normalize(context.base))),
                })
            },
        );
    }

    /// Parse stop events from the full concatenated values (ch 09).
    fn push_stop_full(&mut self, values: &str, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.stop_events,
            &mut self.warnings,
            |position, value| {
                let stop_id = value.parse::<StopIndex>().ok()?;
                Some(StopEvent {
                    position,
                    stop_id: StopIndex::from(stop_id.normalize(context.base)),
                })
            },
        );
    }

    /// Parse scroll events from the full concatenated values (ch SC).
    fn push_scroll_full(&mut self, values: &str, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.scroll_events,
            &mut self.warnings,
            |position, value| {
                let scroll_id = value.parse::<ScrollIndex>().ok()?;
                Some(ScrollEvent {
                    position,
                    scroll_id: ScrollIndex::from(scroll_id.normalize(context.base)),
                })
            },
        );
    }

    /// Parse speed keyframe events from the full concatenated values (ch SP).
    fn push_speed_full(&mut self, values: &str, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.speed_events,
            &mut self.warnings,
            |position, value| {
                let speed_id = value.parse::<SpeedIndex>().ok()?;
                Some(SpeedEvent {
                    position,
                    speed_id: SpeedIndex::from(speed_id.normalize(context.base)),
                })
            },
        );
    }

    /// Parse BGA display events from the full concatenated values (ch 04-07).
    ///
    /// `"00"` entries are filtered out: the object `00` is a rest on every
    /// channel, so it must not display `#BMP00` (the miss image referenced by
    /// the miss system, not by channel data).
    fn push_bga_full(&mut self, values: &str, layer: BmsBgaLayer, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.bga_events,
            &mut self.warnings,
            |position, value| {
                let bmp_id = value.parse::<BmpIndex>().ok()?;
                Some(BgaEvent {
                    position,
                    layer,
                    bmp_id: BmpIndex::from(bmp_id.normalize(context.base)),
                })
            },
        );
    }

    /// Parse playable note events from the full concatenated values
    /// (ch 11-49).
    ///
    /// Entries whose WAV index is `"00"` are filtered out: they mean "no note"
    /// (a silent step) and should not produce an event.
    fn push_playable_full(
        &mut self,
        values: &str,
        player: u8,
        lane: u8,
        key_type: KeyType,
        context: ChannelContext,
    ) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.note_events,
            &mut self.warnings,
            |position, value| {
                let wav_id = value.parse::<WavIndex>().ok()?;
                Some(NoteEvent {
                    position,
                    player,
                    lane,
                    key_type,
                    wav_id: WavIndex::from(wav_id.normalize(context.base)),
                })
            },
        );
    }

    /// Parse long-note events from the full concatenated values (ch 51-69).
    ///
    /// `"00"` entries are kept: in LNTYPE 2 (MGQ), `"00"` is the release marker
    /// used by `pair_lntype2`. `pair_lntype1` filters `"00"` internally as well,
    /// so they are kept uniformly and the LN pairing function handles them by
    /// mode.
    fn push_long_note_full(&mut self, values: &str, player: u8, lane: u8, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::including_zero(values),
            &mut self.long_note_events,
            &mut self.warnings,
            |position, value| {
                let wav_id = value.parse::<WavIndex>().ok()?;
                Some(LongNoteEvent {
                    position,
                    player,
                    lane,
                    wav_id: WavIndex::from(wav_id.normalize(context.base)),
                })
            },
        );
    }

    /// Parse mine events from the full concatenated values (ch D1-E9).
    ///
    /// The two-character index is decoded to a Base36 number and stored in
    /// `raw_value`; damage calculation belongs to the processor. `"00"` entries
    /// are filtered by [`BodyCells::non_zero`].
    fn push_mine_full(&mut self, values: &str, player: u8, lane: u8, context: ChannelContext) {
        parse_indexed(
            context,
            BodyCells::non_zero(values),
            &mut self.mine_events,
            &mut self.warnings,
            |position, value| {
                Some(MineEvent {
                    position,
                    player,
                    lane,
                    raw_value: decode_mine_raw(value),
                })
            },
        );
    }

    /// Dispatch a note channel to the corresponding handler.
    ///
    /// Called from [`finalize`](Self::finalize). The second channel byte is
    /// Base36-decoded to obtain `lane` (`'1'`-`'9'` for 1-9, `'A'`-`'Z'` for
    /// 10-35); the first byte determines the player side and note kind (visible,
    /// invisible, long note, or mine).
    ///
    /// `lane == 0` (reserved channels `"10"`, `"20"`, and so on) is filtered
    /// out; although [`classify_channel`](bmsrs_bms_values::classify_channel) already
    /// classifies these as `Unknown`, filtering again here is defensive.
    ///
    /// Mine channels `D1`-`D9` and `E1`-`E9` are valid only for lanes 1-9;
    /// extended ranges (such as a hypothetical `DA`) do not produce a
    /// `MineEvent`.
    fn dispatch_note_channel(&mut self, values: &str, ch: ChannelIndex, context: ChannelContext) {
        let bytes = ch.as_bytes();
        // classify_channel returns the Note variant only for 2-byte channels;
        // 1-byte channels (such as `"1"` for Bgm) take the non-Note branch.
        // Require 2 bytes here for safe indexing.
        let &[first, second] = bytes else {
            return;
        };

        // Reserved channels `"10"`, `"20"`, and so on have lane 0; filter them.
        // `base36_digit_value` returns `Option<u16>`; lane values 0-35 fit in
        // u8, so `try_from` always succeeds.
        let Some(lane_u16) = base36_digit_value(second) else {
            return;
        };
        #[expect(
            clippy::expect_used,
            reason = "base36 value 0–35 always fits in u8, guaranteed by base36_digit_value contract"
        )]
        let lane = u8::try_from(lane_u16).expect(
            "base36 value 0–35 always fits in u8, guaranteed by base36_digit_value contract",
        );
        if lane == 0 {
            return;
        }

        match first {
            b'1' => self.push_playable_full(values, 1, lane, KeyType::Visible, context),
            b'2' => self.push_playable_full(values, 2, lane, KeyType::Visible, context),
            b'3' => self.push_playable_full(values, 1, lane, KeyType::Invisible, context),
            b'4' => self.push_playable_full(values, 2, lane, KeyType::Invisible, context),
            b'5' => self.push_long_note_full(values, 1, lane, context),
            b'6' => self.push_long_note_full(values, 2, lane, context),
            // Mine channels are valid only for lanes 1-9; `classify_channel`
            // already ensures that D/E channels reaching this branch have
            // second in '1'..='9'. Extended lanes are rejected explicitly here.
            b'D' if lane <= 9 => self.push_mine_full(values, 1, lane, context),
            b'E' if lane <= 9 => self.push_mine_full(values, 2, lane, context),
            _ => { /* Non-note first byte: kept only in raw. */ }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_channel_basic_example() {
        // Specification example from memo/03:
        //   #00113:11111111   (4 values)
        //   #00113:0022332255224400  (8 values)
        //   #00113:0066        (2 values)
        //   Result: 1122332266224400
        let lines = vec![
            "11111111".to_owned(),
            "0022332255224400".to_owned(),
            "0066".to_owned(),
        ];
        assert_eq!(Messages::merge_bodies(&lines), "1122332266224400");
    }

    #[test]
    fn merge_channel_single_line_passthrough() {
        let lines = vec!["AABBCC".to_owned()];
        assert_eq!(Messages::merge_bodies(&lines), "AABBCC");
    }

    #[test]
    fn merge_channel_00_preserves_earlier() {
        // When line 2 has "00" at that position, line 1's non-00 value is
        // preserved.
        let lines = vec!["11".to_owned(), "00".to_owned()];
        assert_eq!(Messages::merge_bodies(&lines), "11");
    }

    #[test]
    fn merge_channel_later_overwrites_non_00() {
        // A later line's non-00 value overrides the earlier non-00 value.
        let lines = vec!["11".to_owned(), "22".to_owned()];
        assert_eq!(Messages::merge_bodies(&lines), "22");
    }
}
