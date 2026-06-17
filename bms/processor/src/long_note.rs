//! Long-note pairing for BMS charts.
//!
//! BMS has two LN notations:
//!
//! - **LNTYPE 1 (RDM)**: Events on channels 51-59 / 61-69. Consecutive
//!   events on the same `(player, lane)` form start-end pairs.
//! - **LNOBJ**: A designated WAV index marks LN endings. LN starts are
//!   regular note events; the matching end is a subsequent note with
//!   the `#LNOBJ` WAV on the same `(player, lane)`.

use std::collections::BTreeMap;

use bms_parser::{LongNoteEvent, NoteEvent};
use bms_tokenizer::{BmsIndex, LnObjTag, WavTag};

use crate::position::MeasureTable;

/// A paired long note: start tick, duration in ticks, and WAV id.
pub struct PairedLn {
    /// Absolute tick of the LN start.
    pub tick: u64,
    /// Duration in ticks (end tick - start tick).
    pub duration: u64,
    /// WAV index of the LN start event.
    pub wav_id: BmsIndex<WavTag>,
    /// Player number (1 or 2).
    pub player: u8,
    /// Original lane (1-9).
    pub lane: u8,
}

/// Pair LNTYPE 1 long-note events from channels 51-69.
///
/// Events are grouped by `(player, lane)` and sorted by position.
/// Consecutive pairs form `(start, end)`: the first event is the start,
/// the second is the end, the third is the next start, etc.
#[expect(
    clippy::indexing_slicing,
    reason = "indices bounded by while condition: i + 1 < len"
)]
pub fn pair_lntype1(events: &[LongNoteEvent], table: &MeasureTable) -> Vec<PairedLn> {
    // Group by (player, lane) preserving insertion order.
    let mut groups: BTreeMap<(u8, u8), Vec<&LongNoteEvent>> = BTreeMap::new();
    for ev in events {
        groups.entry((ev.player, ev.lane)).or_default().push(ev);
    }

    let mut result = Vec::new();

    for (&(player, lane), group) in &groups {
        let mut sorted = group.clone();
        sorted.sort_by_key(|ev| (u32::from(ev.position.measure) * 1_000_000) + ev.position.numer);

        let mut i = 0;
        while i + 1 < sorted.len() {
            let start = sorted[i];
            let end = sorted[i + 1];
            let start_tick = table.position_to_tick(start.position);
            let end_tick = table.position_to_tick(end.position);

            result.push(PairedLn {
                tick: start_tick,
                duration: end_tick.saturating_sub(start_tick),
                wav_id: start.wav_id,
                player,
                lane,
            });
            i += 2;
        }
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
    ln_obj: BmsIndex<LnObjTag>,
    table: &MeasureTable,
) -> (Vec<PairedLn>, std::collections::BTreeSet<usize>) {
    let ln_obj_str = ln_obj.as_str();
    // Index notes by (player, lane) to find preceding notes.
    let mut last_by_lane: BTreeMap<(u8, u8), (usize, &NoteEvent)> = BTreeMap::new();
    let mut paired = Vec::new();
    let mut consumed = std::collections::BTreeSet::new();

    for (i, ev) in note_events.iter().enumerate() {
        if ev.wav_id.as_str() == ln_obj_str {
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
    use bms_tokenizer::LnObjTag;

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
        let ln_obj: BmsIndex<LnObjTag> = "FF".parse().unwrap();
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
        let ln_obj: BmsIndex<LnObjTag> = "FF".parse().unwrap();
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
        let ln_obj: BmsIndex<LnObjTag> = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "FF"), // marker with no preceding note
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert!(paired.is_empty());
        assert!(consumed.is_empty());
    }
}
