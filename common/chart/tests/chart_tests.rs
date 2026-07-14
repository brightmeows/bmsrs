//! `bmsrs-chart` 的集成测试。
//!
//! 覆盖 Chart 模型、TimingTrack、Event、NoteKind 等核心类型。
#![expect(clippy::unwrap_used, reason = "test code")]

use std::num::NonZeroU8;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BgaLayer, BgaResource, BpmChange, Chart, ChartData, ChartInfo, Damage, Event,
    EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint, NoCustomEvent, NoteKind, NoteSide,
    SongInfo, StopEvent, TimingTrack,
};

const fn nz(n: u8) -> NonZeroU8 {
    match NonZeroU8::new(n) {
        Some(v) => v,
        None => panic!("nz: n must be non-zero"),
    }
}

const fn key(n: u8) -> Lane {
    Lane::Key(nz(n))
}

// Chart 构造

#[test]
fn chart_construction_is_empty_by_default() {
    // Chart 不实现 Default（内嵌 ChartData 无合法默认），
    // 此处验证一个显式构造的空谱面：元数据为空、无事件。
    let chart: Chart = Chart {
        song: SongInfo::default(),
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::simple(120.0).unwrap(),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: vec![],
            audio_assets: vec![],
        },
    };
    assert!(chart.song.title.is_empty());
    assert!(chart.chart.subtitle.is_empty());
    assert!(chart.data.events.is_empty());
}

#[test]
fn chart_construction() {
    let chart: Chart = Chart {
        song: SongInfo {
            title: "Test".into(),
            ..Default::default()
        },
        chart: ChartInfo::default(),
        data: ChartData {
            resolution: 240,
            timing: TimingTrack::simple(120.0).unwrap(),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: vec![Event::new(
                0,
                EventKind::Note {
                    side: NoteSide::P1,
                    lane: key(1),
                    kind: NoteKind::Normal,
                    audio_index: None,
                    ext: (),
                },
            )],
            audio_assets: vec![],
        },
    };
    assert_eq!(chart.song.title, "Test");
    assert_eq!(chart.data.events.len(), 1);
}

#[test]
fn chart_song_info_default() {
    let info = SongInfo::default();
    assert!(info.title.is_empty());
    assert!(info.artist.is_empty());
    assert!(info.genre.is_empty());
    assert!(info.subartists.is_empty());
}

#[test]
fn chart_info_default() {
    let info = ChartInfo::default();
    assert!(info.subtitle.is_empty());
    assert!(info.chart_name.is_empty());
    assert_eq!(info.level, 0);
    assert!(info.bga_resources.is_empty());
}

// ChartData

#[test]
fn chart_data_last_tick_empty() {
    // 空事件列表 → last_tick() 返回 0。
    let data = ChartData::<(), NoCustomEvent> {
        resolution: 240,
        timing: TimingTrack::simple(120.0).unwrap(),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        judge_deltas: None,
        life_deltas: None,
        events: vec![],
        audio_assets: vec![],
    };
    assert_eq!(data.last_tick(), 0);
}

#[test]
fn chart_data_last_tick_with_events() {
    let data: ChartData = ChartData {
        resolution: 240,
        timing: TimingTrack::simple(120.0).unwrap(),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        judge_deltas: None,
        life_deltas: None,
        events: vec![Event::bar(0), Event::bar(960)],
        audio_assets: vec![],
    };
    assert_eq!(data.last_tick(), 960);
}

#[test]
fn chart_data_duration() {
    let data: ChartData = ChartData {
        resolution: 240,
        timing: TimingTrack::simple(120.0).unwrap(),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        judge_deltas: None,
        life_deltas: None,
        events: vec![Event::bar(960)],
        audio_assets: vec![],
    };
    // 960 ticks at 120 BPM with resolution 240 = 2 seconds
    assert_eq!(data.duration(), Duration::from_secs_f64(2.0));
}

// Event

#[test]
fn event_tick_accessor() {
    assert_eq!(Event::<()>::bar(42).tick(), 42);
    assert_eq!(
        Event::<()>::new(
            100,
            EventKind::Note {
                side: NoteSide::P1,
                lane: key(1),
                kind: NoteKind::Normal,
                audio_index: None,
                ext: (),
            },
        )
        .tick(),
        100
    );
    assert_eq!(
        Event::<()>::new(200, EventKind::Bpm { bpm: 180.0 },).tick(),
        200
    );
}

#[test]
fn event_priority_order() {
    assert!(
        Event::<()>::bar(0).priority()
            < Event::<()>::new(
                0,
                EventKind::Note {
                    side: NoteSide::P1,
                    lane: key(1),
                    kind: NoteKind::Normal,
                    audio_index: None,
                    ext: (),
                },
            )
            .priority()
    );
    assert!(
        Event::<()>::new(
            0,
            EventKind::Note {
                side: NoteSide::P1,
                lane: key(1),
                kind: NoteKind::Normal,
                audio_index: None,
                ext: (),
            },
        )
        .priority()
            < Event::<()>::new(0, EventKind::Bpm { bpm: 120.0 },).priority()
    );
    assert!(
        Event::<()>::new(0, EventKind::Bpm { bpm: 120.0 },).priority()
            < Event::<()>::new(0, EventKind::Stop { duration: 192 },).priority()
    );
    assert!(
        Event::<()>::new(0, EventKind::Stop { duration: 192 },).priority()
            < Event::<()>::new(0, EventKind::Scroll { rate: 1.0 }).priority()
    );
    assert!(
        Event::<()>::new(0, EventKind::Scroll { rate: 1.0 }).priority()
            < Event::<()>::new(0, EventKind::Speed { rate: 1.0 }).priority()
    );
}

#[test]
fn event_bga_fields() {
    let ev = Event::<()>::new(
        480,
        EventKind::Bga {
            layer: BgaLayer::Layer2,
            resource_id: 3,
        },
    );
    assert_eq!(ev.tick(), 480);
    if let EventKind::Bga {
        layer, resource_id, ..
    } = ev.kind
    {
        assert_eq!(layer, BgaLayer::Layer2);
        assert_eq!(resource_id, 3);
    } else {
        panic!("expected Bga event");
    }
}

#[test]
fn event_note_with_ext() {
    let ev = Event::<()>::new(
        240,
        EventKind::Note {
            side: NoteSide::P2,
            lane: Lane::Scratch(nz(1)),
            kind: NoteKind::Mine {
                damage: Damage::new(25.0),
            },
            audio_index: Some(0),
            ext: (),
        },
    );
    assert_eq!(ev.tick(), 240);
    if let EventKind::Note {
        side,
        lane,
        kind,
        audio_index,
        ..
    } = ev.kind
    {
        assert_eq!(side, NoteSide::P2);
        assert_eq!(lane, Lane::Scratch(nz(1)));
        assert_eq!(audio_index, Some(0));
        assert!(matches!(kind, NoteKind::Mine { .. }));
    } else {
        panic!("expected Note event");
    }
}

// NoteKind

#[test]
fn note_kind_normal_default() {
    assert_eq!(NoteKind::default(), NoteKind::Normal);
}

#[test]
fn note_kind_long_duration() {
    let nk = NoteKind::Long { duration: 480 };
    if let NoteKind::Long { duration } = nk {
        assert_eq!(duration, 480);
    } else {
        panic!("expected Long");
    }
}

#[test]
fn note_kind_mine_damage() {
    let nk = NoteKind::Mine {
        damage: Damage::new(5.0),
    };
    if let NoteKind::Mine { damage } = nk {
        assert!((damage.get() - 5.0).abs() < f64::EPSILON);
    } else {
        panic!("expected Mine");
    }
}

// Damage

#[test]
fn damage_creation() {
    let d = Damage::new(10.0);
    assert!((d.get() - 10.0).abs() < f64::EPSILON);
}

#[test]
fn damage_default() {
    let d = Damage::default();
    assert!((d.get() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn damage_eq() {
    assert_eq!(Damage::new(5.0), Damage::new(5.0));
    assert_ne!(Damage::new(5.0), Damage::new(10.0));
}

// NoteSide & Lane

#[test]
fn note_side_constants() {
    assert_eq!(NoteSide::P1.as_u8(), 1);
    assert_eq!(NoteSide::P2.as_u8(), 2);
}

#[test]
fn lane_variants() {
    assert!(matches!(key(1), Lane::Key(_)));
    assert!(matches!(Lane::Scratch(nz(1)), Lane::Scratch(_)));
    assert!(matches!(Lane::FootPedal, Lane::FootPedal));
}

// BgaLayer

#[test]
fn bga_layer_default() {
    assert_eq!(BgaLayer::default(), BgaLayer::Base);
}

#[test]
fn bga_layer_order() {
    assert_ne!(BgaLayer::Base, BgaLayer::Layer);
    assert_ne!(BgaLayer::Layer, BgaLayer::Layer2);
    assert_ne!(BgaLayer::Poor, BgaLayer::Base);
}

// AudioAsset

#[test]
fn audio_asset_path_construction() {
    let asset = AudioAsset::from_path_buf("kick.wav".into(), Duration::ZERO, None);
    assert_eq!(asset.path.to_string_lossy(), "kick.wav");
    assert_eq!(asset.start, Duration::ZERO);
    assert_eq!(asset.duration, None);
}

#[test]
fn audio_asset_with_duration() {
    let asset = AudioAsset::from_path_buf(
        "loop.ogg".into(),
        Duration::from_millis(500),
        Some(Duration::from_secs(2)),
    );
    assert_eq!(asset.start, Duration::from_millis(500));
    assert_eq!(asset.duration, Some(Duration::from_secs(2)));
}

// BgaResource

#[test]
fn bga_resource_construction() {
    let res = BgaResource {
        id: 1,
        path: "bg.png".into(),
        crop: None,
    };
    assert_eq!(res.id, 1);
    assert_eq!(res.path.to_string_lossy(), "bg.png");
    assert!(res.crop.is_none());
}

// NoteExt

#[test]
fn unit_note_ext() {
    let ext: () = <_ as Clone>::clone(&());
    assert_eq!(ext, ());
}

// TimingTrack

#[test]
fn timing_track_default() {
    let tt = TimingTrack::default();
    assert!((tt.init_bpm() - 120.0).abs() < f64::EPSILON);
    assert!(tt.bpm_changes().is_empty());
    assert!(tt.stops().is_empty());
}

#[test]
fn timing_track_new() {
    let tt = TimingTrack::simple(120.0).unwrap();
    assert!((tt.init_bpm() - 120.0).abs() < f64::EPSILON);
}

#[test]
fn tick_to_duration_constant_bpm() {
    let tt = TimingTrack::simple(120.0).unwrap();
    // 240 ticks at 120 BPM with resolution 240 = 0.5s
    let dur = tt.tick_to_duration(240, 240);
    assert!((dur.as_secs_f64() - 0.5).abs() < 1e-9);
}

#[test]
fn tick_to_duration_bpm_change() {
    let tt = TimingTrack::new(
        120.0,
        vec![BpmChange {
            tick: 240,
            bpm: 240.0,
        }],
        vec![],
    )
    .unwrap();
    // 0-240 ticks at 120 BPM = 0.5s, 240-480 ticks at 240 BPM = 0.25s
    let dur = tt.tick_to_duration(480, 240);
    assert!((dur.as_secs_f64() - 0.75).abs() < 1e-9);
}

#[test]
fn tick_to_duration_with_stop() {
    let tt = TimingTrack::new(
        120.0,
        vec![],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
    )
    .unwrap();
    // 0-240 ticks = 0.5s. At tick 240, a stop of 240 ticks = 0.5s.
    // tick_to_duration(240) should return time BEFORE stop = 0.5s.
    let dur_at_stop = tt.tick_to_duration(240, 240);
    assert!((dur_at_stop.as_secs_f64() - 0.5).abs() < 1e-9);
    // tick_to_duration(480) = 0.5s (first segment) + 0.5s (stop) + 0.5s (after stop) = 1.5s
    let dur_after_stop = tt.tick_to_duration(480, 240);
    assert!((dur_after_stop.as_secs_f64() - 1.5).abs() < 1e-9);
}

#[test]
fn duration_to_tick_constant_bpm() {
    let tt = TimingTrack::simple(120.0).unwrap();
    // 0.5s at 120 BPM with resolution 240 = 240 ticks
    let tick = tt.duration_to_tick(Duration::from_secs_f64(0.5), 240);
    assert_eq!(tick, 240);
}

#[test]
fn duration_to_tick_with_stop() {
    let tt = TimingTrack::new(
        120.0,
        vec![],
        vec![StopEvent {
            tick: 240,
            duration: 240,
        }],
    )
    .unwrap();
    // Within stop: 0.5s (to stop) + 0.25s (into stop) → should still be at tick 240
    let tick = tt.duration_to_tick(Duration::from_secs_f64(0.75), 240);
    assert_eq!(tick, 240);
    // After stop: 0.5s (to stop) + 0.5s (stop) + 0.25s (after) → tick 360
    let tick2 = tt.duration_to_tick(Duration::from_secs_f64(1.25), 240);
    assert_eq!(tick2, 360);
}

#[test]
fn duration_to_tick_zero_duration() {
    let tt = TimingTrack::simple(120.0).unwrap();
    let tick = tt.duration_to_tick(Duration::ZERO, 240);
    assert_eq!(tick, 0);
}

#[test]
fn tick_to_duration_zero_tick() {
    let tt = TimingTrack::simple(120.0).unwrap();
    let dur = tt.tick_to_duration(0, 240);
    assert!(dur.is_zero());
}

#[test]
fn timing_track_clone_resets_cache() {
    let tt = TimingTrack::simple(120.0).unwrap();
    let _primed = tt.tick_to_duration(240, 240);
    assert!((tt.init_bpm() - 120.0).abs() < f64::EPSILON);
}

#[test]
fn timing_track_partial_eq() {
    let a = TimingTrack::simple(120.0).unwrap();
    let b = TimingTrack::simple(120.0).unwrap();
    let c = TimingTrack::simple(140.0).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

/// 负 BPM roundtrip——`tick_to_duration` 与 `duration_to_tick` 互为反函数。
/// 覆盖 A12 场景。
#[test]
fn timing_track_negative_bpm_roundtrip() {
    let tt = TimingTrack::new(
        120.0,
        vec![
            BpmChange {
                tick: 240,
                bpm: -60.0,
            },
            BpmChange {
                tick: 720,
                bpm: -180.0,
            },
            BpmChange {
                tick: 1200,
                bpm: 120.0,
            },
        ],
        vec![StopEvent {
            tick: 480,
            duration: 120,
        }],
    )
    .unwrap();
    let resolution = 240;

    for tick in [0, 120, 240, 360, 480, 600, 720, 960, 1200, 1440] {
        let dur = tt.tick_to_duration(tick, resolution);
        let tick_back = tt.duration_to_tick(dur, resolution);
        assert_eq!(
            tick_back, tick,
            "roundtrip failed at tick {tick}: {dur:?} → {tick_back}"
        );
    }
}

#[test]
fn bpm_change_fields() {
    let bc = BpmChange {
        tick: 960,
        bpm: 180.0,
    };
    assert_eq!(bc.tick, 960);
    assert!((bc.bpm - 180.0).abs() < f64::EPSILON);
}

#[test]
fn stop_event_fields() {
    let se = StopEvent {
        tick: 480,
        duration: 192,
    };
    assert_eq!(se.tick, 480);
    assert_eq!(se.duration, 192);
}

// NoCustomEvent — 纯标记类型，无行为需测试。

// EventKind::map_custom / map_ext

/// 测试用自定义事件类型。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TestCustom(String);

impl bmsrs_chart::CustomEvent for TestCustom {}

#[test]
fn map_custom_preserves_non_custom_variants() {
    let kind: EventKind<(), TestCustom> = EventKind::Bpm { bpm: 180.0 };
    let mapped = kind.map_custom(|_| NoCustomEvent);
    assert!(matches!(mapped, EventKind::Bpm { bpm: 180.0 }));
}

#[test]
fn map_custom_applies_fn_to_custom_variant() {
    let kind: EventKind<(), TestCustom> = EventKind::Custom(TestCustom("hello".into()));
    let mapped: EventKind<(), NoCustomEvent> = kind.map_custom(|_| NoCustomEvent);
    assert!(matches!(mapped, EventKind::Custom(NoCustomEvent)));
}

#[test]
fn map_ext_preserves_non_note_variants() {
    let kind: EventKind<i32, NoCustomEvent> = EventKind::Bar;
    let mapped = kind.map_ext(|_| ());
    assert!(matches!(mapped, EventKind::Bar));
}

#[test]
fn map_ext_applies_fn_to_note_variant() {
    let kind = EventKind::<i32, NoCustomEvent>::Note {
        side: NoteSide::P1,
        lane: key(1),
        kind: NoteKind::Normal,
        audio_index: None,
        ext: 42,
    };
    let mapped = kind.map_ext(|e| {
        assert_eq!(e, 42);
    });
    if let EventKind::Note { ext, .. } = mapped {
        assert_eq!(ext, ());
    } else {
        panic!("expected Note");
    }
}

// ChartData / Chart::filter_map_events

fn make_simple_chart_data() -> ChartData<(), NoCustomEvent> {
    ChartData {
        resolution: 240,
        timing: TimingTrack::simple(120.0).unwrap(),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        judge_deltas: None,
        life_deltas: None,
        events: vec![
            Event::bar(0),
            Event::new(
                240,
                EventKind::Note {
                    side: NoteSide::P1,
                    lane: key(1),
                    kind: NoteKind::Normal,
                    audio_index: None,
                    ext: (),
                },
            ),
            Event::bpm(480, 180.0),
        ],
        audio_assets: vec![],
    }
}

#[test]
fn filter_map_events_preserves_all_when_none_filtered() {
    let data = make_simple_chart_data();
    let mapped: ChartData<(), NoCustomEvent> = data
        .filter_map_events(|e| Some(Event::new(e.tick(), e.kind.map_custom(|_| NoCustomEvent))));
    assert_eq!(mapped.events.len(), 3);
    assert_eq!(mapped.resolution, 240);
    assert!((mapped.timing.init_bpm() - 120.0).abs() < f64::EPSILON);
}

#[test]
fn filter_map_events_drops_custom_events() {
    let data = ChartData {
        resolution: 240,
        timing: TimingTrack::simple(120.0).unwrap(),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        judge_deltas: None,
        life_deltas: None,
        events: vec![
            Event::bar(0),
            Event::new(0, EventKind::<(), NoCustomEvent>::Custom(NoCustomEvent)),
            Event::bpm(240, 150.0),
        ],
        audio_assets: vec![],
    };

    let mapped: ChartData<(), NoCustomEvent> = data.filter_map_events(|e| {
        let tick = e.tick();
        match e.kind {
            EventKind::Custom(_) => None,
            kind => Some(Event::new(tick, kind.map_custom(|_| NoCustomEvent))),
        }
    });

    assert_eq!(mapped.events.len(), 2);
    assert!(matches!(mapped.events[0].kind, EventKind::Bar));
    assert!(matches!(mapped.events[1].kind, EventKind::Bpm { .. }));
}

#[test]
fn chart_filter_map_events_preserves_metadata() {
    let chart = Chart {
        song: SongInfo {
            title: "Test".into(),
            ..Default::default()
        },
        chart: ChartInfo {
            chart_name: "HYPER".into(),
            ..Default::default()
        },
        data: make_simple_chart_data(),
    };

    let mapped: Chart<(), NoCustomEvent> = chart
        .filter_map_events(|e| Some(Event::new(e.tick(), e.kind.map_custom(|_| NoCustomEvent))));

    assert_eq!(mapped.song.title, "Test");
    assert_eq!(mapped.chart.chart_name, "HYPER");
    assert_eq!(mapped.data.events.len(), 3);
}
