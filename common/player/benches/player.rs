//! Player 热路径基准测试。
//!
//! 量化以下操作的耗时，为优化决策提供数据（而非猜测）：
//!
//! - `TimingTrack::duration_to_tick` —— `O(log n)` 二分
//! - `TimingTrack::tick_to_duration` —— `O(log n)` 二分
//! - `Player::advance` —— 播放循环每帧调用
//!
//! 运行：`cargo bench -p bmsrs-player`
#![expect(clippy::unwrap_used, reason = "bench code")]
#![expect(clippy::expect_used, reason = "bench code")]

use std::num::NonZeroU8;
use std::time::Duration;

use bmsrs_chart::{
    BpmChange, Chart, ChartData, ChartInfo, Event, EventKind, Lane, LnJudgeHint, LnLifeHint,
    LnTypeHint, NoteKind, NoteSide, SongInfo, StopEvent, TimingTrack,
};
use bmsrs_player::Player;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// 默认节拍分辨率（每四分音符的脉冲数）。
const RESOLUTION: u64 = 240;
/// 第一个常规按键（1P 侧），用 `NonZeroU8::MIN` 避免 `unwrap`。
const KEY1: NonZeroU8 = NonZeroU8::MIN;
/// 每帧增量（约 60 FPS）。
const FRAME: Duration = Duration::from_nanos(16_666_667);

/// 构建一个含 `n` 个音符、若干 BPM 变更与停止的谱面。
///
/// 模拟真实谱面密度：每拍约一个音符，每隔固定间隔插入 BPM 变更与停止，
/// 使计时轨非平凡。
#[expect(
    clippy::cast_precision_loss,
    reason = "indices fit in f64 for benchmark sizes"
)]
fn build_chart(n: usize) -> Chart {
    let mut events = Vec::with_capacity(n + n / 50 + 4);

    for i in 0..n {
        let tick = (i as u64) * RESOLUTION; // 每拍一个音符
        events.push(Event::new(
            tick,
            EventKind::Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Normal,
                audio_index: None,
                ext: (),
            },
        ));
    }

    // 每隔 50 个音符插入一次 BPM 变更。
    let mut bpm_changes = Vec::new();
    for i in (0..n).step_by(50) {
        let tick = (i as u64) * RESOLUTION;
        let bpm = 120.0 + ((i % 200) as f64);
        events.push(Event::bpm(tick, bpm));
        bpm_changes.push(BpmChange { tick, bpm });
    }

    // 每隔 100 个音符插入一次停止。
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
            resolution: RESOLUTION,
            timing,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
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

/// 基准：`TimingTrack::duration_to_tick`（`O(n)` 旧路径，P1 前用于 advance）。
///
/// 目标取谱面 90% 处的时刻，使 `O(n)` 扫描处理绝大多数事件
/// （反映后期游玩中每帧的最坏情况）。
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

/// 基准：`TimingTrack::duration_to_tick`（`O(log n)` 二分）。
fn bench_timing_cache_duration_to_tick(c: &mut Criterion) {
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

/// 基准：`TimingTrack::tick_to_duration`（`O(log n)` 二分）。
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

/// 基准：`Player::advance`（播放循环每帧调用）。
///
/// 预先将 player seek 到谱面 90% 处，测量后期游玩中每帧的最坏情况。
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

/// 构建一个 BGM 远多于 Note 的高密度谱面（模拟真实 BMS：每个按键音都是 BGM）。
///
/// `bgm_per_beat` 控制每拍的 BGM 数。用于量化 AOS 下从含大量 BGM 的
/// 时间窗过滤 Note 事件的成本。
fn build_dense_chart(n_notes: usize, bgm_per_beat: usize) -> Chart {
    let mut events = Vec::with_capacity(n_notes * (1 + bgm_per_beat) + 4);

    // 每拍一个 Note。
    for i in 0..n_notes {
        let tick = (i as u64) * RESOLUTION;
        events.push(Event::new(
            tick,
            EventKind::Note {
                side: NoteSide::P1,
                lane: Lane::Key(KEY1),
                kind: NoteKind::Normal,
                audio_index: None,
                ext: (),
            },
        ));
    }

    // 每拍 bgm_per_beat 个 BGM（均布于拍内）。
    for i in 0..n_notes {
        let beat_start = (i as u64) * RESOLUTION;
        for j in 0..bgm_per_beat {
            let tick = beat_start + (RESOLUTION * j as u64 / bgm_per_beat.max(1) as u64);
            events.push(Event::bgm(tick, 0));
        }
    }

    events.sort_by_key(Event::sort_key);

    Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: RESOLUTION,
            timing: TimingTrack::simple(120.0, RESOLUTION).unwrap(),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
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

/// 基准：`Player::events_in_range` / `notes_in_range`（渲染层每帧查询）。
///
/// 量化 AOS（当前统一 `Vec<Event>`）下，从含大量 BGM 的时间窗中
/// 过滤 Note 事件的实际成本，为是否需要 AOS→SOA 分桶提供数据。
fn bench_player_events_in_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("player::events_in_range");
    for &n in &[2000usize, 8000] {
        // 标准谱面（无 BGM）。
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

    // BGM 密集场景：每拍 10 个 BGM（Note:BGM = 1:10），模拟真实 BMS。
    // 固定 2000 个 Note，窗口内含约 20 Note + 200 BGM。
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
    bench_timing_cache_duration_to_tick,
    bench_timing_cache_tick_to_duration,
    bench_player_advance,
    bench_player_events_in_range,
);
criterion_main!(benches);
