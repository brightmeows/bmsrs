//! osu!mania 转换测试。

#![expect(clippy::unwrap_used, reason = "test code")]

mod helper;

use std::num::NonZeroU8;

use bmsrs_chart::{EventKind, Lane, NoteKind};
use osu_processor::{OsuCustomEvent, OsuHitSampleName, OsuProcessor, OsuSampleBank};

const fn key(value: u8) -> Lane {
    match NonZeroU8::new(value) {
        Some(nonzero) => Lane::Key(nonzero),
        None => panic!("key must be non-zero"),
    }
}

#[test]
fn song_info_uses_romanised_text() {
    let chart = helper::chart();
    assert_eq!(chart.song.title, "Test Song");
}

#[test]
fn unicode_metadata_is_preserved() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).title_unicode, "テスト");
}

#[test]
fn chart_info_uses_version() {
    let chart = helper::chart();
    assert_eq!(chart.chart.chart_name, "4K");
}

#[test]
fn background_is_mapped() {
    let chart = helper::chart();
    assert_eq!(chart.chart.back_image.as_deref(), Some("bg.jpg"));
}

#[test]
fn audio_file_is_mapped() {
    let chart = helper::chart();
    assert_eq!(
        chart
            .data
            .audio_assets
            .first()
            .map(|asset| asset.path.to_string_lossy().into_owned()),
        Some("audio.ogg".to_owned())
    );
}

#[test]
fn creator_is_mapped() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).creator, "Test Mapper");
}

#[test]
fn source_and_tags_are_mapped() {
    let chart = helper::chart();
    let meta = helper::chart_meta(&chart);
    assert_eq!(
        (meta.source.as_str(), meta.tags.as_str()),
        ("Test", "test mania")
    );
}

#[test]
fn beatmap_ids_are_mapped() {
    let chart = helper::chart();
    let meta = helper::chart_meta(&chart);
    assert_eq!((meta.beatmap_id, meta.beatmap_set_id), (1, 2));
}

#[test]
fn audio_lead_in_is_mapped() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).audio_lead_in_ms, 250);
}

#[test]
fn preview_time_is_mapped() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).preview_time_ms, 1000);
}

#[test]
fn key_count_is_mapped() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).key_count, 4);
}

#[test]
fn difficulty_is_scaled() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).hp_drain_rate_x10, 70);
}

#[test]
fn slider_multiplier_is_scaled() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).slider_multiplier_x100, 140);
}

#[test]
fn stack_leniency_is_scaled() {
    let chart = helper::chart();
    assert_eq!(helper::chart_meta(&chart).stack_leniency_x1000, 700);
}

#[test]
fn notes_have_expected_ticks() {
    let chart = helper::chart();
    let ticks: Vec<u64> = helper::note_events(&chart)
        .iter()
        .map(|event| event.tick())
        .collect();
    assert_eq!(ticks, [2000, 3000, 6000, 8000]);
}

#[test]
fn notes_have_expected_columns() {
    let chart = helper::chart();
    let lanes: Vec<Lane> = helper::note_events(&chart)
        .iter()
        .filter_map(|event| match event.kind {
            EventKind::Note { lane, .. } => Some(lane),
            _ => None,
        })
        .collect();
    assert_eq!(lanes, [key(1), key(2), key(3), key(4)]);
}

#[test]
fn hold_duration_is_converted() {
    let chart = helper::chart();
    let duration = helper::note_events(&chart)
        .iter()
        .find_map(|event| match event.kind {
            EventKind::Note {
                kind: NoteKind::Long { duration },
                ..
            } => Some(duration),
            _ => None,
        })
        .unwrap();
    assert_eq!(duration, 1000);
}

#[test]
fn hit_samples_include_whistle() {
    let chart = helper::chart();
    let samples = helper::note_events(&chart)
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Note { ext, .. } if event.tick() == 6000 => Some(&ext.hit_samples),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        samples
            .iter()
            .map(|sample| &sample.name)
            .collect::<Vec<_>>(),
        [&OsuHitSampleName::Normal, &OsuHitSampleName::Whistle,]
    );
}

#[test]
fn hit_sample_bank_is_mapped() {
    let chart = helper::chart();
    let bank = helper::note_events(&chart)
        .first()
        .and_then(|event| match &event.kind {
            EventKind::Note { ext, .. } => ext.hit_samples.first(),
            _ => None,
        })
        .map(|sample| sample.bank);
    assert_eq!(bank, Some(OsuSampleBank::Soft));
}

#[test]
fn hit_sample_layering_is_mapped() {
    let chart = helper::chart();
    let layered = helper::note_events(&chart)
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Note { ext, .. } if event.tick() == 6000 => {
                ext.hit_samples.first().map(|sample| sample.is_layered)
            }
            _ => None,
        });
    assert_eq!(layered, Some(true));
}

#[test]
fn bpm_change_is_mapped() {
    let chart = helper::chart();
    assert_eq!(
        chart
            .data
            .timing
            .bpm_changes()
            .first()
            .map(|change| (change.tick, change.bpm)),
        Some((4000, 240.0))
    );
}

#[test]
fn scroll_event_is_mapped() {
    let chart = helper::chart();
    let rate = chart.data.events.iter().find_map(|event| match event.kind {
        EventKind::Scroll { rate } if event.tick() == 2000 => Some(rate),
        _ => None,
    });
    assert_eq!(rate, Some(2.0));
}

#[test]
fn kiai_event_is_mapped() {
    let chart = helper::chart();
    let kiai = chart
        .data
        .events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Kiai(enabled)) if event.tick() == 2000 => {
                Some(*enabled)
            }
            _ => None,
        });
    assert_eq!(kiai, Some(true));
}

#[test]
fn break_event_is_mapped() {
    let chart = helper::chart();
    let end_tick = chart
        .data
        .events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Break { end_tick }) => Some(*end_tick),
            _ => None,
        });
    assert_eq!(end_tick, Some(4000));
}

#[test]
fn timing_event_is_mapped() {
    let chart = helper::chart();
    let signature = chart
        .data
        .events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Timing { time_signature, .. }) => {
                Some(*time_signature)
            }
            _ => None,
        });
    assert_eq!(signature, Some(4));
}

#[test]
fn timing_event_omits_first_bar_line() {
    let chart = helper::chart();
    let omit = chart
        .data
        .events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Timing {
                omit_first_bar_line,
                ..
            }) if event.tick() == 4000 => Some(*omit_first_bar_line),
            _ => None,
        });
    assert_eq!(omit, Some(true));
}

#[test]
fn chart_validates() {
    let chart = helper::chart();
    assert!(chart.validate().is_ok());
}

#[test]
fn from_bytes_matches_from_str() {
    let from_bytes = OsuProcessor::from_bytes(helper::MANIA_4K.as_bytes()).unwrap();
    let from_str = helper::chart();
    assert_eq!(from_bytes.data.events.len(), from_str.data.events.len());
}

#[test]
fn from_path_matches_from_str() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mania_4k.osu");
    let from_path = OsuProcessor::from_path(path).unwrap();
    let from_str = helper::chart();
    assert_eq!(from_path.data.events.len(), from_str.data.events.len());
}
