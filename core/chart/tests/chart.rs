#![expect(missing_docs, reason = "integration test")]

mod helper;

use bmsrs_chart::{BgmEvent, DefaultNoteData, Note, NoteKind};
use helper::make_test_chart;

#[test]
fn last_tick_empty_chart() {
    let chart = make_test_chart(vec![], vec![]);
    assert_eq!(chart.last_tick(), 0);
}

#[test]
fn last_tick_from_notes() {
    let chart = make_test_chart(
        vec![Note {
            tick: 960,
            audio: None,
            data: DefaultNoteData {
                lane: 0,
                kind: NoteKind::Normal,
            },
        }],
        vec![],
    );
    assert_eq!(chart.last_tick(), 960);
}

#[test]
fn last_tick_from_bgm_beyond_notes() {
    let chart = make_test_chart(
        vec![Note {
            tick: 480,
            audio: None,
            data: DefaultNoteData {
                lane: 0,
                kind: NoteKind::Normal,
            },
        }],
        vec![BgmEvent {
            tick: 1920,
            audio: 0,
        }],
    );
    assert_eq!(chart.last_tick(), 1920);
}

#[test]
fn duration_constant_bpm() {
    let chart = make_test_chart(
        vec![Note {
            tick: 480,
            audio: None,
            data: DefaultNoteData {
                lane: 0,
                kind: NoteKind::Normal,
            },
        }],
        vec![],
    );
    // 480 ticks at 120 BPM, resolution 240: 480/240 * 0.5 = 1.0s
    assert_eq!(chart.duration(), std::time::Duration::from_secs(1));
}
