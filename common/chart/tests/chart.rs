#![expect(missing_docs, reason = "integration test")]

mod helper;

use std::num::NonZeroU8;

use bmsrs_chart::{Event, EventKind, Lane, NoteKind, NoteSide};
use helper::make_test_chart;

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn note(tick: u64) -> Event<()> {
    Event::new(
        tick,
        EventKind::Note {
            side: NoteSide::P1,
            lane: Lane::Key(nz(1)),
            kind: NoteKind::Normal,
            audio_index: None,
            ext: (),
        },
    )
}

#[test]
fn last_tick_empty_chart() {
    let chart = make_test_chart(vec![]);
    assert_eq!(chart.data.last_tick(), 0);
}

#[test]
fn last_tick_from_notes() {
    let chart = make_test_chart(vec![note(960)]);
    assert_eq!(chart.data.last_tick(), 960);
}

#[test]
fn last_tick_from_bgm_beyond_notes() {
    let chart = make_test_chart(vec![
        note(480),
        Event::new(1920, EventKind::Bgm { audio_index: 0 }),
    ]);
    assert_eq!(chart.data.last_tick(), 1920);
}

#[test]
fn duration_constant_bpm() {
    let chart = make_test_chart(vec![note(480)]);
    // 120 BPM、分辨率 240 下 480 脉冲：480/240 * 0.5 = 1.0s
    assert_eq!(chart.data.duration(), std::time::Duration::from_secs(1));
}
