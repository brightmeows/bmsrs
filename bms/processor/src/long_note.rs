//! Long-note pairing for BMS charts.
//!
//! BMS has three LN notations:
//!
//! - **LNTYPE 1 (RDM)**: Events on channels 51–59 / 61–69. Entries with
//!   index `"00"` are skipped (no LN at that position). Consecutive non-`"00"`
//!   events on the same `(player, lane)` form start-end pairs.
//! - **LNTYPE 2 (MGQ)**: Events on the same channels. An LN persists while
//!   consecutive non-`"00"` entries appear, and closes at the next `"00"`.
//! - **LNOBJ**: A designated WAV index marks LN endings. LN starts are
//!   regular note events; the matching end is a subsequent note with
//!   the `#LNOBJ` WAV on the same `(player, lane)`.

use std::collections::BTreeMap;

use bms_parser::{LongNoteEvent, NoteEvent};
use bms_tokenizer::{LnObjIndex, WavIndex};

use crate::position::MeasureTable;

/// A paired long note: start tick, duration in ticks, and WAV id.
pub struct PairedLn {
    /// Absolute tick of the LN start.
    pub tick: u64,
    /// Duration in ticks (end tick - start tick).
    pub duration: u64,
    /// WAV index of the LN start event.
    pub wav_id: WavIndex,
    /// Player number (1 or 2).
    pub player: u8,
    /// Original lane (1-9).
    pub lane: u8,
}

/// Returns `true` if the WAV index represents an empty / no-note position
/// (the `"00"` index in BMS notation).
#[inline]
fn is_empty_index(idx: WavIndex) -> bool {
    idx.as_str() == "00"
}

/// Pair LNTYPE 1 (RDM) long-note events from channels 51-69.
///
/// Entries with `"00"` index are **filtered out** (they represent gaps).
/// The remaining non-`"00"` events form consecutive start-end pairs:
/// the first event is the LN start, the second is the end, the third
/// is the next start, etc.
pub fn pair_lntype1(events: &[LongNoteEvent], table: &MeasureTable) -> Vec<PairedLn> {
    // Group by (player, lane), filtering out "00" entries.
    let mut groups: BTreeMap<(u8, u8), Vec<&LongNoteEvent>> = BTreeMap::new();
    for ev in events {
        if is_empty_index(ev.wav_id) {
            continue;
        }
        groups.entry((ev.player, ev.lane)).or_default().push(ev);
    }

    let mut result = Vec::new();

    for (&(player, lane), group) in &groups {
        let mut sorted = group.clone();
        sorted.sort_by_key(|ev| (u32::from(ev.position.measure) * 1_000_000) + ev.position.numer);

        // Consume events in consecutive pairs: first = start, second = end.
        let mut iter = sorted.into_iter();
        while let (Some(start), Some(end)) = (iter.next(), iter.next()) {
            let start_tick = table.position_to_tick(start.position);
            let end_tick = table.position_to_tick(end.position);

            result.push(PairedLn {
                tick: start_tick,
                duration: end_tick.saturating_sub(start_tick),
                wav_id: start.wav_id,
                player,
                lane,
            });
        }
    }

    result
}

/// Pair LNTYPE 2 (MGQ) long-note events from channels 51-69.
///
/// In MGQ notation, an LN persists while consecutive non-`"00"` entries
/// appear and closes at the **next** `"00"` entry.  Every `"00"` entry
/// acts as a release for any active LN.
///
/// Algorithm per `(player, lane)` group:
/// 1. Scan events in sorted order.
/// 2. Outside an LN: skip `"00"` entries; first non-`"00"` = LN start.
/// 3. Inside an LN: the first `"00"` entry = LN end → emit pair.
/// 4. If an LN is still active at the end of the group, it is dropped
///    (no paired end → no LN).
pub fn pair_lntype2(events: &[LongNoteEvent], table: &MeasureTable) -> Vec<PairedLn> {
    // Group by (player, lane) — keep ALL entries including "00".
    let mut groups: BTreeMap<(u8, u8), Vec<&LongNoteEvent>> = BTreeMap::new();
    for ev in events {
        groups.entry((ev.player, ev.lane)).or_default().push(ev);
    }

    let mut result = Vec::new();

    for (&(player, lane), group) in &groups {
        let mut sorted = group.clone();
        sorted.sort_by_key(|ev| (u32::from(ev.position.measure) * 1_000_000) + ev.position.numer);

        let mut in_ln = false;
        let mut start_ev: Option<&LongNoteEvent> = None;

        for ev in sorted {
            if in_ln {
                // Inside an LN: a "00" entry ends it.
                if is_empty_index(ev.wav_id) {
                    if let Some(start) = start_ev.take() {
                        let start_tick = table.position_to_tick(start.position);
                        let end_tick = table.position_to_tick(ev.position);
                        result.push(PairedLn {
                            tick: start_tick,
                            duration: end_tick.saturating_sub(start_tick),
                            wav_id: start.wav_id,
                            player,
                            lane,
                        });
                    }
                    in_ln = false;
                }
                // Non-"00" while inside LN: LN continues (no state change).
            } else if !is_empty_index(ev.wav_id) {
                // Outside an LN: first non-"00" starts one.
                in_ln = true;
                start_ev = Some(ev);
            }
        }
        // If an LN is still active at group end, it is silently dropped
        // (unterminated — no paired end available).
    }

    result
}

/// Pair LNOBJ long notes from regular note events.
///
/// Scans `note_events` for notes whose `wav_id` matches the `#LNOBJ`
/// marker. Each such note is an LN end; the preceding note on the same
/// `(player, lane)` (that is NOT an LNOBJ marker) is the LN start.
///
/// Returns the paired LNs and the indices of consumed note events
/// (both starts and ends) so the caller can remove them.
pub fn pair_lnobj(
    note_events: &[NoteEvent],
    ln_obj: LnObjIndex,
    table: &MeasureTable,
) -> (Vec<PairedLn>, std::collections::BTreeSet<usize>) {
    // Index notes by (player, lane) to find preceding notes.
    let mut last_by_lane: BTreeMap<(u8, u8), (usize, &NoteEvent)> = BTreeMap::new();
    let mut paired = Vec::new();
    let mut consumed = std::collections::BTreeSet::new();

    for (i, ev) in note_events.iter().enumerate() {
        // Compare underlying BmsIndex values (case-insensitive in standard BMS).
        if *ev.wav_id == *ln_obj {
            // This is an LN end. Find the preceding note on the same (player, lane).
            if let Some(&(start_idx, start_ev)) = last_by_lane.get(&(ev.player, ev.lane)) {
                let start_tick = table.position_to_tick(start_ev.position);
                let end_tick = table.position_to_tick(ev.position);

                paired.push(PairedLn {
                    tick: start_tick,
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
    use bms_parser::Position;
    use bms_tokenizer::LnObjIndex;

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
            key_type: bms_parser::KeyType::Visible,
            wav_id: wav.parse().unwrap(),
        }
    }

    #[test]
    fn lntype1_pairs_consecutive_events() {
        let events = vec![ln_event(1, 1, 0, 0, "AA"), ln_event(1, 1, 1, 0, "BB")];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 1);
        let ln = &result[0];
        assert_eq!(ln.tick, 0);
        assert_eq!(ln.duration, 960); // 1 measure at res 240
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

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].tick, 0);
        assert_eq!(result[1].tick, 960);
    }

    #[test]
    fn lntype1_odd_event_unpaired() {
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 1, 0, "BB"),
            ln_event(1, 1, 2, 0, "CC"), // unpaired
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 1);
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

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].lane, 1);
        assert_eq!(result[1].lane, 2);
    }

    #[test]
    fn lnobj_pairs_start_and_end() {
        let ln_obj: LnObjIndex = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "AA"),
            note_event(1, 1, 1, 0, "FF"), // LNOBJ marker
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
            note_event(1, 2, 0, 0, "BB"), // regular note, not consumed
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
            note_event(1, 1, 0, 0, "FF"), // marker with no preceding note
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert!(paired.is_empty());
        assert!(consumed.is_empty());
    }

    // LNTYPE 1: "00" entries must be filtered out

    #[test]
    fn lntype1_skips_zero_entries() {
        // Channel data: AA(sound) 00(silent) BB(sound) → single pair (AA, BB).
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "BB"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
    }

    #[test]
    fn lntype1_skips_multiple_zero_entries() {
        // AA 00 00 BB 00 CC DD → pairs: (AA, BB), (CC, DD)
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

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
        assert_eq!(result[1].wav_id, "CC".parse().unwrap());
    }

    #[test]
    fn lntype1_all_zero_entries_produces_nothing() {
        let events = vec![ln_event(1, 1, 0, 0, "00"), ln_event(1, 1, 0, 1, "00")];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert!(result.is_empty());
    }

    // LNTYPE 2 (MGQ)

    #[test]
    fn lntype2_pairs_run_with_trailing_zero() {
        // AA BB CC 00 → LN from AA to the 00.
        // denom=8 (via ln_event helper), so each numer step = 960/8 = 120 ticks.
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "BB"),
            ln_event(1, 1, 0, 2, "CC"),
            ln_event(1, 1, 0, 3, "00"),
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tick, 0);
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
        // 00 at numer=3 out of 8 → position = 3/8 measure → tick = 960 * 3/8 = 360
        assert_eq!(result[0].duration, 360);
    }

    #[test]
    fn lntype2_multiple_runs() {
        // AA 00 BB CC 00 DD 00 → LN1: (AA, first 00), LN2: (BB, second 00),
        // LN3: (DD, third 00).  denom=8 → each numer step = 120 ticks.
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"), // tick 0
            ln_event(1, 1, 0, 1, "00"), // tick 120
            ln_event(1, 1, 0, 2, "BB"), // tick 240
            ln_event(1, 1, 0, 3, "CC"), // tick 360 — continues LN2
            ln_event(1, 1, 1, 0, "00"), // tick 960
            ln_event(1, 1, 1, 1, "DD"), // tick 1080
            ln_event(1, 1, 1, 2, "00"), // tick 1200
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.len(), 3);
        // LN1: AA(0) → 00(120)
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
        assert_eq!(result[0].tick, 0);
        assert_eq!(result[0].duration, 120);
        // LN2: BB(240) → 00(960) — CC(360) continues the LN
        assert_eq!(result[1].wav_id, "BB".parse().unwrap());
        assert_eq!(result[1].tick, 240);
        assert_eq!(result[1].duration, 720);
        // LN3: DD(1080) → 00(1200)
        assert_eq!(result[2].wav_id, "DD".parse().unwrap());
        assert_eq!(result[2].tick, 1080);
        assert_eq!(result[2].duration, 120);
    }

    #[test]
    fn lntype2_unterminated_run_dropped() {
        // AA BB (no trailing 00) → no paired LN.
        let events = vec![ln_event(1, 1, 0, 0, "AA"), ln_event(1, 1, 0, 1, "BB")];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert!(result.is_empty());
    }

    #[test]
    fn lntype2_immediate_release() {
        // AA 00 00 BB 00 → LN1: (AA, first 00), gap, LN2: (BB, second 00)
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "00"),
            ln_event(1, 1, 0, 3, "BB"),
            ln_event(1, 1, 1, 0, "00"),
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.len(), 2);
    }
}
