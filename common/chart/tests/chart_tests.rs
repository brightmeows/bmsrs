//! `bmsrs-chart` 的集成测试。
//!
//! 覆盖 Chart 模型、TimingTrack、Event、NoteKind 等核心类型。

use std::num::NonZeroU8;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BgaLayer, BgaResource, BpmChange, Chart, ChartData, ChartInfo, CustomEvent as _,
    Damage, Event, Lane, LnJudgeHint, LnLifeHint, LnTypeHint, NoCustomEvent, NoteKind, NoteSide,
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
            timing: TimingTrack::new(120.0, vec![], vec![]),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
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
            timing: TimingTrack::new(120.0, vec![], vec![]),
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint: LnTypeHint::default(),
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            events: vec![Event::Note {
                tick: 0,
                side: NoteSide::P1,
                lane: key(1),
                kind: NoteKind::Normal,
                audio_index: None,
                ext: (),
            }],
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
        timing: TimingTrack::new(120.0, vec![], vec![]),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        events: vec![],
        audio_assets: vec![],
    };
    assert_eq!(data.last_tick(), 0);
}

#[test]
fn chart_data_last_tick_with_events() {
    let data: ChartData = ChartData {
        resolution: 240,
        timing: TimingTrack::new(120.0, vec![], vec![]),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        events: vec![Event::Bar { tick: 0 }, Event::Bar { tick: 960 }],
        audio_assets: vec![],
    };
    assert_eq!(data.last_tick(), 960);
}

#[test]
fn chart_data_duration() {
    let data: ChartData = ChartData {
        resolution: 240,
        timing: TimingTrack::new(120.0, vec![], vec![]),
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        events: vec![Event::Bar { tick: 960 }],
        audio_assets: vec![],
    };
    // 960 ticks at 120 BPM with resolution 240 = 2 seconds
    assert_eq!(data.duration(), Duration::from_secs_f64(2.0));
}

// Event

#[test]
fn event_tick_accessor() {
    assert_eq!(Event::<(), NoCustomEvent>::Bar { tick: 42 }.tick(), 42);
    assert_eq!(
        Event::<(), NoCustomEvent>::Note {
            tick: 100,
            side: NoteSide::P1,
            lane: key(1),
            kind: NoteKind::Normal,
            audio_index: None,
            ext: (),
        }
        .tick(),
        100
    );
    assert_eq!(
        Event::<(), NoCustomEvent>::Bpm {
            tick: 200,
            bpm: 180.0,
        }
        .tick(),
        200
    );
}

#[test]
fn event_priority_order() {
    assert!(
        Event::<(), NoCustomEvent>::Bar { tick: 0 }.priority()
            < Event::<(), NoCustomEvent>::Note {
                tick: 0,
                side: NoteSide::P1,
                lane: key(1),
                kind: NoteKind::Normal,
                audio_index: None,
                ext: ()
            }
            .priority()
    );
    assert!(
        Event::<(), NoCustomEvent>::Note {
            tick: 0,
            side: NoteSide::P1,
            lane: key(1),
            kind: NoteKind::Normal,
            audio_index: None,
            ext: ()
        }
        .priority()
            < Event::<(), NoCustomEvent>::Bpm {
                tick: 0,
                bpm: 120.0
            }
            .priority()
    );
    assert!(
        Event::<(), NoCustomEvent>::Bpm {
            tick: 0,
            bpm: 120.0
        }
        .priority()
            < Event::<(), NoCustomEvent>::Stop {
                tick: 0,
                duration: 192
            }
            .priority()
    );
    assert!(
        Event::<(), NoCustomEvent>::Stop {
            tick: 0,
            duration: 192
        }
        .priority()
            < Event::<(), NoCustomEvent>::Scroll { tick: 0, rate: 1.0 }.priority()
    );
    assert!(
        Event::<(), NoCustomEvent>::Scroll { tick: 0, rate: 1.0 }.priority()
            < Event::<(), NoCustomEvent>::Speed { tick: 0, rate: 1.0 }.priority()
    );
}

#[test]
fn event_bga_fields() {
    let ev = Event::<(), NoCustomEvent>::Bga {
        tick: 480,
        layer: BgaLayer::Layer2,
        resource_id: 3,
    };
    assert_eq!(ev.tick(), 480);
    if let Event::Bga {
        layer, resource_id, ..
    } = ev
    {
        assert_eq!(layer, BgaLayer::Layer2);
        assert_eq!(resource_id, 3);
    } else {
        panic!("expected Bga event");
    }
}

#[test]
fn event_note_with_ext() {
    let ev = Event::<(), NoCustomEvent>::Note {
        tick: 240,
        side: NoteSide::P2,
        lane: Lane::Scratch(nz(1)),
        kind: NoteKind::Mine {
            damage: Damage::new(25.0),
        },
        audio_index: Some(0),
        ext: (),
    };
    assert_eq!(ev.tick(), 240);
    if let Event::Note {
        side,
        lane,
        kind,
        audio_index,
        ..
    } = ev
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
    assert!(BgaLayer::Base != BgaLayer::Layer);
    assert!(BgaLayer::Layer != BgaLayer::Layer2);
    assert!(BgaLayer::Poor != BgaLayer::Base);
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
    };
    assert_eq!(res.id, 1);
    assert_eq!(res.path.to_string_lossy(), "bg.png");
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
    assert!((tt.init_bpm - 120.0).abs() < f64::EPSILON);
    assert!(tt.bpm_changes.is_empty());
    assert!(tt.stops.is_empty());
}

#[test]
fn timing_track_new() {
    let tt = TimingTrack::new(120.0, vec![], vec![]);
    assert!((tt.init_bpm - 120.0).abs() < f64::EPSILON);
}

#[test]
fn tick_to_duration_constant_bpm() {
    let tt = TimingTrack::new(120.0, vec![], vec![]);
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
    );
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
    );
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
    let tt = TimingTrack::new(120.0, vec![], vec![]);
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
    );
    // Within stop: 0.5s (to stop) + 0.25s (into stop) → should still be at tick 240
    let tick = tt.duration_to_tick(Duration::from_secs_f64(0.75), 240);
    assert_eq!(tick, 240);
    // After stop: 0.5s (to stop) + 0.5s (stop) + 0.25s (after) → tick 360
    let tick2 = tt.duration_to_tick(Duration::from_secs_f64(1.25), 240);
    assert_eq!(tick2, 360);
}

#[test]
fn duration_to_tick_zero_duration() {
    let tt = TimingTrack::new(120.0, vec![], vec![]);
    let tick = tt.duration_to_tick(Duration::ZERO, 240);
    assert_eq!(tick, 0);
}

#[test]
fn tick_to_duration_zero_tick() {
    let tt = TimingTrack::new(120.0, vec![], vec![]);
    let dur = tt.tick_to_duration(0, 240);
    assert!(dur.is_zero());
}

#[test]
fn timing_track_clone_resets_cache() {
    let tt = TimingTrack::new(120.0, vec![], vec![]);
    let _primed = tt.tick_to_duration(240, 240);
    assert!((tt.init_bpm - 120.0).abs() < f64::EPSILON);
}

#[test]
fn timing_track_partial_eq() {
    let a = TimingTrack::new(120.0, vec![], vec![]);
    let b = TimingTrack::new(120.0, vec![], vec![]);
    let c = TimingTrack::new(140.0, vec![], vec![]);
    assert_eq!(a, b);
    assert_ne!(a, c);
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

// NoCustomEvent

#[test]
fn no_custom_event_default() {
    let nce = NoCustomEvent;
    assert_eq!(nce.tick(), 0);
}
