#![expect(missing_docs, reason = "integration test")]

mod helper;

use std::num::NonZeroU8;

use bmsrs_chart::{
    BpmChange, Chart, ChartData, ChartInfo, Damage, Event, LnJudgeHint, LnLifeHint, LnTypeHint,
    NoteKind, SongInfo, TimingTrack,
    mode::{Lane, NoteSide},
};
use bmsrs_player::Player;
use helper::{make_chart, note};
use std::time::Duration;

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

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
        let expected = TimingTrack::new(120.0, vec![], vec![]).tick_to_duration(tick, 240);
        let actual = player.tick_to_duration(tick);
        assert_eq!(actual, expected, "mismatch at tick {tick}");
    }
}

#[test]
fn notes_in_range_returns_subset() {
    let chart = make_chart(vec![
        note(0, key(1), NoteKind::Normal),
        note(240, key(2), NoteKind::Normal),
        note(480, key(1), NoteKind::Normal),
        note(720, key(2), NoteKind::Normal),
        note(960, key(1), NoteKind::Normal),
    ]);
    let player = Player::new(chart);

    let range: Vec<&Event<()>> = player.notes_in_range(240..720).collect();
    assert_eq!(range.len(), 2);
    assert_eq!(range[0].tick(), 240);
    assert_eq!(range[1].tick(), 480);
}

#[test]
fn notes_in_lane_filters_by_side_and_lane() {
    let chart = make_chart(vec![
        note(0, key(1), NoteKind::Normal),
        note(240, key(2), NoteKind::Normal),
        note(480, key(1), NoteKind::Normal),
    ]);
    let player = Player::new(chart);

    let lane1: Vec<&Event<()>> = player.notes_in_lane(NoteSide::P1, key(1), 0..960).collect();
    assert_eq!(lane1.len(), 2);
    assert_eq!(lane1[0].tick(), 0);
    assert_eq!(lane1[1].tick(), 480);
}

#[test]
fn notes_for_judgement_excludes_invisible_and_mines() {
    let chart = make_chart(vec![
        note(0, key(1), NoteKind::Normal),
        note(240, key(1), NoteKind::Invisible),
        note(
            480,
            key(1),
            NoteKind::Mine {
                damage: Damage::new(1.0),
            },
        ),
        note(720, key(1), NoteKind::Long { duration: 240 }),
    ]);
    let player = Player::new(chart);

    let judge_notes: Vec<&Event<()>> = player.notes_for_judgement(0..960).collect();
    assert_eq!(judge_notes.len(), 2);
    assert_eq!(judge_notes[0].tick(), 0);
    assert_eq!(judge_notes[1].tick(), 720);
}

#[test]
fn bgm_in_range_returns_events() {
    let chart: Chart = Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(120.0, vec![], vec![]),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![
                Event::Bgm {
                    tick: 0,
                    audio_index: 0,
                },
                Event::Bgm {
                    tick: 480,
                    audio_index: 1,
                },
                Event::Bgm {
                    tick: 960,
                    audio_index: 2,
                },
            ],
            audio_assets: vec![],
        },
    };
    let player = Player::new(chart);

    let range: Vec<&Event<()>> = player.bgm_in_range(240..960).collect();
    assert_eq!(range.len(), 1);
    assert_eq!(range[0].tick(), 480);
}

#[test]
fn scroll_rate_at_returns_latest_multiplier() {
    let chart: Chart = Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(120.0, vec![], vec![]),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![
                Event::Scroll {
                    tick: 240,
                    rate: 2.0,
                },
                Event::Scroll {
                    tick: 720,
                    rate: 0.5,
                },
            ],
            audio_assets: vec![],
        },
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
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(120.0, vec![], vec![]),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![
                Event::Bar { tick: 0 },
                Event::Bar { tick: 960 },
                Event::Bar { tick: 1920 },
            ],
            audio_assets: vec![],
        },
    };
    let player = Player::new(chart);

    let range: Vec<&Event<()>> = player.bar_lines_in_range(500..2000).collect();
    assert_eq!(range.len(), 2);
    assert_eq!(range[0].tick(), 960);
    assert_eq!(range[1].tick(), 1920);
}

#[test]
fn current_bpm_returns_active_bpm() {
    let chart: Chart = Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(
                120.0,
                vec![BpmChange {
                    tick: 480,
                    bpm: 200.0,
                }],
                vec![],
            ),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![],
            audio_assets: vec![],
        },
    };
    let mut player = Player::new(chart);

    assert!((player.current_bpm() - 120.0).abs() < 1e-9);
    player.advance(Duration::from_secs(10));
    assert!((player.current_bpm() - 200.0).abs() < 1e-9);
}

#[test]
fn duration_to_tick_constant_bpm() {
    let chart = make_chart(vec![]); // 120 BPM，脉冲 0-239 = 0.5s
    let player = Player::new(chart);

    assert_eq!(player.duration_to_tick(Duration::ZERO), 0);
    assert_eq!(player.duration_to_tick(Duration::from_secs(1)), 480);
    assert_eq!(player.duration_to_tick(Duration::from_secs(2)), 960);
}

#[test]
fn duration_to_tick_matches_timing_track() {
    let chart = make_chart(vec![]);
    let player = Player::new(chart);
    let track = TimingTrack::new(120.0, vec![], vec![]);

    for d_ms in [0u64, 100, 250, 500, 1000, 2000, 5000] {
        let dur = Duration::from_millis(d_ms);
        let expected = track.duration_to_tick(dur, 240);
        let actual = player.duration_to_tick(dur);
        assert_eq!(actual, expected, "mismatch at {d_ms}ms");
    }
}

#[test]
fn duration_to_tick_with_bpm_changes_and_stops() {
    let chart: Chart = Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(
                120.0,
                vec![BpmChange {
                    tick: 480,
                    bpm: 60.0,
                }],
                vec![bmsrs_chart::StopEvent {
                    tick: 960,
                    duration: 480,
                }],
            ),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![],
            audio_assets: vec![],
        },
    };
    let player = Player::new(chart);
    let track = TimingTrack::new(
        120.0,
        vec![BpmChange {
            tick: 480,
            bpm: 60.0,
        }],
        vec![bmsrs_chart::StopEvent {
            tick: 960,
            duration: 480,
        }],
    );

    for d_ms in [0u64, 100, 500, 1000, 2000, 4000, 8000] {
        let dur = Duration::from_millis(d_ms);
        let expected = track.duration_to_tick(dur, 240);
        let actual = player.duration_to_tick(dur);
        assert_eq!(actual, expected, "mismatch at {d_ms}ms");
    }
}

#[test]
fn duration_to_tick_stop_does_not_advance() {
    let chart: Chart = Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::new(
                120.0,
                vec![],
                vec![bmsrs_chart::StopEvent {
                    tick: 240,
                    duration: 480,
                }],
            ),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![],
            audio_assets: vec![],
        },
    };
    let player = Player::new(chart);

    // 停止前：240 个脉冲在 120 BPM 下 = 0.5s
    assert_eq!(player.duration_to_tick(Duration::from_secs_f64(0.5)), 240);

    // 停止期间：脉冲不应推进。
    assert_eq!(player.duration_to_tick(Duration::from_secs_f64(0.75)), 240);
    assert_eq!(player.duration_to_tick(Duration::from_secs_f64(1.0)), 240);

    // 停止后：480 个脉冲的时间先消耗掉停止时长
    // 0.5s（到停止点）+ 1.0s（120 BPM 下的停止）= 1.5s，随后继续
    assert_eq!(player.duration_to_tick(Duration::from_secs_f64(1.5)), 240);
    assert_eq!(player.duration_to_tick(Duration::from_secs_f64(2.0)), 480);
}

#[test]
fn into_chart_returns_original_chart() {
    let chart = make_chart(vec![note(0, key(1), NoteKind::Normal)]);
    let player = Player::new(chart);
    let recovered = player.into_chart();
    assert_eq!(recovered.data.events.len(), 1);
}
