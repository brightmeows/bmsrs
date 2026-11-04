//! `Player` integration tests.

#![expect(clippy::unwrap_used, reason = "test code")]

use std::time::Duration;

use bmsrs_chart::{Chart, EventKind};
use bmsrs_osu_processor::{OsuChartMeta, OsuCustomEvent, OsuNoteExt, OsuProcessor};
use bmsrs_player::Player;

/// The synthetic 4K chart fixture.
const MANIA_4K: &str = include_str!("fixtures/mania_4k.osu");

/// Parse and convert the synthetic 4K chart.
fn chart() -> Chart<OsuNoteExt, OsuCustomEvent> {
    OsuProcessor::from_str(MANIA_4K).unwrap()
}

/// Return the chart-level extension metadata.
fn chart_meta(converted: &Chart<OsuNoteExt, OsuCustomEvent>) -> &OsuChartMeta {
    converted
        .data()
        .events()
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::ChartMeta(meta)) => Some(meta),
            _ => None,
        })
        .unwrap()
}

#[test]
fn player_starts_at_tick_zero() {
    let player = Player::new(chart());
    assert_eq!(player.current_tick(), 0, "a fresh player starts at tick 0");
}

#[test]
fn player_advances_to_expected_tick() {
    let mut player = Player::new(chart());
    player.advance(Duration::from_millis(1500));
    assert_eq!(
        player.current_tick(),
        3000,
        "1500 ms at 120 BPM is 3000 ticks"
    );
}

#[test]
fn player_duration_is_expected() {
    let player = Player::new(chart());
    assert_eq!(player.duration(), Duration::from_secs(3), "render duration");
}

#[test]
fn player_note_queries_include_holds() {
    let player = Player::new(chart());
    assert_eq!(
        player.notes_in_range(..).count(),
        4,
        "three circles plus one hold"
    );
    assert_eq!(
        player.long_notes().len(),
        1,
        "the hold pairs into one long note"
    );
}

#[test]
fn player_scroll_rate_uses_scroll_events() {
    let player = Player::new(chart());
    assert_eq!(player.scroll_rate_at(2000), 2.0, "scroll rate at tick 2000");
}

#[test]
fn player_notes_in_range_are_limited() {
    let player = Player::new(chart());
    assert_eq!(
        player.notes_in_range(0..=3000).count(),
        2,
        "only ticks 2000 and 3000 fall into the range"
    );
}

#[test]
fn player_audio_assets_are_exposed() {
    let player = Player::new(chart());
    assert_eq!(player.audio_assets().len(), 1, "one audio asset");
}

#[test]
fn player_chart_keeps_meta() {
    let player = Player::new(chart());
    assert_eq!(
        chart_meta(player.chart()).creator,
        "Test Mapper",
        "chart metadata survives into the player"
    );
}
