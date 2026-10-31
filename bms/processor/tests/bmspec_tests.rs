//! `BMSpec` executable specification integration tests.
//!
//! These tests start from raw BMS text, run the full pipeline
//! (tokenizer, parser, processor, Chart), and verify that event timing matches
//! the bmspec assertions. Each test corresponds to a scenario in the bmspec
//! directory.
//!
//! Beat conversion: 1 beat = resolution (240) ticks.

use bmsrs_bms_parser::Bms;
use bmsrs_bms_processor::BmsProcessor;
use bmsrs_bms_processor::custom_event::BmsCustomEvent;
use bmsrs_bms_processor::layout::Bme;
use bmsrs_bms_syntax::SourceFile;
use bmsrs_chart::{Chart, EventKind, NoteKind};
use bmsrs_player::Player;

/// Chart type returned by bmsrs-bms-processor.
type BmsChart = Chart<(), BmsCustomEvent>;

/// Pipeline helper: BMS text to `Chart<(), BmsCustomEvent>`.
#[expect(clippy::expect_used, reason = "test helper panics on process failure")]
fn process(bms_text: &str) -> BmsChart {
    BmsProcessor::process::<Bme>(&parse_bms(bms_text))
        .expect("process should succeed")
        .chart
}

/// Parse BMS text with random value 1.
fn parse_bms(bms_text: &str) -> Bms {
    let source = SourceFile::parse(bms_text);
    Bms::from_cst(&source.source_file(), |_| 1).bms
}

/// Extract every Note event (including kind information).
fn all_notes(chart: &BmsChart) -> Vec<(u64, NoteKind)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note { kind, .. } = &e.kind {
                Some((e.tick(), *kind))
            } else {
                None
            }
        })
        .collect()
}

/// Extract every Scroll event.
fn scroll_events(chart: &BmsChart) -> Vec<(u64, f64)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Scroll { rate } = &e.kind {
                Some((e.tick(), *rate))
            } else {
                None
            }
        })
        .collect()
}

/// Extract every Speed event.
fn speed_events(chart: &BmsChart) -> Vec<(u64, f64)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Speed { rate } = &e.kind {
                Some((e.tick(), *rate))
            } else {
                None
            }
        })
        .collect()
}

/// Extract Long `Note` (`NoteKind::Long`) events.
fn long_notes(chart: &BmsChart) -> Vec<(u64, u64)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Long { duration },
                ..
            } = &e.kind
            {
                Some((e.tick(), *duration))
            } else {
                None
            }
        })
        .collect()
}

/// bmspec-1-01-Sentences: parse a standard BMS file.
#[test]
fn bmspec_1_01_parse_sentences() {
    let chart = process(
        "#TITLE BY MY SIDE\n\
         #ARTIST flicknote\n\
         #00101:0100010001\n\
         This is a comment\n",
    );
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
    // Channel 01 contains 4 BGM events. Comment lines are ignored.
    assert_eq!(chart.data.audio_assets.len(), 0); // WAV is undefined.
}

/// bmspec-1-01-Sentences: malformed command lines (a header without a space
/// separator is treated as a comment).
#[test]
fn bmspec_1_01_malformed_command() {
    // #ARTIST:flicknote (colon instead of a space) is not recognized as a
    // header.
    let chart = process("#TITLE BY MY SIDE\n#ARTIST:flicknote\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    // #ARTIST is not parsed (the colon makes parse_header_line fall back).
    assert_eq!(chart.song.artist, "");
}

/// bmspec-1-02-Header: standard header.
#[test]
fn bmspec_1_02_standard_header() {
    let chart = process("#TITLE BY MY SIDE\n#ARTIST flicknote\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
}

/// bmspec-1-02-Header: multiple-space separator.
#[test]
fn bmspec_1_02_header_multiple_spaces() {
    let chart = process("#TITLE      BY MY SIDE\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
}

/// bmspec-1-02-Header: case-insensitive.
#[test]
fn bmspec_1_02_case_insensitive_header() {
    let chart = process("#Title BY MY SIDE\n#Artist flicknote\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
}

/// bmspec-1-02-Header: duplicate headers, last wins.
#[test]
fn bmspec_1_02_duplicated_header() {
    let chart = process("#TiTlE BEAT MUSIC SEQUENCE\n#tItLe BY*MY*SIDE\n");
    assert_eq!(chart.song.title, "BY*MY*SIDE");
}

/// bmspec-1-03-Objects: basic note object positions.
#[test]
fn bmspec_1_03_basic_objects() {
    let chart = process("#00111:01000002\n#00311:0003\n");
    let notes = all_notes(&chart);
    // 3 notes at beats 4, 7, and 14.
    // beat = tick / 240.
    assert_eq!(notes.len(), 3);
    let ticks: Vec<u64> = notes.into_iter().map(|(t, _)| t).collect();
    assert_eq!(ticks[0], 960, "object 01 at beat 4"); // measure 1, pos 0/4
    assert_eq!(ticks[1], 1680, "object 02 at beat 7"); // measure 1, pos 3/4
    assert_eq!(ticks[2], 3360, "object 03 at beat 14"); // measure 3, pos 1/2
}

/// bmspec-1-03-Objects: overlapping note objects (channel merging).
#[test]
fn bmspec_1_03_overlapped_objects() {
    let chart = process(
        "#00113:11111111\n\
         #00113:0022332255224400\n\
         #00113:0066\n",
    );
    let notes = all_notes(&chart);
    // After merging there should be 7 objects ("00" filtered out).
    assert_eq!(notes.len(), 7);
    // merge: max_count=8.
    //   Line 1 ("11111111", 4 values) gives positions 0, 2, 4, 6.
    //   Line 2 ("0022332255224400", 8 values) gives 1=22, 2=33, 3=22, 4=55,
    //   5=22, 6=44.
    //   Line 3 ("0066", 2 values) gives 4=66 (overrides 55).
    // Merged result (00 filtered): 11(0), 22(1), 33(2), 22(3), 66(4), 22(5),
    // 44(6).
    // tick at pos i = 960 + i/8 * 960
    let ticks: Vec<u64> = notes.iter().map(|(t, _)| *t).collect();
    assert_eq!(ticks[4], 1440, "object 66 should be at beat 6"); // pos 4/8 → 960 + 480 = 1440
    assert_eq!(ticks[6], 1680, "object 44 at beat 7"); // pos 6/8 → 960 + 720 = 1680
}

/// bmspec-1-03-Objects: empty channel line.
#[test]
fn bmspec_1_03_empty_channel() {
    let chart = process("#00111:\n");
    let notes = all_notes(&chart);
    assert!(notes.is_empty(), "empty channel produces no objects");
}

/// bmspec-1-04-Time-Signature: variable time-signature positioning.
#[test]
fn bmspec_1_04_time_signature() {
    let chart = process(
        "#00102:0.750\n\
         #00111:0104\n\
         #00211:02\n\
         #00311:03\n",
    );
    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 4);
    let ticks: Vec<u64> = notes.into_iter().map(|(t, _)| t).collect();

    // measure 1 is 3/4 (0.75 × 960 = 720 ticks), 2 values → each=360 ticks
    // beat = tick / 240
    // obj 01 at beat 4: measure 1, pos 0/2 → tick 960
    // obj 04 at beat 5.5: measure 1, pos 1/2 → tick 1320
    // obj 02 at beat 7: measure 2, pos 0/1 → tick 1680
    // obj 03 at beat 11: measure 3, pos 0/1 → tick 2640

    assert_eq!(ticks[0], 960, "obj 01 at beat 4");
    assert_eq!(ticks[1], 1320, "obj 04 at beat 5.5");
    assert_eq!(ticks[2], 1680, "obj 02 at beat 7");
    assert_eq!(ticks[3], 2640, "obj 03 at beat 11");
}

/// bmspec-1-05-BPM: BPM changes; objects should land at the correct time
/// positions.
#[test]
fn bmspec_1_05_bpm_change() {
    let chart = process("#BPM 60\n#00003:0078\n#00111:01\n");

    // #00003:0078 → channel 03 (BPM change), track=0, values [00, 78]
    // 78 hex = 120 decimal → BPM change to 120 at position 0/2 of measure 0
    // Then #00111:01 → note at measure 1, pos 0/1

    let has_bpm = chart
        .data
        .events
        .iter()
        .any(|e| matches!(&e.kind, EventKind::Bpm { .. }));
    assert!(has_bpm, "should have BPM events");

    // The note should be at 3 seconds
    // At 60 BPM: tick 0 to ... Let me calculate
    // #00003:0078 → Position(0, 1, 2) → starts[0] + 1/2 * 960 = 0 + 480 = 480
    // At tick 480, BPM changes from 60 to 120
    // #00111:01 → Position(1, 0, 1) → starts[1] + 0 = 960

    let notes = all_notes(&chart);
    let note_tick = notes.first().map_or(0, |(t, _)| *t);
    assert_eq!(note_tick, 960, "note at tick 960");

    let dur = chart.data.timing.tick_to_duration(note_tick);
    // 0-480 ticks at 60 BPM: 480/240 * 60/60 = 2.0s
    // 480-960 ticks at 120 BPM: 480/240 * 60/120 = 1.0s
    // Total: 3.0s
    assert!(
        (dur.as_secs_f64() - 3.0).abs() < 1e-6,
        "expected 3.0s, got {dur:?}"
    );
}

/// bmspec-1-05-BPM: multiple BPM changes.
#[test]
fn bmspec_1_05_multiple_bpm_changes() {
    let chart = process(
        "#BPM 100\n\
         #00003:0060C0\n\
         #00011:00010203\n\
         #00111:04\n",
    );

    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 4, "should have 4 notes");
    let ticks: Vec<u64> = notes.iter().map(|(t, _)| *t).collect();
    // #00003:0060C0 → Position(0, 0, 2)=BPM 96 (60h=96), (0,1,2)=BPM 192 (C0h=192)
    // #00011:00010203 → 4 notes at measure 0: positions 0/4, 1/4, 2/4, 3/4

    // #00011:00010203 → measure=0, channel 11, values [00, 01, 02, 03]
    // "00" is filtered out. The remaining 3 notes are in measure 0 at positions
    // 1/4, 2/4, and 3/4.
    // #00111:04 → measure=1, pos=0/1
    // starts = [0, 960, 1920]
    // Position(0, 0, 4) → 0 (filtered out by "00" rule)
    // Position(0, 1, 4) → 0 + 960/4 = 240
    // Position(0, 2, 4) → 0 + 960/4*2 = 480
    // Position(0, 3, 4) → 0 + 960/4*3 = 720
    // Position(1, 0, 1) → 960
    assert_eq!(ticks[0], 240, "note 01");
    assert_eq!(ticks[1], 480, "note 02");
    assert_eq!(ticks[2], 720, "note 03");
    assert_eq!(ticks[3], 960, "note 04");

    // Times:
    // 0-240 ticks at 100 BPM: 240/240 * 60/100 = 0.6s → note 01 at 0.6s ✓
    let dur0 = chart.data.timing.tick_to_duration(240);
    assert!(
        (dur0.as_secs_f64() - 0.6).abs() < 1e-4,
        "note 01 should be at 0.6s, got {dur0:?}"
    );
}

/// bmspec-1-05-BPM: extended BPM (`#BPMxx` definition plus `#00008` reference).
#[test]
fn bmspec_1_05_extended_bpm() {
    let chart = process(
        "#BPM 60\n\
         #BPM01 120\n\
         #00008:0001\n\
         #00111:05\n",
    );

    // #BPM01 120 defines extended BPM index 01 = 120.0
    // #00008:0001 → channel 08 (extended BPM), track=0, values [00, 01]
    // "00" is a rest (no BPM change); "01" references BPM01=120 at position
    // 1/2.
    // BPM sequence:
    //   tick 0: init 60 (from #BPM 60)
    //   tick 480: BPM 120 (from "01" → BPM01)
    // Note at tick 960 = measure 1, pos 0/1
    //   0-480 ticks at 60 BPM = 480/240 * 60/60 = 2.0s
    //   480-960 ticks at 120 BPM = 480/240 * 60/120 = 1.0s
    //   Total: 3.0s

    let notes = all_notes(&chart);
    assert_eq!(notes[0].0, 960, "note at tick 960");
    let dur = chart.data.timing.tick_to_duration(notes[0].0);
    assert!(
        (dur.as_secs_f64() - 3.0).abs() < 1e-6,
        "expected 3.0s, got {dur:?}"
    );
}

/// bmspec-1-06-STOP: basic stop.
#[test]
fn bmspec_1_06_basic_stop() {
    let chart = process(
        "#BPM 60\n\
         #STOP11 96\n\
         #00111:01000200\n\
         #00109:00110000\n",
    );

    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 2, "should have 2 notes");
    // Note 01 at measure 1, pos 0/4 = tick 960
    // Note 02 at measure 1, pos 3/4 = tick 1680
    // Stop at position 0/4 of measure 1, duration = 96/192 * 960 = 480 ticks
    //   Wait, STOP duration = raw / 192.0 * resolution * 4
    //   96 / 192 * 960 = 480 ticks at 60 BPM = 4 seconds

    // Note 01 at tick 960:
    //   0-960 at 60 BPM = 960/240 * 60/60 = 4.0s
    //   But there's a stop at the same position as Note 01 (measure 1, pos 0/4).
    //   In tick_to_duration, stop at target tick doesn't add pause (by spec).
    //   So note 01 is at 4.0s

    // Note 02 at tick 1680:
    //   0-960 at 60 BPM = 4.0s
    //   Stop at tick 960: duration = 480 ticks = 4.0s at 60 BPM
    //   The stop is strictly at tick 960 (< 1680), so its duration counts.
    //   After the stop, the position advances to tick 960 + 480 = 1440.
    //   1440-1680 = 240 ticks at 60 BPM = 1.0s
    //   Total = 4.0 + 4.0 + 1.0 = 9.0s.

    let dur1 = chart.data.timing.tick_to_duration(notes[0].0);

    // bmspec: obj 01 at 4s
    assert!(
        (dur1.as_secs_f64() - 4.0).abs() < 1.0,
        "obj 01 approx at 4s, got {dur1:?}"
    );
}

/// bmspec-1-06-STOP: STOP and BPM at the same tick; BPM (priority 2) takes
/// effect before STOP (priority 3). Covers the A11 scenario.
#[test]
fn bmspec_1_06_stop_on_same_beat_as_bpm_matches_expected_timing() {
    let chart = process(
        "#BPM 60\n\
         #BPM01 120\n\
         #STOP01 96\n\
         #00108:0100\n\
         #00109:0100\n\
         #00211:01\n",
    );

    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 1, "should have 1 note");
    // The BPM change to 120 and the STOP are both at tick 960 (measure 1,
    // pos 0/2).
    // Event order: BPM(2) before STOP(3).
    //   tick 0-960 at 60 BPM = 4.0s
    //   STOP 480 ticks at 120 BPM (BPM already changed) = 1.0s.
    //   tick 960-1920 at 120 BPM = 2.0s
    //   Total = 7.0s.
    // If BPM changed after STOP: STOP 480 ticks at 60 BPM = 2.0s, total 8.0s.
    let dur = chart.data.timing.tick_to_duration(notes[0].0);
    assert!(
        (dur.as_secs_f64() - 7.0).abs() < 1e-6,
        "expected note at 7.0s (BPM before STOP), got {dur:?}"
    );
}

/// bmspec-2-LNOBJ: long-note object pairing.
#[test]
fn bmspec_2_lnobj() {
    let chart = process("#LNOBJ XX\n#00111:01XX02XX03XX04XX\n");
    let lns = long_notes(&chart);

    // 4 long notes, each from beat 4 to 4.5 (half-beat spacing in 8 values per
    // measure).
    // #00111:01XX02XX03XX04XX has 8 values.
    // Each value is 960/8 = 120 ticks.
    // 01 at pos 0/8: tick 960, beat 4.
    // XX at pos 1/8: tick 1080, beat 4.5 (long-note end).
    // 02 at pos 2/8: tick 1200, beat 5 (next long-note start).
    // And so on.

    assert_eq!(lns.len(), 4, "should have 4 long notes");

    // Each long note's duration: 1 subdivision = 120 ticks.
    assert_eq!(lns[0], (960, 120), "LN 01: beat 4 to 4.5");
    assert_eq!(lns[1], (1200, 120), "LN 02: beat 5 to 5.5");
    assert_eq!(lns[2], (1440, 120), "LN 03: beat 6 to 6.5");
    assert_eq!(lns[3], (1680, 120), "LN 04: beat 7 to 7.5");
}

/// An `#LNOBJ` marker index placed on an invisible channel (31-49) stays an
/// ordinary invisible note: `#LNOBJ` applies to visible channels 11-29 only.
#[test]
fn lnobj_marker_on_invisible_channel_stays_invisible_note() {
    let chart = process("#WAVAA a.wav\n#WAVBB ghost.wav\n#LNOBJ BB\n#00111:AA\n#00131:BB\n");
    let notes: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Note { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect();
    assert!(
        !notes.iter().any(|k| matches!(k, NoteKind::Long { .. })),
        "an invisible marker must not pair into a long note"
    );
    assert!(
        notes.iter().any(|k| matches!(k, NoteKind::Invisible)),
        "the invisible note carrying the marker stays invisible"
    );
    assert_eq!(
        notes
            .iter()
            .filter(|k| matches!(k, NoteKind::Normal))
            .count(),
        1,
        "the visible note stays an ordinary note"
    );
}

/// bmspec-2-LNTYPE1: dedicated long-note channels.
#[test]
fn bmspec_2_lntype1() {
    let chart = process("#00151:0101020203030404\n");
    let lns = long_notes(&chart);

    // Channel 51, 8 values: 01 01 02 02 03 03 04 04.
    // Values 1-2: 01 at pos 0 and 01 at pos 1, giving long note 01 from pos 0
    // to pos 1.
    // Values 3-4: 02 at pos 2 and 02 at pos 3, giving long note 02 from pos 2
    // to pos 3.
    // And so on.

    assert_eq!(lns.len(), 4, "should have 4 long notes");

    let sub = 960 / 8; // Each subdivision is 120 ticks.
    assert_eq!(lns[0], (960, sub), "LN 01: beat 4 to 4.5");
    assert_eq!(lns[1], (1200, sub), "LN 02: beat 5 to 5.5");
    assert_eq!(lns[2], (1440, sub), "LN 03: beat 6 to 6.5");
    assert_eq!(lns[3], (1680, sub), "LN 04: beat 7 to 7.5");
}

/// bmspec-2-LNTYPE2: MGQ long-note notation.
#[test]
fn bmspec_2_lntype2_pairs_long_notes_correctly() {
    let chart = process(
        "#LNTYPE 2\n\
         #00151:00222222\n\
         #00251:22\n\
         #00351:2200\n",
    );
    let lns = long_notes(&chart);

    // In LNTYPE 2 (MGQ), "00" acts as the release marker.
    // #00151:00222222 gives 4 values: 00 22 22 22 (denom=4).
    //   00 at pos 0/4 = tick 960: skip.
    //   22 at pos 1/4 = tick 1200: start the long note.
    //   22 at pos 2/4 = tick 1440: continue.
    //   22 at pos 3/4 = tick 1680: continue.
    // #00251:22 gives 1 value: 22 (denom=1).
    //   22 at pos 0/1 = tick 1920: continue.
    // #00351:2200 gives 2 values: 22 00 (denom=2).
    //   22 at pos 0/2 = tick 2880: continue.
    //   00 at pos 1/2 = tick 3360: release the long note.
    //
    // A single long note from tick 1200 to tick 3360, duration = 2160.

    assert_eq!(lns.len(), 1, "should have 1 long note");
    assert_eq!(lns[0], (1200, 2160), "LN from beat 5 to beat 14");
}

/// bmspec-3-SCROLL: basics: rate and position accumulation.
#[test]
fn bmspec_3_scroll_basic() {
    let chart = process("#SCROLL02 0.5\n#001SC:02\n");

    // scroll_defs["02"] = 0.5
    // #001SC:02 → channel SC (scroll), value "02" at position 0/1 of measure 1
    let scrolls = scroll_events(&chart);
    assert!(!scrolls.is_empty(), "should have scroll events");
    // The scroll event is at measure 1, pos 0 = tick 960
    assert_eq!(scrolls[0].0, 960, "scroll at beat 4");
    assert!(
        (scrolls[0].1 - 0.5).abs() < 1e-9,
        "scroll rate should be 0.5"
    );

    // Verify scroll position via Player
    let player = Player::new(chart).unwrap();
    // Before scroll: rate 1.0, position = ticks / resolution
    assert!((player.scroll_rate_at(0) - 1.0).abs() < 1e-9);
    assert!((player.scroll_position_at(0) - 0.0).abs() < 1e-9);
    // At beat 4 (tick 960): scroll just changed to 0.5, position = 4 (no delay yet)
    assert!((player.scroll_position_at(960) - 4.0).abs() < 1e-9);
    // At beat 6 (tick 1440): 2 beats at 0.5 → position advances 1 → cum = 5
    assert!(
        (player.scroll_position_at(1440) - 5.0).abs() < 1e-9,
        "expected scroll position 5 at beat 6 (tick 1440)"
    );
}

/// bmspec-3-SCROLL: initial speed specified in measure 0.
#[test]
fn bmspec_3_scroll_initial_speed() {
    let chart = process("#SCROLL02 0.5\n#000SC:02\n");
    let scrolls = scroll_events(&chart);
    assert!(!scrolls.is_empty(), "should have scroll events");
    // Scroll at measure 0, pos 0 → tick 0
    assert_eq!(scrolls[0].0, 0, "initial scroll at tick 0");
    assert!(
        (scrolls[0].1 - 0.5).abs() < 1e-9,
        "scroll rate should be 0.5"
    );

    // Verify position: scroll at tick 0 with rate 0.5
    let player = Player::new(chart).unwrap();
    assert!((player.scroll_rate_at(0) - 0.5).abs() < 1e-9);
    // At tick 0: position = 0, but rate = 0.5 from start
    // At beat -1 (negative beat, tick -240): position = 0.5 * (-1) = -0.5
    // But we can't test negative ticks...
    // At beat 4 (tick 960): 4 beats at 0.5 = position 2.0
    assert!(
        (player.scroll_position_at(960) - 2.0).abs() < 1e-9,
        "expected position 2.0 at beat 4 with 0.5x scroll"
    );
}

/// Helper: process BMS through the full control-flow pipeline.
/// Pins branch values with `SETRANDOM` / `SETSWITCH`.
fn process_with_cf(bms_text: &str) -> BmsChart {
    let source = SourceFile::parse(bms_text);
    let bms = Bms::from_cst(&source.source_file(), |_| 0).bms;
    #[expect(clippy::expect_used, reason = "test helper panics on failure")]
    BmsProcessor::process::<Bme>(&bms)
        .expect("process should succeed")
        .chart
}

/// bmspec-4-RANDOM: SETRANDOM picks a fixed branch.
#[test]
fn bmspec_4_random_setrandom() {
    // SETRANDOM 1 always selects #IF 1.
    let chart = process_with_cf(
        "#SETRANDOM 1\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #BPM 120\n",
    );
    // Only WAV01 (a.wav) should appear in audio_assets.
    assert_eq!(chart.data.audio_assets.len(), 1);
    assert_eq!(chart.data.audio_assets[0].path.to_string_lossy(), "a.wav");
}

/// bmspec-4-RANDOM: SETRANDOM selects the second branch.
#[test]
fn bmspec_4_random_setrandom_second() {
    let chart = process_with_cf(
        "#SETRANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #BPM 120\n",
    );
    assert_eq!(chart.data.audio_assets.len(), 1);
    assert_eq!(chart.data.audio_assets[0].path.to_string_lossy(), "b.wav");
}

/// bmspec-4-RANDOM: consecutive blocks select independently.
#[test]
fn bmspec_4_random_multiple_blocks() {
    // The first block selects #IF 2, the second selects #IF 1 (order does not
    // matter).
    let chart = process_with_cf(
        "#SETRANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #SETRANDOM 1\n\
         #IF 1\n\
         #WAV02 c.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n",
    );
    assert_eq!(chart.data.audio_assets.len(), 2);
    let paths: Vec<_> = chart
        .data
        .audio_assets
        .iter()
        .map(|a| a.path.to_string_lossy().to_string())
        .collect();
    assert!(
        paths.contains(&"b.wav".to_owned()),
        "first block picks branch 2"
    );
    assert!(
        paths.contains(&"c.wav".to_owned()),
        "second block picks branch 1"
    );
}

/// bmspec-4-RANDOM: no WAV is emitted when nothing matches.
#[test]
fn bmspec_4_random_no_match() {
    let chart = process_with_cf(
        "#SETRANDOM 3\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n",
    );
    // RNG=3 matches no IF, so no WAV is emitted.
    assert_eq!(chart.data.audio_assets, [] as [bmsrs_chart::AudioAsset; 0]);
}

/// bmspec-5-Subtitle: basic subtitle.
#[test]
fn bmspec_5_subtitle() {
    let chart = process("#TITLE BY MY SIDE\n#SUBTITLE [TUTORIAL]\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.chart.subtitle, "[TUTORIAL]");
}

/// bmspec-1-07-Basic-Info: full basic-info parsing.
#[test]
fn bmspec_1_07_basic_info() {
    let chart = process(
        "#TITLE BY MY SIDE\n\
         #ARTIST flicknote\n\
         #GENRE Trance Core\n\
         #DIFFICULTY 2\n\
         #PLAYLEVEL 5\n",
    );
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
    assert_eq!(chart.song.genre, "Trance Core");
    assert_eq!(chart.chart.chart_name, "5");
    assert_eq!(chart.chart.level, 5);
}

/// bmspec-1-08-WAV: WAV definition references (verified at the Bms model layer;
/// Chart does not keep the `WavIndex` to file-name mapping).
#[test]
fn bmspec_1_08_wav_references() {
    let bms = parse_bms(
        "#TITLE #WAV test case\n\
         #WAV01 index.mp3\n\
         #WAVZZ wow.mp3\n\
         #WAVAA zz.ogg\n",
    );

    let wav01: bmsrs_bms_parser::WavIndex = "01".parse().unwrap();
    let wav_zz: bmsrs_bms_parser::WavIndex = "ZZ".parse().unwrap();
    let wav_aa: bmsrs_bms_parser::WavIndex = "AA".parse().unwrap();
    let wav02: bmsrs_bms_parser::WavIndex = "02".parse().unwrap();

    assert_eq!(
        bms.audio.wav_files.get(&wav01).map(String::as_str),
        Some("index.mp3")
    );
    assert_eq!(
        bms.audio.wav_files.get(&wav_zz).map(String::as_str),
        Some("wow.mp3")
    );
    assert_eq!(
        bms.audio.wav_files.get(&wav_aa).map(String::as_str),
        Some("zz.ogg")
    );
    assert_eq!(bms.audio.wav_files.get(&wav02).map(String::as_str), None);
}

/// bmspec-6-SPEED: stays at the default 1.0 when unset.
#[test]
fn bmspec_6_speed_without_set() {
    let chart = process("#SPEED01 0.5\n");
    let speeds = speed_events(&chart);
    assert!(speeds.is_empty(), "no SPEED event without #xxxSP channel");
}

/// bmspec-6-SPEED: single-value setting.
#[test]
fn bmspec_6_speed_single() {
    let chart = process("#SPEED01 0.5\n#001SP:0001\n");
    let speeds = speed_events(&chart);
    // #001SP:0001 gives 2 values [00, 01], but SPEED00 is undefined, so it is
    // skipped.
    // Only SPEED01=0.5 is emitted: Position(1, 1, 2), tick 960+480=1440.
    assert_eq!(speeds.len(), 1, "single speed event");
    assert_eq!(speeds[0].0, 1440, "speed at beat 6");
    assert!((speeds[0].1 - 0.5).abs() < 1e-9, "speed rate should be 0.5");
}

/// bmspec-6-SPEED: multiple values (several `SPEEDxx` definitions).
#[test]
fn bmspec_6_speed_multiple() {
    let chart = process(
        "#SPEED01 0.5\n\
         #SPEED02 1.5\n\
         #SPEED03 1\n\
         #001SP:0102\n\
         #002SP:03\n",
    );
    let speeds = speed_events(&chart);
    // #001SP:0102 = [01, 02] gives 2 events (00 is not present in the input).
    //   Position(1, 0, 2)=tick 960(value=01→0.5),
    //   Position(1, 1, 2)=tick 1440(value=02→1.5)
    // #002SP:03 = [03] → 1 event
    //   Position(2, 0, 1)=tick 1920(value=03→1.0)
    assert_eq!(speeds.len(), 3, "should have 3 speed events");
    assert_eq!(speeds[0].0, 960, "speed 01 at beat 4");
    assert!((speeds[0].1 - 0.5).abs() < 1e-9);
    assert_eq!(speeds[1].0, 1440, "speed 02 at beat 6");
    assert!((speeds[1].1 - 1.5).abs() < 1e-9);
    assert_eq!(speeds[2].0, 1920, "speed 03 at beat 8");
    assert!((speeds[2].1 - 1.0).abs() < 1e-9);
}

/// bmspec-6-SPEED: spacing interpolation (verified through
/// `Player::spacing_at`).
#[test]
fn bmspec_6_speed_interpolation() {
    let chart = process(
        "#SPEED01 0.5\n\
         #SPEED02 1.5\n\
         #SPEED03 1\n\
         #001SP:0102\n\
         #002SP:03\n",
    );
    let player = Player::new(chart).unwrap();

    // Before the first keyframe: 1.0 (default).
    assert!((player.spacing_at(0) - 1.0).abs() < 1e-9, "before first kf");

    // keyframe 01: tick 960 (beat 4), rate 0.5
    assert!((player.spacing_at(960) - 0.5).abs() < 1e-9, "kf 01");

    // Interpolation from keyframe 01 to 02: tick 1200 is the midpoint of 960
    // and 1440.
    // 0.5 + (1.5 - 0.5) * 0.5 = 1.0
    assert!(
        (player.spacing_at(1200) - 1.0).abs() < 1e-9,
        "linear interpolation midpoint"
    );

    // keyframe 02: tick 1440 (beat 6), rate 1.5
    assert!((player.spacing_at(1440) - 1.5).abs() < 1e-9, "kf 02");

    // Interpolation from keyframe 02 to 03: tick 1680 (beat 7) is 75% between
    // 1440 and 1920.
    // 1.5 + (1.0 - 1.5) * (1680 - 1440) / (1920 - 1440)
    // = 1.5 + (-0.5) * 0.5 = 1.25
    assert!(
        (player.spacing_at(1680) - 1.25).abs() < 1e-9,
        "interpolation at beat 7"
    );

    // keyframe 03: tick 1920 (beat 8), rate 1.0
    assert!((player.spacing_at(1920) - 1.0).abs() < 1e-9, "kf 03");

    // After the last keyframe: stays at 1.0.
    assert!(
        (player.spacing_at(2400) - 1.0).abs() < 1e-9,
        "after last kf"
    );
}

// BmsCustomEvent integration tests

/// BGA Base Opacity (channel 0B) produces `BmsCustomEvent::BgaOpacity`.
#[test]
fn bga_opacity_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#0010B:FF00000000000000\n");
    let customs: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgaOpacity { layer, opacity }) => {
                Some((*layer, *opacity))
            }
            _ => None,
        })
        .collect();
    assert_eq!(customs.len(), 1, "should produce one BgaOpacity event");
    assert_eq!(customs[0], (bmsrs_chart::BgaLayer::Base, 0xFF));
}

/// The TEXT channel (99) produces `BmsCustomEvent::TextDisplay`.
#[test]
fn text_display_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00199:0100000000000000\n");
    let texts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::TextDisplay { text_index }) => Some(*text_index),
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], 1);
}

/// BGM Volume (channel 97) produces `BmsCustomEvent::BgmVolume`.
#[test]
fn bgm_volume_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00197:8000000000000000\n");
    let vols: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgmVolume { volume }) => Some(*volume),
            _ => None,
        })
        .collect();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0], 0x80);
}

/// The Option channel (A6) produces `BmsCustomEvent::OptionChange`.
#[test]
fn option_change_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A6:0500000000000000\n");
    let opts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::OptionChange { option_id, .. }) => Some(*option_id),
            _ => None,
        })
        .collect();
    assert_eq!(opts.len(), 1);
    assert_eq!(opts[0], 5, "option_id should be decoded as base36");
}

/// ARGB channels (A1-A4) need an `#ARGBxx` definition to resolve colors; they
/// are skipped when undefined.
#[test]
fn argb_custom_event_skips_when_undefined() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A1:0100000000000000\n");
    // ARGB index 01 is undefined (no `#ARGB01` header), so no event should be
    // produced.
    let argb_count = chart
        .data
        .events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Custom(BmsCustomEvent::BgaArgb { .. })))
        .count();
    assert_eq!(argb_count, 0, "undefined ARGB should be skipped");
}

/// The SWBGA channel (A5) produces `BmsCustomEvent::BgaKeyBound`.
#[test]
fn swbga_custom_event() {
    let chart = process(
        "#BPM 120\n#BMP01 bg.png\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A5:0100000000000000\n",
    );
    let keybounds: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgaKeyBound { resource_id }) => Some(*resource_id),
            _ => None,
        })
        .collect();
    assert_eq!(keybounds.len(), 1);
    assert_eq!(
        keybounds[0].get(),
        0,
        "BMP index 01 maps to a zero-based resource_id through bmp_map"
    );
}

/// The Judge channel (A0) produces `BmsCustomEvent::JudgeOverride`.
#[test]
fn judge_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A0:5000000000000000\n");
    let judges: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::JudgeOverride { rank }) => Some(*rank),
            _ => None,
        })
        .collect();
    assert_eq!(judges.len(), 1);
    // "50" in base36 = 5*36 + 0 = 180
    assert_eq!(judges[0], 180, "base36 '50' should decode to 180");
}

/// The Key Volume channel (98) produces `BmsCustomEvent::KeyVolume`.
#[test]
fn key_volume_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00198:4000000000000000\n");
    let vols: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::KeyVolume { volume }) => Some(*volume),
            _ => None,
        })
        .collect();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0], 0x40, "hex '40' should decode to 64");
}

/// The Video Seek channel (05) looks up the `#SEEK` definition table and
/// produces `BmsCustomEvent::VideoSeek`, with position taken from the
/// definition's millisecond value (C4: no longer the raw Base36 value).
#[test]
fn video_seek_custom_event() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#SEEK03 500\n#00101:1100000000000000\n#00105:0300000000000000\n",
    );
    let seeks: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::VideoSeek { position }) => Some(*position),
            _ => None,
        })
        .collect();
    assert_eq!(seeks.len(), 1);
    assert_eq!(seeks[0], 500, "#SEEK03 500 should map to 500 milliseconds");
}

/// When the Video Seek channel references an undefined `#SEEK` id, the event is
/// skipped (C4).
#[test]
fn video_seek_undefined_skipped() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00105:0300000000000000\n");
    let seek_count = chart
        .data
        .events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Custom(BmsCustomEvent::VideoSeek { .. })))
        .count();
    assert_eq!(seek_count, 0, "undefined #SEEK ids should be skipped");
}

/// Inserting custom events does not affect native event ordering or queries.
#[test]
fn custom_events_dont_affect_note_queries() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00111:0100000000000000\n#0010B:FF00000000000000\n");
    let player = Player::new(chart).unwrap();
    let note_count = player
        .events_in_range(0..2000)
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Note { .. }))
        .count();
    assert_eq!(note_count, 1, "note should still be present");
}

// F1: non_event_data lookup normalization (lowercase Base36 indices)

/// The ARGB channel (A1) references a lowercase-defined `#ARGB` with a
/// lowercase index; normalization should make it match (F1).
#[test]
fn argb_lowercase_index_resolves_after_normalize() {
    // #ARGBaa defines a lowercase id; channel A1 references it with lowercase
    // "aa".
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#ARGBaa 10,20,30,40\n#00101:0100000000000000\n#001A1:aa00000000000000\n",
    );
    let argbs: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgaArgb { a, r, g, b, .. }) => Some((*a, *r, *g, *b)),
            _ => None,
        })
        .collect();
    assert_eq!(
        argbs.len(),
        1,
        "lowercase ARGB index should match after normalization"
    );
    assert_eq!(argbs[0], (10, 20, 30, 40));
}

/// The BGA `KeyBound` channel (A5) references a lowercase-defined `#BMP` with a
/// lowercase index (F1).
#[test]
fn keybound_lowercase_index_resolves_after_normalize() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#BMPaa bg.png\n#00101:0100000000000000\n#001A5:aa00000000000000\n",
    );
    let keybound_count = chart
        .data
        .events
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Custom(BmsCustomEvent::BgaKeyBound { .. })
            )
        })
        .count();
    assert_eq!(
        keybound_count, 1,
        "lowercase KeyBound index should match bmp_map after normalization"
    );
}

/// The Option channel (A6) and the `#CHANGEOPTION` definition both use
/// lowercase; normalization keeps both sides consistent (F1+F2).
#[test]
fn option_lowercase_def_and_ref_resolve() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#CHANGEOPTIONaa opt_value\n#00101:0100000000000000\n#001A6:aa00000000000000\n",
    );
    let opts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::OptionChange { value, .. }) => Some(value.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        opts.len(),
        1,
        "lowercase Option definition and reference should match after normalization"
    );
    assert_eq!(opts[0], "opt_value");
}

// F3a: #LNMODE to ln_type_hint

/// Each `#LNMODE` value maps to an `LnTypeHint` (F3a).
#[test]
fn ln_mode_maps_to_ln_type_hint() {
    let base = "#BPM 120\n#WAV01 kick.wav\n#00101:0100000000000000\n";
    assert_eq!(
        process(&format!("#LNMODE 1\n{base}")).data.ln_type_hint,
        bmsrs_chart::LnTypeHint::Ln,
    );
    assert_eq!(
        process(&format!("#LNMODE 2\n{base}")).data.ln_type_hint,
        bmsrs_chart::LnTypeHint::Cn,
    );
    assert_eq!(
        process(&format!("#LNMODE 3\n{base}")).data.ln_type_hint,
        bmsrs_chart::LnTypeHint::Hcn,
    );
    // Defaults to Ln when #LNMODE is not declared.
    assert_eq!(process(base).data.ln_type_hint, bmsrs_chart::LnTypeHint::Ln,);
}

// C1: #BGA / #@BGA to BgaResource.crop

/// A `#BGA` definition produces a BGA resource with a crop (C1).
#[test]
fn bga_crop_def_attaches_crop_rect() {
    // #BMP01 is the source; #BGA01 crops from BMP 1 (decimal) with rectangle
    // (0,0)-(100,100) and offset (10,20).
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#BMP01 src.png\n#BGA01 1 0 0 100 100 10 20\n#00104:0100000000000000\n",
    );
    let cropped: Vec<_> = chart
        .chart
        .bga_resources
        .iter()
        .filter_map(|r| r.crop.map(|c| (r.path.to_string_lossy().into_owned(), c)))
        .collect();
    assert_eq!(cropped.len(), 1, "there should be one cropped BGA resource");
    let (path, crop) = &cropped[0];
    assert_eq!(path, "src.png", "path comes from the source BMP");
    assert_eq!((crop.x1, crop.y1, crop.x2, crop.y2), (0, 0, 100, 100));
    assert_eq!((crop.dx, crop.dy), (10, 20));
}

/// `#@BGA` (width/height form) is normalized to the bottom-right-corner form
/// (C1).
#[test]
fn at_bga_wh_form_normalizes_to_xy2() {
    // #@BGA: sx=0, sy=0, w=50, h=40 gives x2=50 and y2=40.
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#BMP01 src.png\n#@BGA01 1 0 0 50 40 0 0\n#00104:0100000000000000\n",
    );
    let cropped: Vec<_> = chart
        .chart
        .bga_resources
        .iter()
        .filter_map(|r| r.crop)
        .collect();
    assert_eq!(cropped.len(), 1);
    let c = cropped[0];
    assert_eq!(
        (c.x1, c.y1, c.x2, c.y2),
        (0, 0, 50, 40),
        "w/h should become x2/y2"
    );
}

// C2: #VIDEOFILE / #MOVIE to VideoAsset

/// `#VIDEOFILE` produces a looping video asset (C2).
#[test]
fn videofile_produces_looping_video_asset() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#VIDEOFILE bg.mp4\n#VIDEOf/s 30\n#VIDEOCOLORS 16\n#VIDEODLY 5\n#00101:0100000000000000\n",
    );
    let video = chart
        .chart
        .video
        .as_ref()
        .expect("video asset should exist");
    assert_eq!(video.path.to_string_lossy(), "bg.mp4");
    assert!(video.loop_playback, "#VIDEOFILE should loop");
    assert_eq!(video.fps, Some(30.0));
    assert_eq!(video.colors, Some(16));
    assert_eq!(video.delay_frames, Some(5));
}

/// `#MOVIE` produces a single-play video asset (C2).
#[test]
fn movie_produces_non_looping_video_asset() {
    let chart = process("#BPM 120\n#WAV01 kick.wav\n#MOVIE intro.mp4\n#00101:0100000000000000\n");
    let video = chart
        .chart
        .video
        .as_ref()
        .expect("video asset should exist");
    assert_eq!(video.path.to_string_lossy(), "intro.mp4");
    assert!(!video.loop_playback, "#MOVIE should play once");
    assert!(
        video.fps.is_none(),
        "fps is None when #VIDEOf/s is not declared"
    );
}
