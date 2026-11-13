//! Integration tests for `bmsrs-bmson-processor`.

#![expect(clippy::panic_in_result_fn, reason = "test helpers and assertions")]

use std::num::NonZeroU8;
use std::path::PathBuf;
use std::time::Duration;

use bmsrs_bmson_model::{
    BGA, BGAEvent, BGAHeader, BGASequence, BGASequenceFrame, BpmEvent, ChartData, ChartInfo,
    KeyChannel, KeyNote, LnType, MineChannel, MineNote, ModeHint, NoteEvent, SongInfo,
    SoundChannel, StopEvent,
};
use bmsrs_bmson_processor::layout::{Beat, GenericLayout, Pms};
use bmsrs_bmson_processor::{BmsonNoteExt, BmsonProcessWarning, BmsonProcessor, ProcessError};
use bmsrs_chart::mode::{Lane, NoteSide};
use bmsrs_chart::{
    BgaLayer, BgaResourceContent, Event, EventKind, LnJudgeHint, LnLifeHint, LnTypeHint, Note,
    NoteKind, NoteRole,
};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

/// Build a simple v2 BMSON chart for tests.
fn make_simple_bmson() -> bmsrs_bmson_model::Bmson {
    bmsrs_bmson_model::Bmson {
        version: "2.0.0".to_owned(),
        song_info: SongInfo {
            title: "Test".to_owned(),
            artist: "Test Artist".to_owned(),
            genre: "Test Genre".to_owned(),
        },
        chart_info: ChartInfo {
            subtitle: String::new(),
            subartists: vec![],
            chart_name: "HYPER".to_owned(),
            level: 8,
            back_image: None,
            eyecatch_image: None,
            banner_image: None,
            preview_music: None,
            title_image: None,
            bga: BGA::default(),
        },
        chart_data: ChartData {
            mode_hint: ModeHint::Beat7k,
            ln_type_hint: Some(LnType::Ln),
            ln_judge_hint: bmsrs_bmson_model::LnJudge::Normal,
            ln_life_hint: bmsrs_bmson_model::LnLife::Normal,
            init_bpm: 140.0,
            resolution: 240,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            lines: None,
            bpm_events: vec![],
            stop_events: vec![],
            sound_channels: vec![],
            judge_deltas: None,
            life_deltas: None,
        },
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    }
}

#[test]
fn process_basic_note() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("kick.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 0,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                side,
                lane,
                kind,
                ext: BmsonNoteExt { .. },
                ..
            }) = &e.kind
            {
                Some((e.tick(), *side, *lane, *kind))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0], (0, NoteSide::P1, key(1), NoteKind::Normal));
    Ok(())
}

#[test]
fn process_layered_notes_fuse_audio_layers() -> TestResult {
    let mut bmson = make_simple_bmson();
    for name in ["a.wav", "b.wav"] {
        bmson.chart_data.sound_channels.push(SoundChannel {
            name: PathBuf::from(name),
            note_events: vec![NoteEvent {
                x: 1,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            }],
        });
    }

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;
    let notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Note(Note { .. })))
        .collect();

    assert_eq!(notes.len(), 1);
    let EventKind::Note(Note { audio, .. }) = &notes[0].kind else {
        panic!("expected a note event");
    };
    assert_eq!(audio.len(), 2);
    Ok(())
}

#[test]
fn process_layered_long_notes_warn_on_length_mismatch() -> TestResult {
    let mut bmson = make_simple_bmson();
    for (name, length) in [("a.wav", 240), ("b.wav", 480)] {
        bmson.chart_data.sound_channels.push(SoundChannel {
            name: PathBuf::from(name),
            note_events: vec![NoteEvent {
                x: 1,
                y: 0,
                l: length,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            }],
        });
    }

    let processed = BmsonProcessor::process::<Beat>(&bmson)?;

    assert!(
        processed
            .warnings
            .contains(&BmsonProcessWarning::LayeredNoteLengthMismatch {
                tick: 0,
                first: 240,
                other: 480,
            })
    );
    Ok(())
}

#[test]
fn process_up_note_sets_release_role() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("release.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 0,
            c: false,
            t: None,
            up: Some(true),
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;
    let role = chart.data().events().iter().find_map(|event| {
        if let EventKind::Note(Note { role, .. }) = &event.kind {
            Some(*role)
        } else {
            None
        }
    });

    assert_eq!(role, Some(NoteRole::Release));
    Ok(())
}

#[test]
fn beatoraja_ln_mode_overrides_ln_type_hint() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("ln.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 240,
            c: false,
            t: Some(bmsrs_bmson_model::LnMode::Cn),
            up: None,
            ln_type_hint: Some(LnType::Hcn),
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;
    let hint = chart.data().events().iter().find_map(|event| {
        if let EventKind::Note(Note { ext, .. }) = &event.kind {
            ext.ln_type_hint
        } else {
            None
        }
    });

    assert_eq!(hint, Some(LnTypeHint::Cn));
    Ok(())
}

#[test]
fn process_beat_mode_sets_key_count() -> TestResult {
    let bmson = make_simple_bmson();

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    assert_eq!(chart.chart_info().key_count, Some(7));
    Ok(())
}

#[test]
fn process_maps_subartists_and_title_image_to_chart_info() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_info.subartists = vec!["music:Composer".to_owned()];
    bmson.chart_info.title_image = Some(PathBuf::from("title.png"));

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    assert_eq!(chart.chart_info().subartists, vec!["music:Composer"]);
    assert_eq!(chart.chart_info().title_image.as_deref(), Some("title.png"));
    Ok(())
}

#[test]
fn process_rejects_parent_directory_path() {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("../evil.wav"),
        note_events: vec![],
    });

    let result = BmsonProcessor::process::<Beat>(&bmson);

    assert!(matches!(
        result,
        Err(ProcessError::UnsafePath {
            reason: "parent directory traversal",
            ..
        })
    ));
}

#[test]
fn process_rejects_absolute_path() {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("/etc/passwd"),
        note_events: vec![],
    });

    let result = BmsonProcessor::process::<Beat>(&bmson);

    assert!(matches!(
        result,
        Err(ProcessError::UnsafePath {
            reason: "absolute path",
            ..
        })
    ));
}

#[test]
fn process_rejects_nul_path() {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("bad\0.wav"),
        note_events: vec![],
    });

    let result = BmsonProcessor::process::<Beat>(&bmson);

    assert!(matches!(
        result,
        Err(ProcessError::UnsafePath {
            reason: "NUL byte",
            ..
        })
    ));
}

#[test]
fn process_normalizes_backslash_path() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("snd\\kick.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 0,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    assert_eq!(
        chart.data().audio_assets()[0].path.as_ref(),
        PathBuf::from("snd/kick.wav")
    );
    Ok(())
}

#[test]
fn process_beat_layout_maps_both_sides() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("p1.wav"),
        note_events: vec![
            NoteEvent {
                x: 1,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            },
            NoteEvent {
                x: 8,
                y: 240,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            },
            NoteEvent {
                x: 9,
                y: 480,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            },
        ],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let note_positions: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                side,
                lane,
                ext: BmsonNoteExt { .. },
                ..
            }) = &e.kind
            {
                Some((*side, *lane))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(note_positions.len(), 3);
    assert!(note_positions.contains(&(NoteSide::P1, key(1))));
    assert!(note_positions.contains(&(NoteSide::P1, Lane::Scratch(nz(1)))));
    assert!(note_positions.contains(&(NoteSide::P2, key(1))));
    Ok(())
}

#[test]
fn process_pms_layout_single_player() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("popn.wav"),
        note_events: (1..=9)
            .map(|i| NoteEvent {
                x: i,
                y: (i - 1) * 240,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            })
            .collect(),
    });

    let chart = BmsonProcessor::process::<Pms>(&bmson)?.value;

    let sides: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                side,
                ext: BmsonNoteExt { .. },
                ..
            }) = &e.kind
            {
                Some(*side)
            } else {
                None
            }
        })
        .collect();

    // PMS mode: every note is Player 1.
    assert_eq!(sides.len(), 9);
    assert!(sides.iter().all(|s| *s == NoteSide::P1));
    Ok(())
}

#[test]
fn process_generic_layout_n_keys() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("generic.wav"),
        note_events: (1..=5)
            .map(|i| NoteEvent {
                x: i,
                y: (i - 1) * 240,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            })
            .collect(),
    });

    let chart = BmsonProcessor::process_nkeys(&bmson, 5)?.value;

    let positions: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                side,
                lane,
                ext: BmsonNoteExt { .. },
                ..
            }) = &e.kind
            {
                Some((*side, *lane))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(positions.len(), 5);
    assert!(positions.contains(&(NoteSide::P1, key(1))));
    assert!(positions.contains(&(NoteSide::P1, key(5))));
    Ok(())
}

#[test]
fn process_default_other_mode_hint_produces_warning() -> TestResult {
    // A non-standard extension hint falls back to the Beat layout, and the
    // fallback itself must be observable.
    let mut bmson = make_simple_bmson();
    bmson.chart_data.mode_hint = ModeHint::Other("somewhere-new".to_owned());
    let warnings = BmsonProcessor::process_default(&bmson)?.warnings;
    assert_eq!(
        warnings,
        vec![BmsonProcessWarning::UnknownModeHint {
            hint: "somewhere-new".to_owned()
        }]
    );
    Ok(())
}

#[test]
fn process_default_routes_by_mode_hint() -> TestResult {
    // Beat
    let mut bmson_beat = make_simple_bmson();
    bmson_beat.chart_data.mode_hint = ModeHint::Beat7k;
    bmson_beat.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("a.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 0,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });
    let chart_beat = BmsonProcessor::process_default(&bmson_beat)?.value;
    assert!(chart_beat.data().events().iter().any(|e| matches!(
        &e.kind,
        EventKind::Note(Note {
            ext: BmsonNoteExt { .. },
            ..
        })
    )));

    // PMS
    let mut bmson_pms = make_simple_bmson();
    bmson_pms.chart_data.mode_hint = ModeHint::Popn9k;
    bmson_pms.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("b.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 0,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });
    let chart_pms = BmsonProcessor::process_default(&bmson_pms)?.value;
    assert!(chart_pms.data().events().iter().any(|e| matches!(
        &e.kind,
        EventKind::Note(Note {
            ext: BmsonNoteExt { .. },
            ..
        })
    )));

    Ok(())
}

#[test]
fn process_bpm_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson
        .chart_data
        .bpm_events
        .push(BpmEvent { y: 960, bpm: 180.0 });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let bpm_evts: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Bpm { bpm } = &e.kind {
                Some((e.tick(), *bpm))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(bpm_evts.len(), 1);
    assert_eq!(bpm_evts[0], (960, 180.0));
    Ok(())
}

#[test]
fn process_stop_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.stop_events.push(StopEvent {
        y: 480,
        duration: 192,
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let stop_evts: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Stop { duration } = &e.kind {
                Some((e.tick(), *duration))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(stop_evts.len(), 1);
    assert_eq!(
        stop_evts[0],
        (480, Duration::from_secs_f64(192.0 / 240.0 * 60.0 / 140.0))
    );
    Ok(())
}

#[test]
fn same_pulse_stop_events_accumulate_durations() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.stop_events.push(StopEvent {
        y: 480,
        duration: 192,
    });
    bmson.chart_data.stop_events.push(StopEvent {
        y: 480,
        duration: 96,
    });
    bmson.chart_data.stop_events.push(StopEvent {
        y: 960,
        duration: 240,
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let stop_evts: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Stop { duration } = &e.kind {
                Some((e.tick(), *duration))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        stop_evts,
        vec![
            (480, Duration::from_secs_f64(288.0 / 240.0 * 60.0 / 140.0)),
            (960, Duration::from_secs_f64(240.0 / 240.0 * 60.0 / 140.0)),
        ]
    );
    Ok(())
}

#[test]
fn scroll_events_converts_negative_values() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson
        .scroll_events
        .push(bmsrs_bmson_model::ScrollEvent { y: 0, rate: -1.0 });
    bmson
        .scroll_events
        .push(bmsrs_bmson_model::ScrollEvent { y: 960, rate: -0.5 });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let scroll_evts: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Scroll { rate } = &e.kind {
                Some((e.tick(), *rate))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(scroll_evts.len(), 2);
    assert_eq!(scroll_evts[0], (0, -1.0));
    assert_eq!(scroll_evts[1], (960, -0.5));
    Ok(())
}

#[test]
fn key_channels_produces_invisible_notes() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.key_channels.push(KeyChannel {
        name: PathBuf::from("key.wav"),
        notes: vec![KeyNote { x: 1, y: 0 }, KeyNote { x: 2, y: 480 }],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let invisible_notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                kind: NoteKind::Invisible,
                lane,
                ext: BmsonNoteExt { .. },
                ..
            }) = &e.kind
            {
                Some((e.tick(), *lane))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(invisible_notes.len(), 2);
    assert_eq!(invisible_notes[0], (0, key(1)));
    assert_eq!(invisible_notes[1], (480, key(2)));
    Ok(())
}

#[test]
fn mine_channels_produces_damage_notes() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.mine_channels.push(MineChannel {
        name: PathBuf::from("mine.wav"),
        notes: vec![
            MineNote {
                x: 1,
                y: 0,
                damage: 2.5,
            },
            MineNote {
                x: 2,
                y: 480,
                damage: 1.0,
            },
        ],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let mine_notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                kind: NoteKind::Mine { damage },
                lane,
                ext: BmsonNoteExt { .. },
                ..
            }) = &e.kind
            {
                Some((e.tick(), *lane, damage.get()))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(mine_notes.len(), 2);
    assert_eq!(mine_notes[0], (0, key(1), 2.5));
    assert_eq!(mine_notes[1], (480, key(2), 1.0));
    Ok(())
}

#[test]
fn process_scroll_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson
        .scroll_events
        .push(bmsrs_bmson_model::ScrollEvent { y: 1920, rate: 2.0 });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let scroll_evts: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Scroll { rate } = &e.kind {
                Some((e.tick(), *rate))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(scroll_evts.len(), 1);
    assert_eq!(scroll_evts[0], (1920, 2.0));
    Ok(())
}

#[test]
fn process_bga_events() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_info.bga = BGA {
        bga_header: vec![BGAHeader {
            id: 1,
            name: PathBuf::from("bg.png"),
        }],
        bga_sequence: None,
        bga_events: vec![BGAEvent { y: 0, id: 1 }],
        layer_events: vec![BGAEvent { y: 480, id: 1 }],
        poor_events: vec![BGAEvent { y: 960, id: 1 }],
    };

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let bga_evts: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Bga { layer, .. } = &e.kind {
                Some((e.tick(), *layer))
            } else {
                None
            }
        })
        .collect();

    assert!(bga_evts.contains(&(0, BgaLayer::Base)));
    assert!(bga_evts.contains(&(480, BgaLayer::Layer)));
    assert!(bga_evts.contains(&(960, BgaLayer::Poor)));

    assert_eq!(chart.chart_info().bga_resources.len(), 1);
    // Densification: raw id 1 becomes index 0; BGA resources no longer store
    // their own id.
    assert!(matches!(
        chart.chart_info().bga_resources[0],
        BgaResourceContent::File { crop: None, .. }
    ));
    Ok(())
}

#[test]
fn process_long_note() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("ln.wav"),
        note_events: vec![NoteEvent {
            x: 1,
            y: 0,
            l: 480,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        }],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let lns: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                kind: NoteKind::Long { duration },
                ext: BmsonNoteExt { .. },
                ..
            }) = &e.kind
            {
                Some(*duration)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(lns, vec![480]);
    Ok(())
}

#[test]
fn process_bgm_note_skips_playable_pulse() -> TestResult {
    // BGM notes (x=0) are dropped when they share a tick with playable notes.
    let mut bmson = make_simple_bmson();
    bmson.chart_data.sound_channels.push(SoundChannel {
        name: PathBuf::from("playable.wav"),
        note_events: vec![
            NoteEvent {
                x: 1,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            },
            NoteEvent {
                x: 0,
                y: 0,
                l: 0,
                c: false,
                t: None,
                up: None,
                ln_type_hint: None,
                ln_judge_hint: None,
                ln_life_hint: None,
                vol: None,
                pan: None,
            },
        ],
    });

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let event_count = chart
        .data()
        .events()
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Note(Note {
                    ext: BmsonNoteExt { .. },
                    ..
                })
            )
        })
        .count();

    // There should be only 1 playable note (the BGM note is dropped).
    assert_eq!(event_count, 1);
    Ok(())
}

#[test]
fn process_zero_bpm_returns_error() {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.init_bpm = 0.0;
    let result = BmsonProcessor::process::<Beat>(&bmson);
    assert!(result.is_err());
}

#[test]
fn process_nan_bpm_returns_error() {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.init_bpm = f64::NAN;
    let result = BmsonProcessor::process::<Beat>(&bmson);
    assert!(result.is_err());
}

#[test]
fn metadata_transferred() -> TestResult {
    let bmson = make_simple_bmson();
    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    assert_eq!(chart.song().title, "Test");
    assert_eq!(chart.song().artist, "Test Artist");
    assert_eq!(chart.song().genre, "Test Genre");
    assert_eq!(chart.chart_info().chart_name, "HYPER");
    assert_eq!(chart.chart_info().level, 8);
    Ok(())
}

#[test]
fn bar_lines_auto_generated() -> TestResult {
    let bmson = make_simple_bmson();
    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    let bars: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Bar))
        .map(Event::tick)
        .collect();

    assert_ne!(bars, [] as [u64; 0]);
    assert_eq!(bars[0], 0);
    assert_eq!(bars[1], 960);
    Ok(())
}

#[test]
fn beat_layout_mapping() {
    // Test the Beat layout's x to (side, lane) mapping.
    assert_eq!(Beat::from_bmson(1), Some((NoteSide::P1, key(1))));
    assert_eq!(Beat::from_bmson(7), Some((NoteSide::P1, key(7))));
    assert_eq!(
        Beat::from_bmson(8),
        Some((NoteSide::P1, Lane::Scratch(nz(1))))
    );
    assert_eq!(Beat::from_bmson(9), Some((NoteSide::P2, key(1))));
    assert_eq!(Beat::from_bmson(15), Some((NoteSide::P2, key(7))));
    assert_eq!(
        Beat::from_bmson(16),
        Some((NoteSide::P2, Lane::Scratch(nz(1))))
    );
    assert_eq!(Beat::from_bmson(0), None);
    assert_eq!(Beat::from_bmson(17), None);
}

#[test]
fn pms_layout_mapping() {
    assert_eq!(Pms::from_bmson(1), Some((NoteSide::P1, key(1))));
    assert_eq!(Pms::from_bmson(9), Some((NoteSide::P1, key(9))));
    assert_eq!(Pms::from_bmson(0), None);
    assert_eq!(Pms::from_bmson(10), None);
}

#[test]
fn generic_layout_mapping() {
    let layout = GenericLayout { keys: 5 };
    assert_eq!(layout.map_x(1), Some((NoteSide::P1, key(1))));
    assert_eq!(layout.map_x(5), Some((NoteSide::P1, key(5))));
    assert_eq!(layout.map_x(0), None);
    assert_eq!(layout.map_x(6), None);
}

#[test]
fn bar_lines_from_explicit_lines() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.lines = Some(vec![
        bmsrs_bmson_model::BarLine { y: 0 },
        bmsrs_bmson_model::BarLine { y: 960 },
        bmsrs_bmson_model::BarLine { y: 1920 },
    ]);

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;
    let bars: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Bar))
        .map(Event::tick)
        .collect();

    assert_eq!(bars, vec![0, 960, 1920]);
    Ok(())
}

#[test]
fn note_ext_fields() {
    let ext = BmsonNoteExt {
        vol: Some(-50),
        pan: Some(100),
        ln_type_hint: Some(LnTypeHint::Cn),
        ln_judge_hint: Some(LnJudgeHint::Ticks),
        ln_life_hint: Some(LnLifeHint::Normal),
    };
    assert_eq!(ext.vol, Some(-50));
    assert_eq!(ext.pan, Some(100));
}

#[test]
fn chart_level_ln_hints_propagate() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.ln_type_hint = Some(bmsrs_bmson_model::LnType::Cn);
    bmson.chart_data.ln_judge_hint = bmsrs_bmson_model::LnJudge::Ticks;
    bmson.chart_data.ln_life_hint = bmsrs_bmson_model::LnLife::Ticks;

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;
    assert_eq!(chart.data().ln_type_hint(), Some(LnTypeHint::Cn));
    assert_eq!(chart.data().ln_judge_hint(), LnJudgeHint::Ticks);
    assert_eq!(chart.data().ln_life_hint(), LnLifeHint::Ticks);
    Ok(())
}

#[test]
fn zero_resolution_falls_back_to_240_with_warning() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_data.resolution = 0;

    let processed = BmsonProcessor::process::<Beat>(&bmson)?;
    assert_eq!(processed.value.data().resolution(), 240);
    assert!(
        processed
            .warnings
            .iter()
            .any(|w| matches!(w, BmsonProcessWarning::ZeroResolutionFallback)),
        "resolution fallback should warn"
    );
    Ok(())
}

/// A POOR event referencing a `bga_sequence` clip id plays the clip as a
/// miss-animation sequence resource (jbms-parser parity).
#[test]
fn process_poor_event_referencing_sequence_clip() -> TestResult {
    let mut bmson = make_simple_bmson();
    bmson.chart_info.bga = BGA {
        bga_header: vec![
            BGAHeader {
                id: 1,
                name: PathBuf::from("a.png"),
            },
            BGAHeader {
                id: 2,
                name: PathBuf::from("b.png"),
            },
        ],
        bga_sequence: Some(vec![BGASequence {
            id: 7,
            sequence: vec![
                BGASequenceFrame {
                    time: 250,
                    id: Some(1),
                },
                BGASequenceFrame {
                    time: 250,
                    id: Some(2),
                },
            ],
        }]),
        bga_events: Vec::new(),
        layer_events: Vec::new(),
        poor_events: vec![bmsrs_bmson_model::BGAEvent { y: 0, id: 7 }],
    };

    let chart = BmsonProcessor::process::<Beat>(&bmson)?.value;

    // Two file resources plus one clip resource.
    assert_eq!(chart.chart_info().bga_resources.len(), 3);
    let BgaResourceContent::Sequence { frames } = &chart.chart_info().bga_resources[2] else {
        return Err("clip resource expected".into());
    };
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].resource, Some(bmsrs_chart::BgaResourceId::new(0)));
    assert_eq!(frames[1].resource, Some(bmsrs_chart::BgaResourceId::new(1)));
    assert_eq!(frames[0].duration, Duration::from_millis(250));

    let poor: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Bga {
                layer: BgaLayer::Poor,
                resource_id,
            } => Some(*resource_id),
            _ => None,
        })
        .collect();
    assert_eq!(poor, vec![Some(bmsrs_chart::BgaResourceId::new(2))]);
    Ok(())
}
