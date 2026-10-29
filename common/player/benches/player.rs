//! Player hot-path benchmarks.
//!
//! Quantifies the cost of the following operations to support optimization
//! decisions with data instead of guesswork:
//!
//! - `TimingTrack::duration_to_tick`: `O(log n)` binary search
//! - `TimingTrack::tick_to_duration`: `O(log n)` binary search
//! - `Player::advance`: called every frame by the playback loop
//!
//! Run: `cargo bench -p bmsrs-player`
#![expect(clippy::unwrap_used, reason = "bench code")]
#![expect(clippy::expect_used, reason = "bench code")]

use std::num::NonZeroU8;
use std::time::Duration;

use bmsrs_chart::{
    BpmChange, Chart, ChartData, ChartInfo, Event, EventKind, Lane, LnJudgeHint, LnLifeHint,
    LnTypeHint, NoteAudio, NoteKind, NoteRole, NoteSide, SongInfo, StopEvent, TimingTrack,
};
use bmsrs_player::Player;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Default beat resolution (ticks per quarter note).
const RESOLUTION: u64 = 240;
/// First regular key on the 1P side, using `NonZeroU8::MIN` to avoid `unwrap`.
const KEY1: NonZeroU8 = NonZeroU8::MIN;
/// Per-frame delta (about 60 FPS).
const FRAME: Duration = Duration::from_nanos(16_666_667);

/// Build a chart with `n` notes, several BPM changes, and stops.
///
/// Simulates real chart density: about one note per beat, with BPM changes and
/// stops inserted at fixed intervals to make the timing track non-trivial.
#[expect(
    clippy::cast_precision_loss,
    reason = "indices fit in f64 for benchmark sizes"
)]
fn build_chart(n: usize) -> Chart {
    let mut events = Vec::with_capacity(n + n / 50 + 4);

    for i in 0..n {
        let tick = (i as u64) * RESOLUTION; // One note per beat.
        events.push(Event::new(
            tick,
            EventKind::Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Normal,
                role: NoteRole::Normal,
                audio: NoteAudio::none(),
                ext: (),
            },
        ));
    }

    // Insert a BPM change every 50 notes.
    let mut bpm_changes = Vec::new();
    for i in (0..n).step_by(50) {
        let tick = (i as u64) * RESOLUTION;
        let bpm = 120.0 + ((i % 200) as f64);
        events.push(Event::bpm(tick, bpm));
        bpm_changes.push(BpmChange { tick, bpm });
    }

    // Insert a stop every 100 notes.
    let mut stops = Vec::new();
    for i in (0..n).step_by(100) {
        let tick = (i as u64) * RESOLUTION;
        events.push(Event::new(
            tick,
            EventKind::Stop {
                duration: RESOLUTION,
            },
        ));
        stops.push(StopEvent {
            tick,
            duration: RESOLUTION,
        });
    }

    events.sort_by_key(Event::sort_key);

    let timing = TimingTrack::new(120.0, bpm_changes, stops, RESOLUTION).unwrap();

    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            timing,
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events,
            judge_deltas: None,
            life_deltas: None,
            audio_assets: vec![],
        },
    }
}

/// Benchmark: `TimingTrack::duration_to_tick` (`O(log n)` binary search).
///
/// Target the moment at 90% of the chart so the inverse lookup covers most
/// events (reflecting the worst case per frame late in play).
fn bench_timing_track_duration_to_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("duration_to_tick / timing_track [O(log n)]");
    for &n in &[500usize, 2000, 8000] {
        let chart = build_chart(n);
        let timing = &chart.data.timing;
        let last_tick = chart.data.last_tick();
        let target = timing.tick_to_duration(last_tick * 9 / 10);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| black_box(timing.duration_to_tick(target)));
        });
    }
    group.finish();
}

/// Benchmark: `TimingTrack::tick_to_duration` (`O(log n)` binary search).
fn bench_timing_cache_tick_to_duration(c: &mut Criterion) {
    let mut group = c.benchmark_group("tick_to_duration / timing_track [O(log n)]");
    for &n in &[500usize, 2000, 8000] {
        let chart = build_chart(n);
        let timing = &chart.data.timing;
        let tick = chart.data.last_tick() * 9 / 10;
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| black_box(timing.tick_to_duration(tick)));
        });
    }
    group.finish();
}

/// Benchmark: `Player::advance` (called every frame by the playback loop).
///
/// Seeks the player to 90% of the chart first, measuring the worst case per frame
/// late in play.
fn bench_player_advance(c: &mut Criterion) {
    let mut group = c.benchmark_group("player::advance (per-frame)");
    for &n in &[500usize, 2000, 8000] {
        let chart = build_chart(n);
        let last_tick = chart.data.last_tick();
        let seek_target = chart.data.timing.tick_to_duration(last_tick * 9 / 10);
        let mut player = Player::new(chart).expect("valid chart for benchmark");
        player.seek(seek_target);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| player.advance(FRAME));
        });
    }
    group.finish();
}

/// Build a dense chart with far more BGM than notes (simulating real BMS, where
/// every key sound is also BGM).
///
/// `bgm_per_beat` controls the BGM count per beat. Used to quantify the cost of
/// filtering Note events out of a window containing many BGM events under AOS.
fn build_dense_chart(n_notes: usize, bgm_per_beat: usize) -> Chart {
    let mut events = Vec::with_capacity(n_notes * (1 + bgm_per_beat) + 4);

    // One Note per beat.
    for i in 0..n_notes {
        let tick = (i as u64) * RESOLUTION;
        events.push(Event::new(
            tick,
            EventKind::Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Normal,
                role: NoteRole::Normal,
                audio: NoteAudio::none(),
                ext: (),
            },
        ));
    }

    // `bgm_per_beat` BGM events per beat (evenly distributed within the beat).
    for i in 0..n_notes {
        let beat_start = (i as u64) * RESOLUTION;
        for j in 0..bgm_per_beat {
            let tick = beat_start + (RESOLUTION * j as u64 / bgm_per_beat.max(1) as u64);
            events.push(Event::bgm(tick, bmsrs_chart::AudioIndex::new(0)));
        }
    }

    events.sort_by_key(Event::sort_key);

    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            timing: TimingTrack::simple(120.0, RESOLUTION).unwrap(),
            judge_multiplier: Some(1.0),
            life_multiplier: Some(1.0),
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events,
            judge_deltas: None,
            life_deltas: None,
            audio_assets: vec![],
        },
    }
}

/// Benchmark: `Player::events_in_range` / `notes_in_range` (per-frame renderer
/// queries).
///
/// Quantifies the real cost of filtering Note events out of a window containing
/// many BGM events under AOS (the current unified `Vec<Event>`), providing data
/// for deciding whether AOS-to-SOA bucketing is needed.
fn bench_player_events_in_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("player::events_in_range");
    for &n in &[2000usize, 8000] {
        // Standard chart (no BGM).
        let chart = build_chart(n);
        let mid_tick = chart.data.last_tick() / 2;
        let seek_target = chart.data.timing.tick_to_duration(mid_tick);
        let mut player = Player::new(chart).expect("valid chart for benchmark");
        player.seek(seek_target);
        let start = mid_tick;
        let end = mid_tick + 480;
        group.bench_with_input(BenchmarkId::new("all_events", n), &n, |b, _| {
            b.iter(|| {
                black_box(player.events_in_range(start..end).len());
            });
        });
        group.bench_with_input(BenchmarkId::new("notes_filter", n), &n, |b, _| {
            b.iter(|| {
                black_box(player.notes_in_range(start..end).count());
            });
        });
    }

    // BGM-dense scenario: 10 BGM events per beat (Note:BGM = 1:10), simulating
    // real BMS. Fixed 2000 notes; the window contains about 20 notes and 200 BGM
    // events.
    let chart = build_dense_chart(2000, 10);
    let mid_tick = chart.data.last_tick() / 2;
    let seek_target = chart.data.timing.tick_to_duration(mid_tick);
    let mut player = Player::new(chart).expect("valid chart for benchmark");
    player.seek(seek_target);
    let start = mid_tick;
    let end = mid_tick + 480;
    group.bench_function("notes_filter (1:10 BGM)", |b| {
        b.iter(|| black_box(player.notes_in_range(start..end).count()));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_timing_track_duration_to_tick,
    bench_timing_cache_tick_to_duration,
    bench_player_advance,
    bench_player_events_in_range,
);
criterion_main!(benches);
