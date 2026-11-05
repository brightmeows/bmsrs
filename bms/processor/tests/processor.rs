#![expect(missing_docs, reason = "integration test")]

use std::num::NonZeroU8;
use std::path::Path;
use std::time::Duration;

use bmsrs_bms_model::{
    BgaEvent, BgmEvent, Bms, BmsBgaLayer, BpmChange, BpmValue, KeyType, LongNoteEvent, MineEvent,
    NonEventData, NoteEvent, Position, Rank, StpEvent,
};
use bmsrs_bms_model::{BmsChannel, BpmIndex, LnObjIndex};
use bmsrs_bms_processor::custom_event::BmsCustomEvent;
use bmsrs_bms_processor::layout::{Bme, BmsLayout as _, ChannelPos, DscOctFp, Nanasi, Pms, PmsBme};
use bmsrs_bms_processor::{BmsProcessor, ProcessError, ProcessWarningKind};
use bmsrs_chart::mode::{Lane, NoteSide};
use bmsrs_chart::{BgaLayer, CustomEvent, EventKind, Note, NoteExt, NoteKind, TimingTrack};

#[expect(clippy::expect_used, reason = "test helper panics on invalid timing")]
fn timing<T: NoteExt, C: CustomEvent>(chart: &bmsrs_chart::Chart<T, C>) -> TimingTrack {
    TimingTrack::from_events(
        chart.data().init_bpm(),
        chart.data().resolution(),
        chart.data().events(),
    )
    .expect("processed chart timing should be valid")
}

/// Shorthand for constructing a valid [`ChannelPos`] in tests.
fn ch(player: u8, lane: u8) -> ChannelPos {
    ChannelPos::new(player, lane).unwrap_or_else(|| panic!("invalid BMS channel"))
}

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

const fn sc(n: u8) -> Lane {
    Lane::Scratch(nz(n))
}

const PEDAL: Lane = Lane::FootPedal;

#[test]
fn bme_maps_key7_channel_19() {
    // Regression: the old Beat7k match `(1, 1..=8)` silently dropped channel 19
    // (decoded lane 9, which is KEY7). Bme must map it to Key(7).
    assert_eq!(Bme::map_channel(ch(1, 9)), Some((NoteSide::P1, key(7))));
}

#[test]
fn bme_maps_both_player_sides() {
    assert_eq!(Bme::map_channel(ch(2, 6)), Some((NoteSide::P2, sc(1))));
    assert_eq!(
        Bme::map_channel(ch(2, 6)),
        Some((NoteSide::P2, Lane::Scratch(NonZeroU8::new(1).unwrap())))
    );
    assert_eq!(Bme::map_channel(ch(2, 9)), Some((NoteSide::P2, key(7))));
}

#[test]
fn pms_maps_cross_side_channels_to_single_player() {
    // PMS KEY1-5 on 1P channels 11-15, mapped to Player1.
    assert_eq!(Pms::map_channel(ch(1, 1)), Some((NoteSide::P1, key(1))));
    assert_eq!(Pms::map_channel(ch(1, 5)), Some((NoteSide::P1, key(5))));
    // PMS KEY6-9 on 2P channels 22-25 still map to Player1 (single-player mode).
    assert_eq!(Pms::map_channel(ch(2, 2)), Some((NoteSide::P1, key(6))));
    assert_eq!(Pms::map_channel(ch(2, 5)), Some((NoteSide::P1, key(9))));
    // Channel 21 (2P lane 1) is unused by PMS.
    assert_eq!(Pms::map_channel(ch(2, 1)), None);
}

#[test]
fn nanasi_maps_foot_pedal_channel_17() {
    assert_eq!(Nanasi::map_channel(ch(1, 7)), Some((NoteSide::P1, PEDAL)));
    assert_eq!(Nanasi::map_channel(ch(2, 7)), Some((NoteSide::P2, PEDAL)));
}

#[test]
fn process_basic_note() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                side,
                lane,
                kind,
                ext: (),
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
    assert_eq!(chart.data().audio_assets().len(), 1);
}

#[test]
fn process_key_count_uses_used_lanes() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    for lane in [1, 2, 3, 4, 5, 8, 9] {
        bms.messages.note_events.push(NoteEvent {
            position: Position::new(0, 0, 8),
            player: 1,
            lane,
            key_type: KeyType::Visible,
            wav_id: "01".parse().unwrap(),
        });
    }

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    assert_eq!(chart.chart_info().key_count, Some(7));
}

#[test]
fn process_rejects_zero_bpm() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(0.0);

    let result = BmsProcessor::process::<Bme>(&bms);

    assert!(matches!(result, Err(ProcessError::ZeroBpm)));
}

#[test]
fn undefined_bmp_reference_clears_layer_with_warning() {
    // Referencing an undefined BMP index clears the layer instead of showing
    // a resource; the reference also warns.
    let mut bms = Bms::default();
    bms.visual
        .bmp_files
        .insert("01".parse().unwrap(), "a.bmp".to_owned());
    bms.messages.bga_events.push(BgaEvent {
        position: Position::new(1, 0, 1),
        layer: BmsBgaLayer::Base,
        bmp_id: "FF".parse().unwrap(),
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    let bga: Vec<_> = processed
        .value
        .chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Bga { layer, resource_id } = &e.kind {
                Some((*layer, resource_id.as_ref().map(|id| id.get()), e.tick()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bga, vec![(BgaLayer::Base, None, 960)]);
    assert!(processed.warnings.iter().any(|warning| {
        matches!(
            warning.kind,
            // "FF" decodes through the default base36, not hexadecimal.
            ProcessWarningKind::MissingBmpDefinition {
                key: 555,
                tick: 960
            }
        )
    }));
}

#[test]
fn text00_definition_becomes_miss_text_without_channel_event() {
    let mut bms = Bms::default();
    bms.metadata
        .text_defs
        .insert("00".parse().unwrap(), "miss text".to_owned());

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert_eq!(
        processed.value.chart.chart_info().miss_text.as_deref(),
        Some("miss text")
    );
    assert!(
        processed
            .value
            .chart
            .data()
            .events()
            .iter()
            .all(|e| !matches!(
                &e.kind,
                EventKind::Custom(BmsCustomEvent::TextDisplay { .. })
            )),
        "miss text must not become a channel event"
    );
}

#[test]
fn undefined_text_reference_warns_and_is_skipped() {
    let mut bms = Bms::default();
    bms.messages.non_event_data.push(NonEventData {
        measure: 1,
        channel: BmsChannel::Text,
        values: vec!["02".parse().unwrap()],
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert!(
        processed.warnings.iter().any(|warning| {
            matches!(
                warning.kind,
                ProcessWarningKind::MissingTextDefinition { key: 2, tick: 960 }
            )
        }),
        "undefined TEXT reference should warn"
    );
    assert!(
        processed
            .value
            .chart
            .data()
            .events()
            .iter()
            .all(|e| !matches!(
                &e.kind,
                EventKind::Custom(BmsCustomEvent::TextDisplay { .. })
            )),
        "skipped text events must not appear"
    );
}

#[test]
fn undefined_swbga_reference_warns_and_is_skipped() {
    let mut bms = Bms::default();
    bms.messages.non_event_data.push(NonEventData {
        measure: 1,
        channel: BmsChannel::BgaKeyBound,
        values: vec!["02".parse().unwrap()],
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert!(
        processed.warnings.iter().any(|warning| {
            matches!(
                warning.kind,
                ProcessWarningKind::MissingSwBgaDefinition { key: 2, tick: 960 }
            )
        }),
        "undefined SWBGA reference should warn"
    );
    assert!(
        processed
            .value
            .chart
            .data()
            .events()
            .iter()
            .all(|e| !matches!(
                &e.kind,
                EventKind::Custom(BmsCustomEvent::BgaKeyBound { .. })
            )),
        "skipped key-bound events must not appear"
    );
}

#[test]
fn undefined_exrank_reference_warns_and_is_skipped() {
    let mut bms = Bms::default();
    bms.messages.non_event_data.push(NonEventData {
        measure: 1,
        channel: BmsChannel::Judge,
        values: vec!["02".parse().unwrap()],
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert!(
        processed.warnings.iter().any(|warning| {
            matches!(
                warning.kind,
                ProcessWarningKind::MissingExRankDefinition { key: 2, tick: 960 }
            )
        }),
        "undefined EXRANK reference should warn"
    );
    assert!(
        processed
            .value
            .chart
            .data()
            .events()
            .iter()
            .all(|e| !matches!(&e.kind, EventKind::JudgeRank { .. })),
        "skipped judge events must not appear"
    );
}

#[test]
fn gameplay_headers_enter_chart_info() {
    let mut bms = Bms::default();
    bms.gameplay.def_ex_rank = Some(150.0);
    bms.gameplay.total = Some(200.0);
    bms.gameplay.vol_wav = Some(120.0);

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;
    let info = chart.chart_info();
    assert_eq!(
        info.judge_rank,
        Some(bmsrs_chart::JudgeRankSpec {
            value: 150.0,
            kind: bmsrs_chart::JudgeRankKind::DefExRank,
        })
    );
    assert_eq!(info.total, Some(200.0));
    assert_eq!(info.volwav_percent, Some(120));
}

#[test]
fn bga_resource_kind_infers_video_extension() {
    let mut bms = Bms::default();
    bms.visual
        .bmp_files
        .insert("01".parse().unwrap(), "clip.mp4".to_owned());
    bms.visual
        .bmp_files
        .insert("02".parse().unwrap(), "img.png".to_owned());

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;
    let resources = chart.chart_info().bga_resources.as_slice();
    assert_eq!(resources.len(), 2);
    assert_eq!(
        resources[0].kind(),
        bmsrs_chart::BgaResourceKind::Video,
        "video extension maps to Video"
    );
    assert_eq!(
        resources[1].kind(),
        bmsrs_chart::BgaResourceKind::Static,
        "image extension maps to Static"
    );
}

#[test]
fn rank_header_maps_to_bms_rank_kind() {
    let mut bms = Bms::default();
    bms.gameplay.rank = Some(Rank::Hard);

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;
    assert_eq!(
        chart.chart_info().judge_rank,
        Some(bmsrs_chart::JudgeRankSpec {
            value: 1.0,
            kind: bmsrs_chart::JudgeRankKind::BmsRank,
        })
    );
}

#[test]
fn stp_entries_at_same_position_accumulate() {
    // The specification example: two `#STP` lines at `001.500` stop scrolling
    // for 4500 ms in total, not last-wins.
    let mut bms = Bms::default();
    bms.messages.stp_events.push(StpEvent {
        position: Position::new(1, 500, 1000),
        duration_ms: 500.0,
    });
    bms.messages.stp_events.push(StpEvent {
        position: Position::new(1, 500, 1000),
        duration_ms: 4000.0,
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let stops: Vec<_> = chart
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
    // Measure 1 starts at tick 960; position 500/1000 adds 480 ticks.
    // Default BPM 130: 4500 ms = 9.75 beats = 2340 ticks at resolution 240.
    assert_eq!(
        stops.len(),
        1,
        "same-position entries must merge into a single stop"
    );
    assert_eq!(stops[0], (1440, Duration::from_millis(4500)));
}

#[test]
fn negative_stp_entry_warns_and_is_skipped() {
    let mut bms = Bms::default();
    bms.messages.stp_events.push(StpEvent {
        position: Position::new(1, 0, 1),
        duration_ms: -500.0,
    });
    bms.messages.stp_events.push(StpEvent {
        position: Position::new(2, 0, 1),
        duration_ms: 1000.0,
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    let stops: Vec<_> = processed
        .value
        .chart
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
    // Only the positive entry survives; the negative one is dropped.
    assert_eq!(stops.len(), 1);
    // 1000 ms at default BPM 130 = 520 ticks, at measure 2 (tick 1920).
    assert_eq!(stops[0], (1920, Duration::from_millis(1000)));
    assert!(processed.warnings.iter().any(|warning| {
        matches!(
            warning.kind,
            ProcessWarningKind::StopDurationClipped { tick: 960, .. }
        )
    }));
}

#[test]
fn process_applies_path_wav_to_audio_asset() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio.path_wav = Some("snd".to_owned());
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    assert_eq!(
        chart.data().audio_assets()[0].path.as_ref(),
        Path::new("snd/kick.wav")
    );
}

#[test]
fn process_preserves_bms_audio_sidecar() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio.cdda = Some("5".to_owned());
    bms.audio.path_wav = Some("snd".to_owned());

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap().value;

    assert_eq!(processed.audio.cdda.as_deref(), Some("5"));
    assert_eq!(processed.audio.path_wav.as_deref(), Some("snd"));
}

#[test]
fn process_preserves_bms_gameplay_sidecar() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.option = Some("774:HI-SPEED_x1.00".to_owned());

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap().value;

    assert_eq!(
        processed.gameplay.option.as_deref(),
        Some("774:HI-SPEED_x1.00")
    );
}

#[test]
fn process_default_detects_oct_fp_layout() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.gameplay.oct_fp = Some(true);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 2,
        lane: 2,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process_default(&bms).unwrap().value.chart;
    let lane = chart.data().events().iter().find_map(|event| {
        if let EventKind::Note(Note { lane, .. }) = &event.kind {
            Some(*lane)
        } else {
            None
        }
    });

    assert_eq!(lane, Some(key(1)));
}

#[test]
fn process_key7_note_lands_on_key_seven() {
    // End-to-end regression for the KEY7 channel 19 drop bug.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "k7.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 9, // Channel 19 gives KEY7.
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note { lane, ext: (), .. }) = &e.kind {
                Some(*lane)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes[0], key(7));
}

#[test]
fn process_zero_bpm_returns_error() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(0.0);
    let result = BmsProcessor::process::<Bme>(&bms);
    assert!(result.is_err());
}

#[test]
fn process_negative_bpm_is_rejected() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(-120.0);
    let result = BmsProcessor::process::<Bme>(&bms);
    assert!(result.is_err());
}

#[test]
fn process_nan_bpm_change_never_enters_event_stream() {
    // NaN can enter Bms through f64 parsing; filtered by the same predicate as
    // the timing track, it must not enter the unified event stream (Event's Eq
    // contract requires f64 fields to contain no NaN).
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 4),
        value: BpmValue::Absolute(f64::NAN),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;
    let has_bpm_event = chart
        .data()
        .events()
        .iter()
        .any(|e| matches!(e.kind, EventKind::Bpm { .. }));
    assert!(!has_bpm_event);
}

#[test]
fn process_nan_bpm_change_produces_warning() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 4),
        value: BpmValue::Absolute(f64::NAN),
    });

    let warnings = BmsProcessor::process::<Bme>(&bms).unwrap().warnings;
    // NaN cannot be compared with assert_eq; use matches! to check the variant
    // and value.
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        &warnings[0].kind,
        ProcessWarningKind::InvalidBpmChangeIgnored { bpm, .. } if bpm.is_nan()
    ));
}

#[test]
fn process_bgm_events_mapped() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "bgm.wav".to_owned());
    bms.messages.bgm_events.push(BgmEvent {
        position: Position::new(0, 0, 8),
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let bgm: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Bgm { .. } = &e.kind {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bgm.len(), 1);
    assert_eq!(bgm[0], 0);
}

#[test]
fn process_metadata_from_headers() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.metadata.title = Some("Test Song".to_owned());
    bms.metadata.artist = Some("Test Artist".to_owned());
    bms.metadata.genre = Some("Test Genre".to_owned());

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    assert_eq!(chart.song().title, "Test Song");
    assert_eq!(chart.song().artist, "Test Artist");
    assert_eq!(chart.song().genre, "Test Genre");
}

#[test]
fn process_default_uses_bme_and_maps_both_sides() {
    // process_default uses Bme unconditionally (the #PLAYER header does not
    // affect the mapping). 2P notes report NoteSide::P2.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "a.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 2,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process_default(&bms).unwrap().value.chart;

    let positions: Vec<(NoteSide, Lane)> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note { side, lane, .. }) = &e.kind {
                Some((*side, *lane))
            } else {
                None
            }
        })
        .collect();
    assert!(positions.contains(&(NoteSide::P1, key(1))));
    assert!(positions.contains(&(NoteSide::P2, key(1))));
}

#[test]
fn process_lnobj_produces_long_note() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("02".parse().unwrap(), "b.wav".to_owned());
    let ln_obj: LnObjIndex = "02".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "02".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let lns: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                kind: NoteKind::Long { .. },
                ..
            }) = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(lns.len(), 1);
    assert_eq!(lns[0], 0);
}

/// In LNOBJ mode, ch51-69 long-note channel events are mutually exclusive with
/// LNOBJ (memo/10 leaves this undefined), but they must not be discarded
/// silently; they are kept as ordinary visible notes.
#[test]
fn lnobj_preserves_long_note_channel_events_as_normal() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "a.wav".to_owned());
    bms.audio
        .wav_files
        .insert("02".parse().unwrap(), "b.wav".to_owned());
    bms.gameplay.ln_obj = Some("02".parse::<LnObjIndex>().unwrap());
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 4),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let normals: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note {
                kind: NoteKind::Normal,
                ..
            }) = &e.kind
            {
                Some(e.tick())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        normals,
        vec![0u64],
        "ch51-69 events should be kept as normal notes"
    );
}

#[test]
fn process_bpm_change_reference_resolved() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    let bpm_id: BpmIndex = "01".parse().unwrap();
    bms.timing.bpm_defs.insert(bpm_id, 200.0);
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 8),
        value: BpmValue::Reference(bpm_id),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    assert_eq!(timing(&chart).bpm_changes().len(), 1);
    assert!((timing(&chart).bpm_changes()[0].bpm - 200.0).abs() < 1e-9);
}

#[test]
fn process_bpm_changes_dedup_preserves_mid_change() {
    // Regression: deduplication used to run before sorting by tick. Equal-value
    // changes at 1/4 and 3/4 in the same channel were adjacent in the vector and
    // merged, so BPM could not return to 120 after the 1/2 reference change
    // between them.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(60.0);
    let bpm_id: BpmIndex = "01".parse().unwrap();
    bms.timing.bpm_defs.insert(bpm_id, 240.0);
    // Simulate parser output order: within the measure, ch03 (1/4, 3/4) comes
    // first, then ch08 (1/2).
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(0, 1, 4),
        value: BpmValue::Absolute(120.0),
    });
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(0, 3, 4),
        value: BpmValue::Absolute(120.0),
    });
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(0, 1, 2),
        value: BpmValue::Reference(bpm_id),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    // At 3/4 of measure 0 (tick 720), BPM should be back to 120.
    assert!((timing(&chart).bpm_at_tick(720) - 120.0).abs() < 1e-9);
}

#[test]
fn process_bar_lines_generated() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let bar_lines: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| matches!(&e.kind, EventKind::Bar).then_some(e.tick()))
        .collect();
    assert_ne!(bar_lines, [] as [u64; 0]);
    assert_eq!(bar_lines[0], 0);
    assert_eq!(bar_lines[1], 960);
}

#[test]
fn process_invisible_note_mapped() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "se.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Invisible,
        wav_id: "01".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Note(Note { kind, ext: (), .. }) = &e.kind {
                Some(*kind)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0], NoteKind::Invisible);
}

#[test]
fn pms_bme_reinterprets_16_17_as_keys() {
    assert_eq!(PmsBme::map_channel(ch(1, 8)), Some((NoteSide::P1, key(6))));
    assert_eq!(PmsBme::map_channel(ch(1, 6)), Some((NoteSide::P1, key(8))));
    assert_eq!(PmsBme::map_channel(ch(2, 7)), Some((NoteSide::P2, key(9))));
}

#[test]
fn dsc_oct_fp_maps_dual_scratch_and_pedal() {
    // P1 Scratch.
    assert_eq!(DscOctFp::map_channel(ch(1, 6)), Some((NoteSide::P1, sc(1))));
    // P2 foot pedal.
    assert_eq!(DscOctFp::map_channel(ch(2, 1)), Some((NoteSide::P2, PEDAL)));
    // P2 second scratch.
    assert_eq!(DscOctFp::map_channel(ch(2, 6)), Some((NoteSide::P2, sc(2))));
}

#[test]
fn pms_bme_maps_second_player_side() {
    assert_eq!(PmsBme::map_channel(ch(2, 1)), Some((NoteSide::P2, key(1))));
    assert_eq!(PmsBme::map_channel(ch(2, 9)), Some((NoteSide::P2, key(7))));
}

#[test]
fn bme_maps_extended_channel_lane_10() {
    // Extended channels (pomu2 family `1A` and so on, lane 10) map to Key(10).
    assert_eq!(Bme::map_channel(ch(1, 10)), Some((NoteSide::P1, key(10))));
    assert_eq!(Bme::map_channel(ch(2, 35)), Some((NoteSide::P2, key(35))));
}

#[test]
fn all_layouts_map_extended_channel_lane_10() {
    // Every family maps extended channels (lanes 10-35) to Key(lane) on the
    // corresponding side.
    assert_eq!(
        Nanasi::map_channel(ch(1, 10)),
        Some((NoteSide::P1, key(10)))
    );
    assert_eq!(Pms::map_channel(ch(1, 10)), Some((NoteSide::P1, key(10))));
    assert_eq!(
        PmsBme::map_channel(ch(1, 10)),
        Some((NoteSide::P1, key(10)))
    );
    assert_eq!(
        DscOctFp::map_channel(ch(1, 10)),
        Some((NoteSide::P1, key(10)))
    );
    assert_eq!(
        DscOctFp::map_channel(ch(2, 10)),
        Some((NoteSide::P2, key(10)))
    );
}

#[test]
fn bms_channel_rejects_invalid_input() {
    assert_eq!(ChannelPos::new(3, 1), None);
    assert_eq!(ChannelPos::new(0, 1), None);
    assert_eq!(ChannelPos::new(1, 0), None);
    // Lane 10 is now supported (extended channels `1A`-`1Z` and so on).
    assert!(ChannelPos::new(1, 10).is_some());
    assert!(ChannelPos::new(1, 35).is_some());
    // Upper bound: lane 36 and above are still invalid.
    assert!(ChannelPos::new(1, 36).is_none());
}

#[test]
fn lnobj_end_marker_emits_no_bgm() {
    // LNOBJ end markers are silent: beatoraja (jbms-parser) stores them as a
    // no-sound note, bmz-player asserts the marker WAV is not used as an end
    // keysound, and LR2 does not play it either. The RDM-spec behavior of
    // playing the marker WAV as BGM is deliberately not implemented.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    // Register the start WAV and the LNOBJ WAV.
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.audio
        .wav_files
        .insert("FF".parse().unwrap(), "ln_end.wav".to_owned());
    let ln_obj: LnObjIndex = "FF".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);
    // Long-note start (a visible note with WAV 01).
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    // Long-note end marker (LNOBJ WAV FF at measure 1).
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "FF".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    // Collect BGM events from the chart.
    let bgm_ticks: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| {
            if let EventKind::Bgm { audio_index } = &e.kind {
                Some((e.tick(), audio_index.get()))
            } else {
                None
            }
        })
        .collect();

    // The end marker (tick 960) must not surface as a BGM event; the only
    // BGM events are those from channel 01, and this chart has none.
    assert_eq!(bgm_ticks, vec![]);
}

#[test]
fn orphan_lnobj_marker_emits_no_bgm() {
    // When an LNOBJ marker has no preceding note on the same lane, no pair is
    // formed, so no LNOBJ BGM should be emitted.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("FF".parse().unwrap(), "orphan.wav".to_owned());
    let ln_obj: LnObjIndex = "FF".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);
    // An LNOBJ marker with no predecessor on the same (player, lane).
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "FF".parse().unwrap(),
    });

    let chart = BmsProcessor::process::<Bme>(&bms).unwrap().value.chart;

    let bgm_count = chart
        .data()
        .events()
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Bgm { .. }))
        .count();
    assert_eq!(bgm_count, 0, "orphan LNOBJ marker should not emit BGM");
}

#[test]
fn duplicate_same_position_note_keeps_last_definition() {
    // beatoraja (jbms-parser) warns and keeps only the later definition when
    // two visible notes land on the same position.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "first.wav".to_owned());
    bms.audio
        .wav_files
        .insert("02".parse().unwrap(), "second.wav".to_owned());
    for wav in ["01", "02"] {
        bms.messages.note_events.push(NoteEvent {
            position: Position::new(1, 0, 8),
            player: 1,
            lane: 1,
            key_type: KeyType::Visible,
            wav_id: wav.parse().unwrap(),
        });
    }

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    let notes: Vec<_> = processed
        .value
        .chart
        .data()
        .events()
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Note(note) if note.kind == NoteKind::Normal => {
                Some(note.audio.primary().map(bmsrs_chart::AudioIndex::get))
            }
            _ => None,
        })
        .collect();
    // Only the later definition (WAV 02, audio index 1) survives.
    assert_eq!(notes, vec![Some(1)]);
    assert!(processed.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            ProcessWarningKind::DuplicateNotePosition {
                player: 1,
                lane: 1,
                tick: 960
            }
        )
    }));
}

#[test]
fn mine_on_occupied_note_position_is_dropped() {
    // A mine never overwrites an occupied slot (matching beatoraja).
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    bms.messages.mine_events.push(MineEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        raw_value: None,
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    let mines = count_mines(&processed.value.chart);
    assert_eq!(mines, 0);
    assert!(processed.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            ProcessWarningKind::MineNoteConflict {
                player: 1,
                lane: 1,
                tick: 960
            }
        )
    }));
}

#[test]
fn note_on_mine_position_replaces_mine() {
    // A note placed after a mine on the same position wins; the mine is
    // dropped with a warning (matching beatoraja's channel order).
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.mine_events.push(MineEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        raw_value: None,
    });
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert_eq!(count_mines(&processed.value.chart), 0);
    let has_note = processed
        .value
        .chart
        .data()
        .events()
        .iter()
        .any(|e| matches!(&e.kind, EventKind::Note(note) if note.kind == NoteKind::Normal));
    assert!(has_note);
    assert!(processed.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            ProcessWarningKind::MineNoteConflict { tick: 960, .. }
        )
    }));
}

#[test]
fn mine_inside_long_note_range_is_dropped() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    // Long note on lane 1 across measure 0 (ch51): start at 0/8, end at 8/8.
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 7, 8),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });
    // Mine in the middle of the long-note range.
    bms.messages.mine_events.push(MineEvent {
        position: Position::new(0, 3, 8),
        player: 1,
        lane: 1,
        raw_value: None,
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert_eq!(count_mines(&processed.value.chart), 0);
    assert!(processed.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            ProcessWarningKind::MineNoteConflict { tick: 360, .. }
        )
    }));
}

#[test]
fn mine_on_unterminated_long_note_start_is_dropped() {
    // jbms-parser keeps an unterminated long-note start on the slot until it
    // is removed at the end of the song, so a mine sharing its position is
    // dropped.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.messages.long_note_events.push(LongNoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        wav_id: "01".parse().unwrap(),
    });
    bms.messages.mine_events.push(MineEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        raw_value: None,
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    assert_eq!(count_mines(&processed.value.chart), 0);
    assert!(processed.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            ProcessWarningKind::MineNoteConflict { tick: 0, .. }
        )
    }));
}

#[test]
fn lnobj_end_position_note_is_dropped_silently() {
    // A visible note sharing the LNOBJ end marker's position falls inside the
    // long-note range [start, end] and is dropped by range suppression before
    // any slot conflict arises (jbms-parser resolves this row-order
    // dependently; bmsrs keeps the range behavior). The marker itself emits
    // nothing.
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.audio
        .wav_files
        .insert("01".parse().unwrap(), "kick.wav".to_owned());
    bms.audio
        .wav_files
        .insert("02".parse().unwrap(), "other.wav".to_owned());
    bms.audio
        .wav_files
        .insert("FF".parse().unwrap(), "ln_end.wav".to_owned());
    let ln_obj: LnObjIndex = "FF".parse().unwrap();
    bms.gameplay.ln_obj = Some(ln_obj);
    // Long-note start (WAV 01) and the LNOBJ end marker (WAV FF) on lane 1.
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(0, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "01".parse().unwrap(),
    });
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "FF".parse().unwrap(),
    });
    // Another visible note on the end marker's position.
    bms.messages.note_events.push(NoteEvent {
        position: Position::new(1, 0, 8),
        player: 1,
        lane: 1,
        key_type: KeyType::Visible,
        wav_id: "02".parse().unwrap(),
    });

    let processed = BmsProcessor::process::<Bme>(&bms).unwrap();
    let chart = processed.value.chart;
    // Only the long note (start tick 0) survives; neither the marker nor the
    // co-positioned note emits anything.
    let notes: Vec<_> = chart
        .data()
        .events()
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Note(note) => Some((e.tick(), note.kind)),
            _ => None,
        })
        .collect();
    assert_eq!(notes.len(), 1, "lane notes: {notes:?}");
    assert!(!processed.warnings.iter().any(|warning| matches!(
        &warning.kind,
        ProcessWarningKind::DuplicateNotePosition { .. }
    )));
}

/// Count mine notes in the chart event stream.
fn count_mines<T: NoteExt, C: CustomEvent>(chart: &bmsrs_chart::Chart<T, C>) -> usize {
    chart
        .data()
        .events()
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Note(note) if matches!(note.kind, NoteKind::Mine { .. })))
        .count()
}

#[test]
fn dedup_adjacent_same_bpm() {
    let mut bms = Bms::default();
    bms.timing.bpm = Some(120.0);
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(0, 0, 1),
        value: BpmValue::Absolute(120.0),
    });
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(0, 1, 2),
        value: BpmValue::Absolute(120.0),
    });
    bms.messages.bpm_changes.push(BpmChange {
        position: Position::new(1, 0, 1),
        value: BpmValue::Absolute(140.0),
    });

    let chart = BmsProcessor::process_default(&bms).unwrap().value.chart;
    let bpm_events: Vec<_> = timing(&chart).bpm_changes().to_vec();
    assert_eq!(bpm_events.len(), 2);
    assert!((bpm_events[0].bpm - 120.0).abs() < f64::EPSILON);
    assert!((bpm_events[1].bpm - 140.0).abs() < f64::EPSILON);
}
