//! `rosu_map::Beatmap` 到 `Chart` 的转换实现。

use std::num::NonZeroU8;
use std::path::PathBuf;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, BpmChange, Chart, ChartData, ChartInfo, Event, EventKind, Lane, LnJudgeHint,
    LnLifeHint, LnTypeHint, NoteKind, NoteSide, SongInfo, TimingTrack, TimingTrackError,
};
use rosu_map::Beatmap;
use rosu_map::section::general::GameMode;
use rosu_map::section::hit_objects::HitObjectKind;
use rosu_map::section::hit_objects::hit_samples::{
    HitSampleDefaultName, HitSampleInfo, HitSampleInfoName, SampleBank,
};
use rosu_map::section::timing_points::TimingPoint;

use crate::custom_event::{OsuChartMeta, OsuCustomEvent, OsuMode};
use crate::error::OsuProcessError;
use crate::note_ext::{OsuHitSample, OsuHitSampleName, OsuNoteExt, OsuSampleBank};

/// osu!mania 的固定 tick 分辨率（每四分音符的脉冲数）。
const RESOLUTION: u64 = 1000;

/// 浮点形式的 tick 分辨率，避免重复转换。
const RESOLUTION_F64: f64 = 1000.0;

/// osu!mania 允许的最大键数。
const MAX_MANIA_KEYS: u8 = 18;

/// `Chart` 事件类型别名。
type OsuEvent = Event<OsuNoteExt, OsuCustomEvent>;

/// 将 osu! 谱面转换为 `Chart`。
///
/// # Errors
///
/// 非 mania、键数非法、缺少 timing point 或音频、出现 slider/spinner、
/// 时间溢出等情况会返回 [`OsuProcessError`]。
pub fn process(map: &Beatmap) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuProcessError> {
    let mode = convert_mode(map.mode);
    if mode != OsuMode::Mania {
        return Err(OsuProcessError::UnsupportedMode { mode });
    }

    let key_count = validate_key_count(map.circle_size)?;
    let audio_assets = build_audio_assets(map)?;
    let timing = build_timing(map)?;

    let mut events = Vec::with_capacity(
        map.hit_objects.len()
            + map.control_points.timing_points.len()
            + map.control_points.effect_points.len() * 2
            + map.breaks.len()
            + 2,
    );

    events.push(Event::new(0, EventKind::Bgm { audio_index: 0 }));
    events.push(Event::new(
        0,
        EventKind::Custom(OsuCustomEvent::ChartMeta(build_meta(map, key_count))),
    ));

    push_timing_events(map, &timing, &mut events)?;
    push_effect_events(map, &timing, &mut events)?;
    push_break_events(map, &timing, &mut events)?;
    push_notes(map, key_count, &timing, &mut events)?;

    let Timing { track, .. } = timing;
    let mut data = ChartData {
        timing: track,
        judge_multiplier: 1.0,
        life_multiplier: 1.0,
        ln_type_hint: LnTypeHint::default(),
        ln_judge_hint: LnJudgeHint::default(),
        ln_life_hint: LnLifeHint::default(),
        judge_deltas: None,
        life_deltas: None,
        events,
        audio_assets,
    };
    data.sort_events();

    Ok(Chart {
        song: build_song(map),
        chart: build_chart_info(map),
        data,
    })
}

/// 转换期间使用的计时锚点。
struct Timing {
    /// 构造好的计时轨。
    track: TimingTrack,
    /// 毫秒到 tick 的分段锚点，按时间升序排列。
    anchors: Vec<Anchor>,
}

/// 一个计时锚点。
struct Anchor {
    /// 锚点时间（毫秒）。
    time_ms: f64,
    /// 锚点对应的 tick。
    tick: u64,
    /// 锚点处的 BPM。
    bpm: f64,
}

impl Timing {
    /// 将毫秒换算为 tick；非正时间统一映射到 0。
    fn tick_at(&self, time_ms: f64) -> Result<u64, OsuProcessError> {
        if time_ms <= 0.0 {
            return Ok(0);
        }

        let idx = self
            .anchors
            .partition_point(|anchor| anchor.time_ms <= time_ms)
            .saturating_sub(1);
        let Some(anchor) = self.anchors.get(idx) else {
            return Ok(0);
        };

        tick_from_anchor(time_ms, anchor)
    }
}

/// 构建计时轨与毫秒到 tick 的锚点。
fn build_timing(map: &Beatmap) -> Result<Timing, OsuProcessError> {
    let points = &map.control_points.timing_points;
    let first = points.first().ok_or(OsuProcessError::MissingTimingPoint)?;

    let mut initial_bpm = bpm_of(first)?;
    for point in points {
        if point.time <= 0.0 {
            initial_bpm = bpm_of(point)?;
        } else {
            break;
        }
    }

    let mut anchors = vec![Anchor {
        time_ms: 0.0,
        tick: 0,
        bpm: initial_bpm,
    }];
    let mut bpm_changes = Vec::new();

    for point in points {
        if point.time <= 0.0 {
            continue;
        }

        let last = anchors.last().ok_or(OsuProcessError::MissingTimingPoint)?;
        let tick = tick_from_anchor(point.time, last)?;
        let bpm = bpm_of(point)?;
        if !same_bpm(bpm, last.bpm) {
            bpm_changes.push(BpmChange { tick, bpm });
        }
        anchors.push(Anchor {
            time_ms: point.time,
            tick,
            bpm,
        });
    }

    let track = TimingTrack::new(initial_bpm, bpm_changes, Vec::new(), RESOLUTION)?;
    Ok(Timing { track, anchors })
}

/// 返回 timing point 的 BPM。
fn bpm_of(point: &TimingPoint) -> Result<f64, OsuProcessError> {
    let bpm = 60_000.0 / point.beat_len;
    if bpm.is_finite() && bpm > 0.0 {
        Ok(bpm)
    } else {
        Err(OsuProcessError::Timing(TimingTrackError::InvalidBpm {
            bpm,
        }))
    }
}

/// 判断两个 BPM 是否相同。
fn same_bpm(left: f64, right: f64) -> bool {
    (left - right).abs() <= f64::EPSILON
}

/// 从锚点换算毫秒到绝对 tick。
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "tick is checked against u64::MAX before conversion"
)]
fn tick_from_anchor(time_ms: f64, anchor: &Anchor) -> Result<u64, OsuProcessError> {
    let elapsed_ms = time_ms - anchor.time_ms;
    let tick = (elapsed_ms * RESOLUTION_F64 * anchor.bpm / 60_000.0).round();
    let max_tick = u64::MAX as f64;
    if !tick.is_finite() || tick < 0.0 || tick > max_tick {
        return Err(OsuProcessError::TimeOverflow { time: time_ms });
    }

    anchor
        .tick
        .checked_add(tick as u64)
        .ok_or(OsuProcessError::TimeOverflow { time: time_ms })
}

/// 构建音频素材表。
fn build_audio_assets(map: &Beatmap) -> Result<Vec<AudioAsset>, OsuProcessError> {
    if map.audio_file.trim().is_empty() {
        return Err(OsuProcessError::MissingAudioFilename);
    }

    Ok(vec![AudioAsset::from_path_buf(
        PathBuf::from(map.audio_file.as_str()),
        Duration::ZERO,
        None,
    )])
}

/// 构建乐曲级元数据。
fn build_song(map: &Beatmap) -> SongInfo {
    let title = if map.title.is_empty() {
        map.title_unicode.clone()
    } else {
        map.title.clone()
    };
    let artist = if map.artist.is_empty() {
        map.artist_unicode.clone()
    } else {
        map.artist.clone()
    };

    SongInfo {
        title,
        artist,
        genre: String::new(),
        subartists: Vec::new(),
    }
}

/// 构建谱面级元数据。
fn build_chart_info(map: &Beatmap) -> ChartInfo {
    let back_image = if map.background_file.trim().is_empty() {
        None
    } else {
        Some(map.background_file.clone())
    };

    ChartInfo {
        chart_name: map.version.clone(),
        back_image,
        ..ChartInfo::default()
    }
}

/// 构建 osu! 谱面级扩展元数据。
fn build_meta(map: &Beatmap, key_count: u8) -> OsuChartMeta {
    OsuChartMeta {
        format_version: map.format_version,
        mode: convert_mode(map.mode),
        creator: map.creator.clone(),
        source: map.source.clone(),
        tags: map.tags.clone(),
        title_unicode: map.title_unicode.clone(),
        artist_unicode: map.artist_unicode.clone(),
        beatmap_id: map.beatmap_id,
        beatmap_set_id: map.beatmap_set_id,
        audio_lead_in_ms: f64_to_u32_saturating(map.audio_lead_in),
        preview_time_ms: map.preview_time,
        hp_drain_rate_x10: scaled_u16(map.hp_drain_rate, 10.0),
        overall_difficulty_x10: scaled_u16(map.overall_difficulty, 10.0),
        approach_rate_x10: scaled_u16(map.approach_rate, 10.0),
        key_count,
        slider_multiplier_x100: scaled_u32(map.slider_multiplier, 100.0),
        slider_tick_rate_x100: scaled_u32(map.slider_tick_rate, 100.0),
        stack_leniency_x1000: scaled_u16(map.stack_leniency, 1000.0),
    }
}

/// 将 `f32` 按比例放大并四舍五入为 `u16`，负值与非法值归零。
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped to the u16 range"
)]
fn scaled_u16(value: f32, scale: f32) -> u16 {
    let scaled = (value * scale).round();
    if scaled.is_finite() && scaled > 0.0 {
        scaled.min(f32::from(u16::MAX)) as u16
    } else {
        0
    }
}

/// 将 `f64` 按比例放大并四舍五入为 `u32`，负值与非法值归零。
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped to the u32 range"
)]
fn scaled_u32(value: f64, scale: f64) -> u32 {
    let scaled = (value * scale).round();
    if scaled.is_finite() && scaled > 0.0 {
        scaled.min(f64::from(u32::MAX)) as u32
    } else {
        0
    }
}

/// 将非负 `f64` 四舍五入为 `u32`，负值与非法值归零。
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped to the u32 range"
)]
fn f64_to_u32_saturating(value: f64) -> u32 {
    if value.is_finite() && value > 0.0 {
        value.round().min(f64::from(u32::MAX)) as u32
    } else {
        0
    }
}

/// 校验 mania 键数。
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "validated integer in 1..=18 fits in u8"
)]
fn validate_key_count(circle_size: f32) -> Result<u8, OsuProcessError> {
    let rounded = circle_size.round();
    let valid = circle_size.is_finite()
        && (circle_size - rounded).abs() <= f32::EPSILON
        && (1.0..=f32::from(MAX_MANIA_KEYS)).contains(&rounded);

    if valid {
        Ok(rounded as u8)
    } else {
        Err(OsuProcessError::InvalidKeyCount { circle_size })
    }
}

/// 根据 `x` 坐标计算 mania 列号。
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "scaled position is clamped to the valid column range"
)]
fn column_for_x(x: f32, key_count: u8) -> u8 {
    let scaled = x * f32::from(key_count) / 512.0;
    if scaled.is_finite() {
        scaled
            .floor()
            .clamp(0.0, f32::from(key_count.saturating_sub(1))) as u8
    } else {
        0
    }
}

/// 将 `rosu_map` 的 sample 转换为公开类型。
fn convert_hit_sample(sample: &HitSampleInfo) -> OsuHitSample {
    OsuHitSample {
        name: convert_hit_sample_name(&sample.name),
        bank: convert_sample_bank(sample.bank),
        suffix: sample.suffix,
        volume: u8::try_from(sample.volume.clamp(0, 100)).unwrap_or(0),
        custom_sample_bank: u16::try_from(sample.custom_sample_bank.max(0)).unwrap_or(u16::MAX),
        bank_specified: sample.bank_specified,
        is_layered: sample.is_layered,
    }
}

/// 转换 sample 名称。
fn convert_hit_sample_name(name: &HitSampleInfoName) -> OsuHitSampleName {
    match name {
        HitSampleInfoName::Default(HitSampleDefaultName::Normal) => OsuHitSampleName::Normal,
        HitSampleInfoName::Default(HitSampleDefaultName::Whistle) => OsuHitSampleName::Whistle,
        HitSampleInfoName::Default(HitSampleDefaultName::Finish) => OsuHitSampleName::Finish,
        HitSampleInfoName::Default(HitSampleDefaultName::Clap) => OsuHitSampleName::Clap,
        HitSampleInfoName::File(filename) => OsuHitSampleName::File(filename.clone()),
    }
}

/// 转换 sample bank。
const fn convert_sample_bank(bank: SampleBank) -> OsuSampleBank {
    match bank {
        SampleBank::None => OsuSampleBank::None,
        SampleBank::Normal => OsuSampleBank::Normal,
        SampleBank::Soft => OsuSampleBank::Soft,
        SampleBank::Drum => OsuSampleBank::Drum,
    }
}

/// 转换游戏模式。
const fn convert_mode(mode: GameMode) -> OsuMode {
    match mode {
        GameMode::Osu => OsuMode::Osu,
        GameMode::Taiko => OsuMode::Taiko,
        GameMode::Catch => OsuMode::Catch,
        GameMode::Mania => OsuMode::Mania,
    }
}

/// 压入非继承 timing point 的拍号信息与 BPM 变更。
fn push_timing_events(
    map: &Beatmap,
    timing: &Timing,
    events: &mut Vec<OsuEvent>,
) -> Result<(), OsuProcessError> {
    for point in &map.control_points.timing_points {
        let tick = timing.tick_at(point.time.max(0.0))?;
        events.push(Event::new(
            tick,
            EventKind::Custom(OsuCustomEvent::Timing {
                time_signature: point.time_signature.numerator.get(),
                omit_first_bar_line: point.omit_first_bar_line,
            }),
        ));
    }

    for change in timing.track.bpm_changes() {
        events.push(Event::bpm(change.tick, change.bpm));
    }

    Ok(())
}

/// 压入继承 timing point 的滚动速度与 Kiai 状态。
fn push_effect_events(
    map: &Beatmap,
    timing: &Timing,
    events: &mut Vec<OsuEvent>,
) -> Result<(), OsuProcessError> {
    for point in &map.control_points.effect_points {
        let tick = timing.tick_at(point.time.max(0.0))?;
        events.push(Event::scroll(tick, point.scroll_speed));
        events.push(Event::new(
            tick,
            EventKind::Custom(OsuCustomEvent::Kiai(point.kiai)),
        ));
    }

    Ok(())
}

/// 压入 Break 区间。
fn push_break_events(
    map: &Beatmap,
    timing: &Timing,
    events: &mut Vec<OsuEvent>,
) -> Result<(), OsuProcessError> {
    for break_period in &map.breaks {
        let start_tick = timing.tick_at(break_period.start_time.max(0.0))?;
        let end_tick = timing.tick_at(break_period.end_time.max(0.0))?;
        if end_tick > start_tick {
            events.push(Event::new(
                start_tick,
                EventKind::Custom(OsuCustomEvent::Break { end_tick }),
            ));
        }
    }

    Ok(())
}

/// 压入 mania 音符。
fn push_notes(
    map: &Beatmap,
    key_count: u8,
    timing: &Timing,
    events: &mut Vec<OsuEvent>,
) -> Result<(), OsuProcessError> {
    for (index, object) in map.hit_objects.iter().enumerate() {
        let start_time = object.start_time;
        if start_time < 0.0 {
            return Err(OsuProcessError::NegativeHitObjectTime {
                index,
                time: start_time,
            });
        }

        let start_tick = timing.tick_at(start_time)?;
        let (kind, x) = match &object.kind {
            HitObjectKind::Circle(circle) => (NoteKind::Normal, circle.pos.x),
            HitObjectKind::Hold(hold) => {
                let end_time = start_time + hold.duration;
                if !end_time.is_finite() || end_time <= start_time {
                    return Err(OsuProcessError::InvalidHold {
                        index,
                        start: start_time,
                        end: end_time,
                    });
                }

                let end_tick = timing.tick_at(end_time)?;
                if end_tick <= start_tick {
                    return Err(OsuProcessError::InvalidHold {
                        index,
                        start: start_time,
                        end: end_time,
                    });
                }

                (
                    NoteKind::Long {
                        duration: end_tick - start_tick,
                    },
                    hold.pos_x,
                )
            }
            HitObjectKind::Slider(_) => {
                return Err(OsuProcessError::UnsupportedObject {
                    index,
                    kind: "slider",
                });
            }
            HitObjectKind::Spinner(_) => {
                return Err(OsuProcessError::UnsupportedObject {
                    index,
                    kind: "spinner",
                });
            }
        };

        let column = column_for_x(x, key_count);
        let lane = Lane::Key(NonZeroU8::new(column + 1).unwrap_or(NonZeroU8::MIN));
        let hit_samples = object.samples.iter().map(convert_hit_sample).collect();

        events.push(Event::new(
            start_tick,
            EventKind::Note {
                side: NoteSide::P1,
                lane,
                kind,
                audio_index: None,
                ext: OsuNoteExt { hit_samples },
            },
        ));
    }

    Ok(())
}
