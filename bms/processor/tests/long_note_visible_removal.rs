#![expect(missing_docs, reason = "integration test")]

use bms_parser::{KeyType, NoteEvent, Position};
use bms_processor::BmsProcessor;
use bms_processor::layout::Bme;
use bmsrs_chart::{EventKind, NoteKind};

/// visible note inside LN range should be removed;
/// visible note outside LN range should remain.
#[test]
fn visible_note_inside_lntype1_range_removed() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());
    bms.audio
        .wav_files
        .insert("CC".parse().unwrap(), "c.wav".to_owned());

    // ch51 (long start) at slot 2 with wav BB
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 2, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    // ch51 (long end) at slot 6 with wav BB
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 6, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });

    // ch11 (visible) at slot 4 with wav CC — inside LN range (tick 480, LN is 240-719)
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 4, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "CC".parse().unwrap(),
    });
    // ch11 (visible) at measure 1 slot 0 with wav AA — outside LN range
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "AA".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let normal_ticks: Vec<u64> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Normal,
                ..
            } = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();

    // Only the outside visible (tick 960) should remain.
    assert_eq!(normal_ticks.len(), 1, "only 1 normal note should remain");
    assert_eq!(
        normal_ticks[0], 960,
        "the outside visible at tick 960 should remain"
    );
}

/// visible note at same tick as LN start → suppressed (covered by LN range).
#[test]
fn visible_note_at_ln_start_tick_suppressed() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());

    // LN from slot 0 to slot 4
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 4, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });

    // Visible at the same tick as LN start
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "AA".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let has_normal = chart.data.events.iter().any(|e| {
        matches!(
            &e.kind,
            EventKind::Note {
                kind: NoteKind::Normal,
                ..
            }
        )
    });
    assert!(!has_normal, "visible at LN start should be suppressed");
}

/// visible note at same tick as LN end → suppressed (covered by LN range).
#[test]
fn visible_note_at_ln_end_tick_suppressed() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());

    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 4, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });

    // Visible at the same tick as LN end (slot 4 = tick 480)
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 4, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "AA".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let has_normal = chart.data.events.iter().any(|e| {
        matches!(
            &e.kind,
            EventKind::Note {
                kind: NoteKind::Normal,
                ..
            }
        )
    });
    assert!(!has_normal, "visible at LN end should be suppressed");
}

/// visible note on a different lane should NOT be suppressed.
#[test]
fn visible_note_on_different_lane_preserved() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());
    bms.audio
        .wav_files
        .insert("CC".parse().unwrap(), "c.wav".to_owned());

    // LN on lane 1
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 4, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });

    // Visible note on lane 2 (different lane)
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 2, 8),
        player: 1,
        lane: 2,
        key_type: KeyType::Visible,
        wav_id: "CC".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let normal_ticks: Vec<u64> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Normal,
                ..
            } = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        normal_ticks.len(),
        1,
        "visible on different lane should remain"
    );
    assert_eq!(
        normal_ticks[0], 240,
        "visible at tick 240 on lane 2 should remain"
    );
}

/// visible note at an unpaired LN start tick → suppressed (B3).
#[test]
fn visible_note_at_unpaired_ln_start_suppressed() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());

    // Only a single LN event (unpaired, odd count)
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 2, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });

    // Visible note at the same tick
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 2, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "AA".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let has_normal = chart.data.events.iter().any(|e| {
        matches!(
            &e.kind,
            EventKind::Note {
                kind: NoteKind::Normal,
                ..
            }
        )
    });
    assert!(
        !has_normal,
        "visible at unpaired LN start should be suppressed"
    );
}

/// visible note in the gap between two LN ranges on the same lane → preserved.
#[test]
fn visible_note_in_gap_between_lns_preserved() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());
    bms.audio
        .wav_files
        .insert("CC".parse().unwrap(), "c.wav".to_owned());

    // LN1: slot 0 → slot 2
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 2, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    // LN2: slot 6 → next measure slot 2
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 6, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(1, 2, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });

    // Visible note in the gap between LN1 (0-240) and LN2 (720-1200)
    // Gap is tick 240 to 720; slot 4 = tick 480
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 4, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "CC".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let normal_ticks: Vec<u64> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Normal,
                ..
            } = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(normal_ticks.len(), 1, "visible in gap should remain");
    assert_eq!(
        normal_ticks[0], 480,
        "visible at tick 480 in gap should remain"
    );
}

/// LNOBJ mode: visible note inside LN range should be suppressed.
#[test]
fn lnobj_visible_note_inside_range_removed() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());
    bms.audio
        .wav_files
        .insert("CC".parse().unwrap(), "c.wav".to_owned());
    let ln_obj: bms_tokenizer::LnObjIndex = "FF".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);

    // LN: start at slot 0 (wav AA), end at slot 4 (wav FF — LNOBJ marker)
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "AA".parse().unwrap(),
    });
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 4, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "FF".parse().unwrap(),
    });

    // Visible note inside LN range (slot 2, tick 240)
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 2, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "BB".parse().unwrap(),
    });

    // Visible note outside LN range (measure 1 slot 0)
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "CC".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let normal_ticks: Vec<u64> = chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Normal,
                ..
            } = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    // The LN consumed note at slot 0 is removed via consumed index.
    // The visible note at slot 2 is inside LN range → removed.
    // Only the outside note at measure 1 remains.
    assert_eq!(normal_ticks.len(), 1, "only one normal note should remain");
    assert_eq!(
        normal_ticks[0], 960,
        "outside LN note at tick 960 should remain"
    );
}

/// LNTYPE 2 (MGQ): visible note inside LN range should be suppressed.
#[test]
fn lntype2_visible_note_inside_range_removed() {
    let mut bms = bms_parser::Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("AA".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("BB".parse().unwrap(), "b.wav".to_owned());
    bms.audio
        .wav_files
        .insert("CC".parse().unwrap(), "c.wav".to_owned());

    bms.gameplay.ln_type = Some(bms_tokenizer::LnType::Type2);

    // LN: AA at slot 0, BB at slot 2, 00 at slot 4 → LN is AA..00 (tick 0..480)
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane: 1,
            wav_id: "AA".parse().unwrap(),
        });
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 2, 8),
            player: 1,
            lane: 1,
            wav_id: "BB".parse().unwrap(),
        });
    bms.messages
        .long_note_events
        .push(bms_parser::LongNoteEvent {
            position: Position::new(0, 4, 8),
            player: 1,
            lane: 1,
            wav_id: "00".parse().unwrap(),
        });

    // Visible note inside LN range (slot 2, tick 240)
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 2, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "BB".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap();

    let has_normal = chart.data.events.iter().any(|e| {
        matches!(
            &e.kind,
            EventKind::Note {
                kind: NoteKind::Normal,
                ..
            }
        )
    });
    assert!(!has_normal, "visible inside LN range should be suppressed");
}
