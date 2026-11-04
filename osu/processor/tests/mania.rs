//! osu!mania conversion tests.

#![expect(clippy::unwrap_used, reason = "test code")]

use std::num::NonZeroU8;

use bmsrs_chart::{Chart, Event, EventKind, Lane, NoteKind};
use bmsrs_osu_processor::{
    OsuChartMeta, OsuCustomEvent, OsuHitSampleName, OsuNoteExt, OsuProcessor, OsuSampleBank,
};

/// The synthetic 4K chart fixture.
const MANIA_4K: &str = include_str!("fixtures/mania_4k.osu");

/// Parse and convert the synthetic 4K chart.
fn chart() -> Chart<OsuNoteExt, OsuCustomEvent> {
    OsuProcessor::from_str(MANIA_4K).unwrap()
}

/// Return the chart-level extension metadata.
fn chart_meta(chart: &Chart<OsuNoteExt, OsuCustomEvent>) -> &OsuChartMeta {
    chart
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::ChartMeta(meta)) => Some(meta),
            _ => None,
        })
        .unwrap()
}

/// Return all note events in event order.
fn note_events(
    chart: &Chart<OsuNoteExt, OsuCustomEvent>,
) -> Vec<&Event<OsuNoteExt, OsuCustomEvent>> {
    chart
        .data()
        .events()
        .iter()
        .filter(|event| matches!(&event.kind, EventKind::Note(_)))
        .collect()
}

/// Build a mania key lane.
const fn key(value: u8) -> Lane {
    match NonZeroU8::new(value) {
        Some(nonzero) => Lane::Key(nonzero),
        None => panic!("key must be non-zero"),
    }
}

#[test]
fn song_info_uses_romanised_text() {
    let converted = chart();
    assert_eq!(converted.song().title, "Test Song", "romanised title wins");
}

#[test]
fn unicode_metadata_is_preserved() {
    let converted = chart();
    assert_eq!(
        chart_meta(&converted).title_unicode,
        "テスト",
        "unicode title must be preserved"
    );
}

#[test]
fn chart_info_uses_version() {
    let converted = chart();
    assert_eq!(
        converted.chart_info().chart_name,
        "4K",
        "Version field becomes the chart name"
    );
}

#[test]
fn background_is_mapped() {
    let converted = chart();
    assert_eq!(
        converted.chart_info().back_image.as_deref(),
        Some("bg.jpg"),
        "background event must map to back_image"
    );
}

#[test]
fn audio_file_is_mapped() {
    let converted = chart();
    assert_eq!(
        converted
            .data()
            .audio_assets()
            .first()
            .map(|asset| asset.path.to_string_lossy().into_owned()),
        Some("audio.ogg".to_owned()),
        "AudioFilename must become the audio asset path"
    );
}

#[test]
fn creator_is_mapped() {
    let converted = chart();
    assert_eq!(chart_meta(&converted).creator, "Test Mapper", "creator");
}

#[test]
fn source_and_tags_are_mapped() {
    let converted = chart();
    let meta = chart_meta(&converted);
    assert_eq!(
        (meta.source.as_str(), meta.tags.as_str()),
        ("Test", "test mania"),
        "source and tags"
    );
}

#[test]
fn beatmap_ids_are_mapped() {
    let converted = chart();
    let meta = chart_meta(&converted);
    assert_eq!(
        (meta.beatmap_id, meta.beatmap_set_id),
        (1, 2),
        "beatmap ids"
    );
}

#[test]
fn audio_lead_in_is_mapped() {
    let converted = chart();
    assert_eq!(chart_meta(&converted).audio_lead_in_ms, 250, "AudioLeadIn");
}

#[test]
fn preview_time_is_mapped() {
    let converted = chart();
    assert_eq!(chart_meta(&converted).preview_time_ms, 1000, "PreviewTime");
}

#[test]
fn key_count_is_mapped() {
    let converted = chart();
    assert_eq!(chart_meta(&converted).key_count, 4, "CircleSize key count");
}

#[test]
fn difficulty_is_scaled() {
    let converted = chart();
    assert_eq!(
        chart_meta(&converted).hp_drain_rate_x10,
        70,
        "HPDrainRate scaled by 10"
    );
}

#[test]
fn slider_multiplier_is_scaled() {
    let converted = chart();
    assert_eq!(
        chart_meta(&converted).slider_multiplier_x100,
        140,
        "SliderMultiplier scaled by 100"
    );
}

#[test]
fn stack_leniency_is_scaled() {
    let converted = chart();
    assert_eq!(
        chart_meta(&converted).stack_leniency_x1000,
        700,
        "StackLeniency scaled by 1000"
    );
}

#[test]
fn notes_have_expected_ticks() {
    let converted = chart();
    let ticks: Vec<u64> = note_events(&converted)
        .iter()
        .map(|event| event.tick())
        .collect();
    assert_eq!(ticks, [2000, 3000, 6000, 8000], "note ticks");
}

#[test]
fn notes_have_expected_columns() {
    let converted = chart();
    let lanes: Vec<Lane> = note_events(&converted)
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::Note(note) => Some(note.lane),
            _ => None,
        })
        .collect();
    assert_eq!(lanes, [key(1), key(2), key(3), key(4)], "note columns");
}

#[test]
fn hold_duration_is_converted() {
    let converted = chart();
    let duration = note_events(&converted)
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Note(note) => match note.kind {
                NoteKind::Long { duration } => Some(duration),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    assert_eq!(duration, 1000, "hold duration in ticks");
}

#[test]
fn hit_samples_include_whistle() {
    let converted = chart();
    let samples = note_events(&converted)
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Note(note) if event.tick() == 6000 => Some(&note.ext.hit_samples),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        samples
            .iter()
            .map(|sample| &sample.name)
            .collect::<Vec<_>>(),
        [&OsuHitSampleName::Normal, &OsuHitSampleName::Whistle,],
        "whistle sample layers over normal"
    );
}

#[test]
fn hit_sample_bank_is_mapped() {
    let converted = chart();
    let bank = note_events(&converted)
        .first()
        .and_then(|event| match &event.kind {
            EventKind::Note(note) => note.ext.hit_samples.first(),
            _ => None,
        })
        .map(|sample| sample.bank);
    assert_eq!(bank, Some(OsuSampleBank::Soft), "sample bank");
}

#[test]
fn hit_sample_layering_is_mapped() {
    let converted = chart();
    let layered = note_events(&converted)
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Note(note) if event.tick() == 6000 => {
                note.ext.hit_samples.first().map(|sample| sample.is_layered)
            }
            _ => None,
        });
    assert_eq!(layered, Some(true), "layered sample flag");
}

#[test]
fn bpm_change_is_mapped() {
    let converted = chart();
    let bpm_change = converted
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Bpm { bpm } => Some((event.tick(), *bpm)),
            _ => None,
        });
    assert_eq!(
        bpm_change,
        Some((4000, 240.0)),
        "BPM change at tick 4000 to 240"
    );
}

#[test]
fn scroll_event_is_mapped() {
    let converted = chart();
    let rate = converted
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Scroll { rate } if event.tick() == 2000 => Some(*rate),
            _ => None,
        });
    assert_eq!(rate, Some(2.0), "scroll rate at tick 2000");
}

#[test]
fn kiai_event_is_mapped() {
    let converted = chart();
    let kiai = converted
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Kiai(enabled)) if event.tick() == 2000 => {
                Some(*enabled)
            }
            _ => None,
        });
    assert_eq!(kiai, Some(true), "kiai at tick 2000");
}

#[test]
fn break_event_is_mapped() {
    let converted = chart();
    let end_tick = converted
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Break { end_tick }) => Some(*end_tick),
            _ => None,
        });
    assert_eq!(end_tick, Some(4000), "break end tick");
}

#[test]
fn timing_event_is_mapped() {
    let converted = chart();
    let signature = converted
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Timing { time_signature, .. }) => {
                Some(*time_signature)
            }
            _ => None,
        });
    assert_eq!(signature, Some(4), "time signature numerator");
}

#[test]
fn timing_event_omits_first_bar_line() {
    let converted = chart();
    let omit = converted
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::Timing {
                omit_first_bar_line,
                ..
            }) if event.tick() == 4000 => Some(*omit_first_bar_line),
            _ => None,
        });
    assert_eq!(omit, Some(true), "omit first bar line at tick 4000");
}

#[test]
fn from_bytes_matches_from_str() {
    let from_bytes = OsuProcessor::from_bytes(MANIA_4K.as_bytes()).unwrap();
    let from_text = chart();
    assert_eq!(
        from_bytes.data().events().len(),
        from_text.data().events().len(),
        "from_bytes and from_str produce the same event count"
    );
}

#[test]
fn from_path_matches_from_str() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mania_4k.osu");
    let from_path = OsuProcessor::from_path(path).unwrap();
    let from_text = chart();
    assert_eq!(
        from_path.data().events().len(),
        from_text.data().events().len(),
        "from_path and from_str produce the same event count"
    );
}
