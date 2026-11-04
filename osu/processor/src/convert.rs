//! Conversion implementation from `rosu_map::Beatmap` to `Chart`.

use std::num::NonZeroU8;
use std::path::PathBuf;
use std::time::Duration;

use bmsrs_chart::{
    AudioAsset, AudioIndex, BpmChange, Chart, ChartData, ChartInfo, Event, EventKind, Lane, Note,
    NoteAudio, NoteKind, NoteRole, NoteSide, SongInfo, TimingTrackError,
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

/// The fixed osu!mania tick resolution (pulses per quarter note).
const RESOLUTION: u64 = 1000;

/// The tick resolution as `f64`, avoiding repeated conversions.
const RESOLUTION_F64: f64 = 1000.0;

/// The maximum number of keys allowed in osu!mania.
const MAX_MANIA_KEYS: u8 = 18;

/// The `Chart` event type alias.
type OsuEvent = Event<OsuNoteExt, OsuCustomEvent>;

/// Convert an osu! beatmap into a `Chart`.
///
/// # Errors
///
/// Returns [`OsuProcessError`] for non-mania modes, invalid key counts,
/// missing timing points or audio, sliders/spinners, non-finite hold
/// ends, and time overflow.
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

    events.push(Event::bgm(0, AudioIndex::new(0)));
    events.push(Event::new(
        0,
        EventKind::Custom(OsuCustomEvent::ChartMeta(build_meta(map, key_count))),
    ));

    push_timing_events(map, &timing, &mut events)?;
    push_effect_events(map, &timing, &mut events)?;
    push_break_events(map, &timing, &mut events)?;
    push_notes(map, key_count, &timing, &mut events)?;

    let data = ChartData::new(timing.initial_bpm, RESOLUTION, events, audio_assets)?;
    Chart::new(build_song(map), build_chart_info(map, key_count), data).map_err(Into::into)
}

/// Timing anchors used during conversion.
struct Timing {
    /// The BPM in effect at tick 0.
    initial_bpm: f64,
    /// BPM changes in tick space, in ascending tick order.
    bpm_changes: Vec<BpmChange>,
    /// Millisecond-to-tick anchors, ascending by time.
    anchors: Vec<Anchor>,
}

/// A single timing anchor.
struct Anchor {
    /// The anchor time in milliseconds.
    time_ms: f64,
    /// The tick at the anchor.
    tick: u64,
    /// The BPM at the anchor.
    bpm: f64,
}

impl Timing {
    /// Convert milliseconds to ticks; non-positive times map to tick 0.
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

/// Build the timing anchors and BPM changes.
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

    Ok(Timing {
        initial_bpm,
        bpm_changes,
        anchors,
    })
}

/// Return the BPM of a timing point.
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

/// Decide whether two BPM values are the same.
fn same_bpm(left: f64, right: f64) -> bool {
    (left - right).abs() <= f64::EPSILON
}

/// Convert milliseconds to an absolute tick from an anchor.
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

/// Build the audio asset table.
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

/// Build the song-level metadata.
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
    }
}

/// Build the chart-level metadata.
fn build_chart_info(map: &Beatmap, key_count: u8) -> ChartInfo {
    let back_image = if map.background_file.trim().is_empty() {
        None
    } else {
        Some(map.background_file.clone())
    };

    ChartInfo {
        chart_name: map.version.clone(),
        back_image,
        key_count: Some(key_count),
        ..ChartInfo::default()
    }
}

/// Build the osu! chart-level extension metadata.
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

/// Scale an `f32` and round it into a `u16`; invalid and negative values become 0.
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

/// Scale an `f64` and round it into a `u32`; invalid and negative values become 0.
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

/// Round a non-negative `f64` into a `u32`; invalid and negative values become 0.
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

/// Validate the mania key count.
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

/// Compute the mania column from an `x` coordinate.
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

/// Convert a `rosu_map` sample into the public type.
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

/// Convert a sample name.
fn convert_hit_sample_name(name: &HitSampleInfoName) -> OsuHitSampleName {
    match name {
        HitSampleInfoName::Default(HitSampleDefaultName::Normal) => OsuHitSampleName::Normal,
        HitSampleInfoName::Default(HitSampleDefaultName::Whistle) => OsuHitSampleName::Whistle,
        HitSampleInfoName::Default(HitSampleDefaultName::Finish) => OsuHitSampleName::Finish,
        HitSampleInfoName::Default(HitSampleDefaultName::Clap) => OsuHitSampleName::Clap,
        HitSampleInfoName::File(filename) => OsuHitSampleName::File(filename.clone()),
    }
}

/// Convert a sample bank.
const fn convert_sample_bank(bank: SampleBank) -> OsuSampleBank {
    match bank {
        SampleBank::None => OsuSampleBank::None,
        SampleBank::Normal => OsuSampleBank::Normal,
        SampleBank::Soft => OsuSampleBank::Soft,
        SampleBank::Drum => OsuSampleBank::Drum,
    }
}

/// Convert a game mode.
const fn convert_mode(mode: GameMode) -> OsuMode {
    match mode {
        GameMode::Osu => OsuMode::Osu,
        GameMode::Taiko => OsuMode::Taiko,
        GameMode::Catch => OsuMode::Catch,
        GameMode::Mania => OsuMode::Mania,
    }
}

/// Push time signature information and BPM changes from uninherited timing points.
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

    for change in &timing.bpm_changes {
        events.push(Event::bpm(change.tick, change.bpm));
    }

    Ok(())
}

/// Push scroll speeds and Kiai states from inherited timing points.
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

/// Push break intervals.
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

/// Push the mania notes.
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
                if !end_time.is_finite() {
                    return Err(OsuProcessError::NonFiniteHold {
                        index,
                        start: start_time,
                        end: end_time,
                    });
                }

                let end_tick = timing.tick_at(end_time)?;
                // Holds ending at or before their start mirror the osu! lazer
                // parser, which clamps them to zero length and plays them as
                // rice; `Normal` carries the same semantics.
                if end_tick > start_tick {
                    (
                        NoteKind::Long {
                            duration: end_tick - start_tick,
                        },
                        hold.pos_x,
                    )
                } else {
                    (NoteKind::Normal, hold.pos_x)
                }
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

        let note = Note::new(
            NoteSide::P1,
            lane,
            kind,
            NoteRole::Normal,
            NoteAudio::none(),
        )
        .with_ext(OsuNoteExt { hit_samples });
        events.push(Event::note(start_tick, note));
    }

    Ok(())
}
