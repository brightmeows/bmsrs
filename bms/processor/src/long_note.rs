//! Long-note pairing for BMS charts.
//!
//! BMS has three LN (long-note) notations:
//!
//! - **LNTYPE 1 (RDM)**: events on channels 51-59 / 61-69. Entries with index
//!   `"00"` are skipped (no long note at that position). Consecutive non-`"00"`
//!   events on the same `(player, lane)` form start-end pairs.
//! - **LNTYPE 2 (MGQ)**: events on the same channels. A long note continues
//!   while consecutive non-`"00"` entries appear and closes at the next `"00"`.
//! - **LNOBJ**: a designated WAV index marks the long-note end. The start is an
//!   ordinary note event; the matching end is a later note on the same
//!   `(player, lane)` carrying the `#LNOBJ` WAV.

use std::collections::{BTreeMap, HashMap, HashSet};

use bmsrs_bms_parser::{KeyType, LnObjIndex, WavIndex};
use bmsrs_bms_parser::{LongNoteEvent, NoteEvent};

use crate::position::MeasureTable;

/// LN pairing result, containing paired long notes and unpaired start ticks.
pub struct LnPairingResult {
    /// Paired long notes.
    pub paired: Vec<PairedLn>,
    /// `(player, lane, start_tick)` for leftover odd long-channel events.
    pub unpaired_starts: Vec<(u8, u8, u64)>,
}

/// A paired long note: start tick, end tick, tick duration, and WAV index.
pub struct PairedLn {
    /// Absolute tick of the long-note start.
    pub tick: u64,
    /// Absolute tick of the long-note end.
    pub end_tick: u64,
    /// Tick duration (end tick minus start tick).
    pub duration: u64,
    /// WAV index of the long-note start event.
    pub wav_id: WavIndex,
    /// Player number (1 or 2).
    pub player: u8,
    /// Original lane (1-9).
    pub lane: u8,
}

/// Return `true` when the WAV index represents an empty / no-note position (the
/// `"00"` index in BMS notation).
#[inline]
fn is_empty_index(idx: WavIndex) -> bool {
    idx.as_str() == "00"
}

/// Pair LNTYPE 1 (RDM) long-note events from channels 51-69.
///
/// Entries with index `"00"` are **filtered out** (they represent gaps). The
/// remaining non-`"00"` events form consecutive start-end pairs: the first is a
/// start, the second is its end, the third is the next start, and so on.
pub fn pair_lntype1(events: &[LongNoteEvent], table: &MeasureTable) -> LnPairingResult {
    // Group by (player, lane), filtering out "00" entries.
    // BTreeMap is used so pairing output is ordered by (player, lane); consumers
    // assume deterministic ordering.
    let mut groups: BTreeMap<(u8, u8), Vec<&LongNoteEvent>> = BTreeMap::new();
    for ev in events {
        if is_empty_index(ev.wav_id) {
            continue;
        }
        groups.entry((ev.player, ev.lane)).or_default().push(ev);
    }

    let mut paired = Vec::new();
    let mut unpaired_starts = Vec::new();

    for (&(player, lane), group) in &groups {
        let mut sorted = group.clone();
        sorted.sort_by_key(|ev| (ev.position.measure(), ev.position.numer()));

        // Consume events in consecutive pairs: first = start, second = end.
        let mut iter = sorted.into_iter();
        while let Some(start) = iter.next() {
            let Some(end) = iter.next() else {
                // A single unpaired event.
                let start_tick = table.position_to_tick(start.position);
                unpaired_starts.push((player, lane, start_tick));
                break;
            };
            let start_tick = table.position_to_tick(start.position);
            let end_tick = table.position_to_tick(end.position);

            paired.push(PairedLn {
                tick: start_tick,
                end_tick,
                duration: end_tick.saturating_sub(start_tick),
                wav_id: start.wav_id,
                player,
                lane,
            });
        }
    }

    LnPairingResult {
        paired,
        unpaired_starts,
    }
}

/// Pair LNTYPE 2 (MGQ) long-note events from channels 51-69.
///
/// In MGQ notation, a long note continues while consecutive non-`"00"` entries
/// appear and closes at the **next** `"00"` entry. Every `"00"` entry acts as a
/// release point for any active long note.
///
/// Algorithm for each `(player, lane)` group:
/// 1. Scan events in sorted order.
/// 2. Outside a long note: skip `"00"` entries; the first non-`"00"` starts one.
/// 3. Inside a long note: the first `"00"` entry ends it and produces a pair.
/// 4. If a long note is still active when the group ends, drop it (no paired
///    end means no long note).
pub fn pair_lntype2(events: &[LongNoteEvent], table: &MeasureTable) -> LnPairingResult {
    // Group by (player, lane), keeping all entries including "00".
    // BTreeMap is used so pairing output is ordered by (player, lane); consumers
    // assume deterministic ordering.
    let mut groups: BTreeMap<(u8, u8), Vec<&LongNoteEvent>> = BTreeMap::new();
    for ev in events {
        groups.entry((ev.player, ev.lane)).or_default().push(ev);
    }

    let mut paired = Vec::new();
    let mut unpaired_starts = Vec::new();

    for (&(player, lane), group) in &groups {
        let mut sorted = group.clone();
        sorted.sort_by_key(|ev| (ev.position.measure(), ev.position.numer()));

        let mut in_ln = false;
        let mut start_ev: Option<&LongNoteEvent> = None;

        for ev in sorted {
            if in_ln {
                // Inside a long note: a "00" entry ends it.
                if is_empty_index(ev.wav_id) {
                    if let Some(start) = start_ev.take() {
                        let start_tick = table.position_to_tick(start.position);
                        let end_tick = table.position_to_tick(ev.position);
                        paired.push(PairedLn {
                            tick: start_tick,
                            end_tick,
                            duration: end_tick.saturating_sub(start_tick),
                            wav_id: start.wav_id,
                            player,
                            lane,
                        });
                    }
                    in_ln = false;
                }
                // A non-"00" inside a long note: the note continues (no state
                // change).
            } else if !is_empty_index(ev.wav_id) {
                // Outside a long note: the first non-"00" starts one.
                in_ln = true;
                start_ev = Some(ev);
            }
        }
        // If a long note is still active when the group ends, record it as an
        // unpaired start.
        if let Some(start) = start_ev {
            let start_tick = table.position_to_tick(start.position);
            unpaired_starts.push((player, lane, start_tick));
        }
    }

    LnPairingResult {
        paired,
        unpaired_starts,
    }
}

/// Pair LNOBJ long notes from ordinary note events.
///
/// Scans `note_events` for notes whose `wav_id` matches the `#LNOBJ` marker.
/// Each such note is a long-note end; the previous note on the same
/// `(player, lane)` (one that is not the LNOBJ marker) is the start.
///
/// `#LNOBJ` applies to the visible channels `11`-`29` only; invisible notes
/// carrying the marker index stay ordinary invisible notes.
///
/// Returns the paired long notes and the indices of consumed note events
/// (including both start and end) so the caller can remove them.
pub fn pair_lnobj(
    note_events: &[NoteEvent],
    ln_obj: LnObjIndex,
    table: &MeasureTable,
) -> (Vec<PairedLn>, HashSet<usize>) {
    // Index notes by (player, lane) to find the preceding note.
    let mut last_by_lane: HashMap<(u8, u8), (usize, &NoteEvent)> = HashMap::new();
    let mut paired = Vec::new();
    let mut consumed = HashSet::new();

    for (i, ev) in note_events.iter().enumerate() {
        if ev.key_type != KeyType::Visible {
            continue;
        }
        // Compare the underlying BmsIndex values (case-insensitive in standard
        // BMS).
        if *ev.wav_id == *ln_obj {
            // This is a long-note end. Find the preceding note on the same
            // (player, lane).
            if let Some(&(start_idx, start_ev)) = last_by_lane.get(&(ev.player, ev.lane)) {
                let start_tick = table.position_to_tick(start_ev.position);
                let end_tick = table.position_to_tick(ev.position);

                paired.push(PairedLn {
                    tick: start_tick,
                    end_tick,
                    duration: end_tick.saturating_sub(start_tick),
                    wav_id: start_ev.wav_id,
                    player: ev.player,
                    lane: ev.lane,
                });
                consumed.insert(start_idx);
                consumed.insert(i);
                last_by_lane.remove(&(ev.player, ev.lane));
            }
        } else {
            last_by_lane.insert((ev.player, ev.lane), (i, ev));
        }
    }

    (paired, consumed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmsrs_bms_parser::LnObjIndex;
    use bmsrs_bms_parser::Position;

    fn make_table() -> MeasureTable {
        MeasureTable::new(4, &[], 240)
    }

    fn ln_event(player: u8, lane: u8, measure: u16, numer: u32, wav: &str) -> LongNoteEvent {
        LongNoteEvent {
            position: Position::new(measure, numer, 8),
            player,
            lane,
            wav_id: wav.parse().unwrap(),
        }
    }

    fn note_event(player: u8, lane: u8, measure: u16, numer: u32, wav: &str) -> NoteEvent {
        NoteEvent {
            position: Position::new(measure, numer, 8),
            player,
            lane,
            key_type: bmsrs_bms_parser::KeyType::Visible,
            wav_id: wav.parse().unwrap(),
        }
    }

    #[test]
    fn lntype1_pairs_consecutive_events() {
        let events = vec![ln_event(1, 1, 0, 0, "AA"), ln_event(1, 1, 1, 0, "BB")];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.paired.len(), 1);
        let ln = &result.paired[0];
        assert_eq!(ln.tick, 0);
        assert_eq!(ln.end_tick, 960);
        assert_eq!(ln.duration, 960); // One measure at resolution 240.
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    #[test]
    fn lntype1_multiple_pairs_same_lane() {
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 4, "BB"),
            ln_event(1, 1, 1, 0, "CC"),
            ln_event(1, 1, 1, 4, "DD"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.paired.len(), 2);
        assert_eq!(result.paired[0].tick, 0);
        assert_eq!(result.paired[1].tick, 960);
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    #[test]
    fn lntype1_odd_event_unpaired() {
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 1, 0, "BB"),
            ln_event(1, 1, 2, 0, "CC"), // Unpaired.
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.paired.len(), 1);
        assert_eq!(result.unpaired_starts.len(), 1);
        assert_eq!(result.unpaired_starts[0], (1, 1, 1920)); // CC at numer 0 of measure 2
    }

    #[test]
    fn lntype1_different_lanes_independent() {
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 2, 0, 0, "BB"),
            ln_event(1, 1, 1, 0, "CC"),
            ln_event(1, 2, 1, 0, "DD"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.paired.len(), 2);
        assert_eq!(result.paired[0].lane, 1);
        assert_eq!(result.paired[1].lane, 2);
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    #[test]
    fn lnobj_pairs_start_and_end() {
        let ln_obj: LnObjIndex = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "AA"),
            note_event(1, 1, 1, 0, "FF"), // LNOBJ marker.
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert_eq!(paired.len(), 1);
        assert_eq!(paired[0].tick, 0);
        assert_eq!(paired[0].duration, 960);
        assert_eq!(paired[0].wav_id, "AA".parse().unwrap());
        assert_eq!(consumed.len(), 2);
    }

    #[test]
    fn lnobj_consumed_notes_excluded_from_regular() {
        let ln_obj: LnObjIndex = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "AA"),
            note_event(1, 1, 1, 0, "FF"),
            note_event(1, 2, 0, 0, "BB"), // Regular note, not consumed.
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert_eq!(paired.len(), 1);
        assert!(consumed.contains(&0));
        assert!(consumed.contains(&1));
        assert!(!consumed.contains(&2));
    }

    #[test]
    fn lnobj_unmatched_marker_produces_no_pair() {
        let ln_obj: LnObjIndex = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "FF"), // Marker with no preceding note.
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert!(paired.is_empty());
        assert!(consumed.is_empty());
    }

    // LNTYPE 1: "00" entries must be filtered out

    #[test]
    fn lntype1_skips_zero_entries() {
        // Channel data: AA (sounding), 00 (silent), BB (sounding), giving a
        // single pair (AA, BB).
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "BB"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.paired.len(), 1);
        assert_eq!(result.paired[0].wav_id, "AA".parse().unwrap());
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    #[test]
    fn lntype1_skips_multiple_zero_entries() {
        // AA 00 00 BB 00 CC DD gives pairs (AA, BB) and (CC, DD).
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "00"),
            ln_event(1, 1, 0, 3, "BB"),
            ln_event(1, 1, 0, 4, "00"),
            ln_event(1, 1, 0, 5, "CC"),
            ln_event(1, 1, 0, 6, "DD"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.paired.len(), 2);
        assert_eq!(result.paired[0].wav_id, "AA".parse().unwrap());
        assert_eq!(result.paired[1].wav_id, "CC".parse().unwrap());
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    #[test]
    fn lntype1_all_zero_entries_produces_nothing() {
        let events = vec![ln_event(1, 1, 0, 0, "00"), ln_event(1, 1, 0, 1, "00")];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert!(result.paired.is_empty());
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    // LNTYPE 2 (MGQ)

    #[test]
    fn lntype2_pairs_run_with_trailing_zero() {
        // AA BB CC 00 gives a long note from AA to 00.
        // denom=8 (through the ln_event helper), so each numer step is
        // 960/8 = 120 ticks.
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "BB"),
            ln_event(1, 1, 0, 2, "CC"),
            ln_event(1, 1, 0, 3, "00"),
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.paired.len(), 1);
        assert_eq!(result.paired[0].tick, 0);
        assert_eq!(result.paired[0].end_tick, 360);
        assert_eq!(result.paired[0].wav_id, "AA".parse().unwrap());
        // 00 at numer=3 of 8 gives position 3/8 of the measure, or
        // 960 * 3/8 = 360 ticks.
        assert_eq!(result.paired[0].duration, 360);
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    #[test]
    fn lntype2_multiple_runs() {
        // AA 00 BB CC 00 DD 00 gives LN1 (AA to the first 00), LN2 (BB to the
        // second 00), and LN3 (DD to the third 00). denom=8, so each numer step
        // is 120 ticks.
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"), // Tick 0.
            ln_event(1, 1, 0, 1, "00"), // Tick 120.
            ln_event(1, 1, 0, 2, "BB"), // Tick 240.
            ln_event(1, 1, 0, 3, "CC"), // Tick 360, continues LN2.
            ln_event(1, 1, 1, 0, "00"), // Tick 960.
            ln_event(1, 1, 1, 1, "DD"), // Tick 1080.
            ln_event(1, 1, 1, 2, "00"), // Tick 1200.
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.paired.len(), 3);
        // LN1: AA (0) to 00 (120).
        assert_eq!(result.paired[0].wav_id, "AA".parse().unwrap());
        assert_eq!(result.paired[0].tick, 0);
        assert_eq!(result.paired[0].end_tick, 120);
        assert_eq!(result.paired[0].duration, 120);
        // LN2: BB (240) to 00 (960); CC (360) continues the long note.
        assert_eq!(result.paired[1].wav_id, "BB".parse().unwrap());
        assert_eq!(result.paired[1].tick, 240);
        assert_eq!(result.paired[1].end_tick, 960);
        assert_eq!(result.paired[1].duration, 720);
        // LN3: DD (1080) to 00 (1200).
        assert_eq!(result.paired[2].wav_id, "DD".parse().unwrap());
        assert_eq!(result.paired[2].tick, 1080);
        assert_eq!(result.paired[2].end_tick, 1200);
        assert_eq!(result.paired[2].duration, 120);
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }

    #[test]
    fn lntype2_unterminated_run_returns_unpaired_start() {
        // AA BB (no trailing 00) gives no paired long note, but
        // unpaired_starts contains the start.
        let events = vec![ln_event(1, 1, 0, 0, "AA"), ln_event(1, 1, 0, 1, "BB")];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert!(result.paired.is_empty());
        assert_eq!(result.unpaired_starts.len(), 1);
        assert_eq!(result.unpaired_starts[0], (1, 1, 0));
    }

    #[test]
    fn lntype2_immediate_release() {
        // AA 00 00 BB 00 gives LN1 (AA to the first 00), a gap, then LN2 (BB to
        // the second 00).
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "00"),
            ln_event(1, 1, 0, 3, "BB"),
            ln_event(1, 1, 1, 0, "00"),
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.paired.len(), 2);
        assert_eq!(result.unpaired_starts, [] as [(u8, u8, u64); 0]);
    }
}
