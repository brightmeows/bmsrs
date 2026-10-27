//! `Player` 集成测试。

#![expect(clippy::unwrap_used, reason = "test code")]

mod helper;

use std::time::Duration;

use bmsrs_player::Player;

#[test]
fn player_initializes() {
    let chart = helper::chart();
    assert!(Player::new(chart).is_ok());
}

#[test]
fn player_advances_to_expected_tick() {
    let chart = helper::chart();
    let mut player = Player::new(chart).unwrap();
    player.advance(Duration::from_millis(1500));
    assert_eq!(player.current_tick(), 3000);
}

#[test]
fn player_duration_is_expected() {
    let chart = helper::chart();
    let player = Player::new(chart).unwrap();
    assert_eq!(player.duration(), Duration::from_secs(3));
}

#[test]
fn player_notes_for_judgement_include_holds() {
    let chart = helper::chart();
    let player = Player::new(chart).unwrap();
    assert_eq!(player.notes_for_judgement(..).count(), 4);
}

#[test]
fn player_scroll_rate_uses_scroll_events() {
    let chart = helper::chart();
    let player = Player::new(chart).unwrap();
    assert_eq!(player.scroll_rate_at(2000), 2.0);
}

#[test]
fn player_notes_in_range_are_limited() {
    let chart = helper::chart();
    let player = Player::new(chart).unwrap();
    assert_eq!(player.notes_in_range(0..=3000).count(), 2);
}

#[test]
fn player_audio_assets_are_exposed() {
    let chart = helper::chart();
    let player = Player::new(chart).unwrap();
    assert_eq!(player.audio_assets().len(), 1);
}

#[test]
fn player_chart_keeps_meta() {
    let chart = helper::chart();
    let player = Player::new(chart).unwrap();
    assert_eq!(helper::chart_meta(player.chart()).creator, "Test Mapper");
}
