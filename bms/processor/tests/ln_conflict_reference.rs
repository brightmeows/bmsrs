#![expect(missing_docs, reason = "integration test")]

use bmsrs_bms_model::{Bms, KeyType, LongNoteEvent, NoteEvent, Position};
use bmsrs_bms_processor::layout::Bme;
use bmsrs_bms_processor::{BmsProcessor, ProcessWarningKind};
use bmsrs_chart::{CustomEvent, EventKind, NoteExt, NoteKind};

#[expect(
    clippy::unwrap_used,
    reason = "static fixture construction; a parse failure is a test bug"
)]
fn chart_with_wav(names: &[&str]) -> Bms {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    for (index, name) in ["01", "AA", "BB", "CC"].iter().zip(names) {
        bms.audio
            .wav_files
            .insert((*index).parse().unwrap(), (*name).to_owned());
    }
    bms
}

fn long_notes<T: NoteExt, C: CustomEvent>(chart: &bmsrs_chart::Chart<T, C>) -> Vec<(u64, u64)> {
    chart
        .data()
        .events()
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::Note(note) => match note.kind {
                NoteKind::Long { duration } => Some((event.tick(), duration)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// jbms-parser reference scenario: a long-note channel receives four events in
/// a row ("a long-note start is defined inside a long note"). LNTYPE 1 pairing
/// resolves them as consecutive start-end pairs; no unpaired warning is
/// expected.
#[test]
fn lntype1_consecutive_events_pair_in_order() {
    let mut bms = chart_with_wav(&["a.wav", "a.wav", "a.wav", "a.wav"]);
    for slot in [0u32, 2, 4, 6] {
        bms.messages.long_note_events.push(LongNoteEvent {
            position: Position::new(0, slot, 8),
            player: 1,
            lane: 1,
            wav_id: "01".parse().unwrap(),
        });
    }

    let outcome = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert_eq!(long_notes(&outcome.value.chart), [(0, 240), (480, 240)]);
    assert!(!outcome.warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::UnterminatedLongNote { .. }
    )));
}

/// jbms-parser reference scenario: a long-note start that never closes by the
/// end of the chart is dropped (jbms removes the note from its timeline), and
/// the unpaired-start warning fires.
#[test]
fn unpaired_ln_start_emits_no_note_and_warns() {
    let mut bms = chart_with_wav(&["a.wav"]);
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });

    let outcome = BmsProcessor::process::<Bme>(&bms).unwrap();
    let note_count = outcome
        .value
        .chart
        .data()
        .events()
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Note(_)))
        .count();
    assert_eq!(note_count, 0, "the unpaired long-note start is dropped");
    assert!(outcome.warnings.iter().any(|warning| matches!(
        warning.kind,
        ProcessWarningKind::UnterminatedLongNote {
            player: 1,
            lane: 1,
            tick: 0,
        }
    )));
}

/// jbms-parser reference scenario: a visible note inside a long-note range is
/// demoted to the BGM lane so its keysound keeps playing, with a warning.
#[test]
fn covered_note_keysound_moves_to_bgm_and_warns() {
    let mut bms = chart_with_wav(&["a.wav", "a.wav", "b.wav", "c.wav"]);
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        wav_id: "BB".parse().unwrap(),
    });
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 6, 8),
        player: 1,
        lane: 1,
        wav_id: "BB".parse().unwrap(),
    });
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 4, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "CC".parse().unwrap(),
    });

    let outcome = BmsProcessor::process::<Bme>(&bms).unwrap();
    let chart = &outcome.value.chart;
    let bgm_paths: Vec<&str> = chart
        .data()
        .events()
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::Bgm { audio_index } => Some(
                chart.data().audio_assets()[audio_index.get() as usize]
                    .path
                    .to_str()
                    .unwrap(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(bgm_paths, ["c.wav"], "the covered keysound moves to BGM");
    assert!(
        outcome.warnings.iter().any(|warning| matches!(
            warning.kind,
            ProcessWarningKind::NoteCoveredByLongNote {
                player: 1,
                lane: 1,
                tick: 480,
            }
        )),
        "the covered note warns"
    );
}
