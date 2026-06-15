#![expect(missing_docs, reason = "integration test")]

mod helper;

use bmsrs_chart::{
    BarLine, Bga, BgmEvent, BpmChange, Chart, ChartMetadata, Note, NoteKind, ScrollChangeEvent,
    TimingTrack,
};
use bmsrs_player::Player;
use helper::{make_chart, note};
use std::time::Duration;

#[test]
fn new_player_starts_at_tick_zero() {
    let player = Player::new(make_chart(vec![]));
    assert_eq!(player.current_tick(), 0);
    assert_eq!(player.current_time(), Duration::ZERO);
}

#[test]
fn advance_one_second_updates_position() {
    let mut player = Player::new(make_chart(vec![]));
    player.advance(Duration::from_secs(1));
    assert_eq!(player.current_time(), Duration::from_secs(1));
}

#[test]
fn advance_multiple_increments_accumulate() {
    let mut player = Player::new(make_chart(vec![]));
    player.advance(Duration::from_millis(500));
    player.advance(Duration::from_millis(500));
    assert_eq!(player.current_time(), Duration::from_secs(1));
}

#[test]
fn seek_sets_absolute_position() {
    let mut player = Player::new(make_chart(vec![]));
    player.advance(Duration::from_secs(2));
    player.seek(Duration::from_millis(500));
    assert_eq!(player.current_time(), Duration::from_millis(500));
}

#[test]
fn reset_returns_to_zero() {
    let mut player = Player::new(make_chart(vec![]));
    player.advance(Duration::from_secs(2));
    player.reset();
    assert_eq!(player.current_tick(), 0);
}

#[test]
fn tick_to_duration_matches_timing_track() {
    let chart = make_chart(vec![]);
    let player = Player::new(chart);

    for tick in [0u64, 120, 240, 480, 960] {
        let expected = TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        }
        .tick_to_duration(tick, 240);
        let actual = player.tick_to_duration(tick);
        assert_eq!(actual, expected, "mismatch at tick {tick}");
    }
}

#[test]
fn notes_in_range_returns_subset() {
    let chart = make_chart(vec![
        note(0, 0, NoteKind::Normal),
        note(240, 1, NoteKind::Normal),
        note(480, 0, NoteKind::Normal),
        note(720, 1, NoteKind::Normal),
        note(960, 0, NoteKind::Normal),
    ]);
    let player = Player::new(chart);

    let range = player.notes_in_range(240, 720);
    assert_eq!(range.len(), 2);
    assert_eq!(range[0].tick, 240);
    assert_eq!(range[1].tick, 480);
}

#[test]
fn notes_in_lane_filters_by_lane() {
    let chart = make_chart(vec![
        note(0, 0, NoteKind::Normal),
        note(240, 1, NoteKind::Normal),
        note(480, 0, NoteKind::Normal),
    ]);
    let player = Player::new(chart);

    let lane0: Vec<&Note> = player.notes_in_lane(0, 0, 960).collect();
    assert_eq!(lane0.len(), 2);
    assert_eq!(lane0[0].tick, 0);
    assert_eq!(lane0[1].tick, 480);
}

#[test]
fn notes_for_judgement_excludes_invisible_and_mines() {
    let chart = make_chart(vec![
        note(0, 0, NoteKind::Normal),
        note(240, 0, NoteKind::Invisible),
        note(480, 0, NoteKind::Mine { damage: 1.0 }),
        note(720, 0, NoteKind::Long { duration: 240 }),
    ]);
    let player = Player::new(chart);

    let judge_notes: Vec<&Note> = player.notes_for_judgement(0, 960).collect();
    assert_eq!(judge_notes.len(), 2);
    assert_eq!(judge_notes[0].tick, 0);
    assert_eq!(judge_notes[1].tick, 720);
}

#[test]
fn bgm_in_range_returns_events() {
    let chart: Chart = Chart {
        metadata: ChartMetadata::default(),
        resolution: 240,
        lane_count: 8,
        timing: TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        },
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        notes: vec![],
        bgm: vec![
            BgmEvent { tick: 0, audio: 0 },
            BgmEvent {
                tick: 480,
                audio: 1,
            },
            BgmEvent {
                tick: 960,
                audio: 2,
            },
        ],
        audio_assets: vec![],
        bar_lines: vec![],
        scroll_events: vec![],
        bga: Bga::default(),
    };
    let player = Player::new(chart);

    let range = player.bgm_in_range(240, 960);
    assert_eq!(range.len(), 1);
    assert_eq!(range[0].tick, 480);
}

#[test]
fn scroll_rate_at_returns_latest_multiplier() {
    let chart: Chart = Chart {
        metadata: ChartMetadata::default(),
        resolution: 240,
        lane_count: 8,
        timing: TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        },
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        notes: vec![],
        bgm: vec![],
        audio_assets: vec![],
        bar_lines: vec![],
        scroll_events: vec![
            ScrollChangeEvent {
                tick: 240,
                rate: 2.0,
            },
            ScrollChangeEvent {
                tick: 720,
                rate: 0.5,
            },
        ],
        bga: Bga::default(),
    };
    let player = Player::new(chart);

    assert!((player.scroll_rate_at(0) - 1.0).abs() < 1e-9);
    assert!((player.scroll_rate_at(240) - 2.0).abs() < 1e-9);
    assert!((player.scroll_rate_at(480) - 2.0).abs() < 1e-9);
    assert!((player.scroll_rate_at(720) - 0.5).abs() < 1e-9);
}

#[test]
fn bar_lines_in_range_returns_subset() {
    let chart: Chart = Chart {
        metadata: ChartMetadata::default(),
        resolution: 240,
        lane_count: 8,
        timing: TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![],
            stops: vec![],
        },
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        notes: vec![],
        bgm: vec![],
        audio_assets: vec![],
        bar_lines: vec![
            BarLine { tick: 0 },
            BarLine { tick: 960 },
            BarLine { tick: 1920 },
        ],
        scroll_events: vec![],
        bga: Bga::default(),
    };
    let player = Player::new(chart);

    let range = player.bar_lines_in_range(500, 2000);
    assert_eq!(range.len(), 2);
    assert_eq!(range[0].tick, 960);
    assert_eq!(range[1].tick, 1920);
}

#[test]
fn current_bpm_returns_active_bpm() {
    let chart: Chart = Chart {
        metadata: ChartMetadata::default(),
        resolution: 240,
        lane_count: 8,
        timing: TimingTrack {
            init_bpm: 120.0,
            bpm_changes: vec![BpmChange {
                tick: 480,
                bpm: 200.0,
            }],
            stops: vec![],
        },
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        notes: vec![],
        bgm: vec![],
        audio_assets: vec![],
        bar_lines: vec![],
        scroll_events: vec![],
        bga: Bga::default(),
    };
    let mut player = Player::new(chart);

    assert!((player.current_bpm() - 120.0).abs() < 1e-9);
    player.advance(Duration::from_secs(10));
    assert!((player.current_bpm() - 200.0).abs() < 1e-9);
}

#[test]
fn into_chart_returns_original_chart() {
    let chart = make_chart(vec![note(0, 0, NoteKind::Normal)]);
    let player = Player::new(chart);
    let recovered = player.into_chart();
    assert_eq!(recovered.notes.len(), 1);
}
