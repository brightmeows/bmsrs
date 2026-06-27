//! BMSON → `Chart` 转换处理器。
//!
//! [`BmsonProcessor`] 将一个 `bmson_def::Bmson`（v2 根 schema）转换为
//! 格式无关的 `Chart<BmsonNoteExt>`，转换时套用一个 [`BmsonLayout`] 模式族
//! （[`Beat`]、[`Pms`]，或用于 n-keys 的 [`GenericLayout`]）。
//!
//! 模式族与 [`BmsonLayout`] trait 位于 [`layout`] 模块中。
//!
//! # 管道
//!
//! ```text
//! bmson_def::Bmson → BmsonProcessor::process(bmson, layout) → Chart<BmsonNoteExt>
//! ```
//!
//! 对于 v0/v1 文件，请先通过 `Bmson::from` 转换为根 schema。
//!
//! # 切片
//!
//! 每个 `bmson_def::SoundChannel` 会在每个唯一的音符脉冲处切片为预计算的
//! `AudioAsset`（详见内部 `slice` 模块）。

mod slice;

pub mod layout;

use std::collections::BTreeSet;
use std::time::Duration;

use bmson_def::{BpmEvent, StopEvent as BmsonStopEvent};
use bmsrs_chart::{
    AudioAsset, BgaLayer, BgaResource, BpmChange, Chart, ChartData, ChartInfo, Event, Lane,
    NoteExt, NoteKind, NoteSide, SongInfo, StopEvent, TimingTrack,
};
use thiserror::Error;

use crate::layout::{Beat, BmsonLayout, GenericLayout, Pms};

use crate::slice::slice_channel;

/// BMSON 格式的每音符扩展数据。
///
/// 携带来自 `bmson_def::NoteEvent` 的可选字段，这些字段不属于核心谱面模型。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BmsonNoteExt {
    /// 音符音量（百分比，DJ.NEXT 扩展）。
    pub vol: Option<i8>,
    /// 音符声像（pan，DJ.NEXT 扩展）。
    pub pan: Option<i8>,
    /// 释放音 / BSS 标志（bmson `up`）。
    pub release_sound: Option<bool>,
    /// beatoraja 长音模式（bmson `t`，1=LN/2=CN/3=HCN）。
    pub beatoraja_ln_mode: Option<u64>,
    /// 每音符的 LN 类型提示覆盖（bmson v2）。
    pub ln_type_hint: Option<String>,
    /// 每音符的 LN 判定提示覆盖（bmson v2）。
    pub ln_judge_hint: Option<String>,
    /// 每音符的 LN 血量提示覆盖（bmson v2）。
    pub ln_life_hint: Option<String>,
}

impl NoteExt for BmsonNoteExt {}

/// BMSON 处理过程中可能出现的错误。
#[derive(Debug, Error)]
pub enum ProcessError {
    /// `init_bpm` 必须严格为正。
    #[error("init_bpm must be positive, got {0}")]
    InvalidBpm(f64),
}

/// 将 [`bmson_def::Bmson`] 转换为 [`Chart<BmsonNoteExt>`] 的零大小处理器。
///
/// 调用 [`process`](Self::process) 时传入一个模式族布局，或调用
/// [`process_default`](Self::process_default) 从 `mode_hint` 中自动选择。
pub struct BmsonProcessor;

impl BmsonProcessor {
    /// 使用无状态模式族布局处理 BMSON 谱面。
    ///
    /// # Errors
    ///
    /// 若 `init_bpm` 不为正，返回 [`ProcessError::InvalidBpm`]。
    pub fn process<L>(bmson: &bmson_def::Bmson<'_>) -> Result<Chart<BmsonNoteExt>, ProcessError>
    where
        L: BmsonLayout,
    {
        Self::process_body(bmson, &|x| L::map_x(x))
    }

    /// 使用 generic-nkeys 布局处理 BMSON 谱面。
    ///
    /// 这是唯一有状态的布局族——当模式提示为 `generic-nkeys` 时，请直接调用
    /// 本方法，而不是 [`process`](Self::process)。
    ///
    /// # Errors
    ///
    /// 若 `init_bpm` 不为正，返回 [`ProcessError::InvalidBpm`]。
    pub fn process_nkeys(
        bmson: &bmson_def::Bmson<'_>,
        keys: u16,
    ) -> Result<Chart<BmsonNoteExt>, ProcessError> {
        let layout = GenericLayout { keys };
        Self::process_body(bmson, &|x| layout.map_x(x))
    }

    /// 处理 BMSON 谱面，根据 `mode_hint` 选择模式族。
    ///
    /// `beat-*` 与 `dj-*` 提示映射到 [`Beat`]；`popn-*` 映射到 [`Pms`]；
    /// `generic-nkeys` 映射到 [`GenericLayout`]（使用给定的按键数）；其它情况
    /// 回退到 [`Beat`]（BMSON 默认值）。
    ///
    /// # Errors
    ///
    /// 若 `init_bpm` 不为正，返回 [`ProcessError::InvalidBpm`]。
    pub fn process_default(
        bmson: &bmson_def::Bmson<'_>,
    ) -> Result<Chart<BmsonNoteExt>, ProcessError> {
        match bmson.chart_data.mode_hint {
            bmson_def::ModeHint::Popn5k | bmson_def::ModeHint::Popn9k => {
                Self::process::<Pms>(bmson)
            }
            bmson_def::ModeHint::Generic(n) => {
                let keys = u16::try_from(n).unwrap_or(0);
                Self::process_nkeys(bmson, keys)
            }
            _ => Self::process::<Beat>(bmson),
        }
    }

    /// 自定义解码逻辑的公开入口（内部使用）。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "BGA header/event ids are in the u32 range for practical charts"
    )]
    fn process_body(
        bmson: &bmson_def::Bmson<'_>,
        decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    ) -> Result<Chart<BmsonNoteExt>, ProcessError> {
        let data = &bmson.chart_data;

        if data.init_bpm <= 0.0 {
            return Err(ProcessError::InvalidBpm(data.init_bpm));
        }

        let timing = build_timing(data);
        let resolution = data.resolution;
        let playable_pulses = collect_playable_pulses(&data.sound_channels);

        let (mut audio_assets, mut events) = process_sound_channels(
            &data.sound_channels,
            decode,
            &timing,
            resolution,
            &playable_pulses,
        );

        process_mine_channels(&bmson.mine_channels, decode, &mut audio_assets, &mut events);
        process_key_channels(&bmson.key_channels, decode, &mut audio_assets, &mut events);

        events.extend(data.bpm_events.iter().map(|e| Event::Bpm {
            tick: e.y,
            bpm: e.bpm,
        }));
        events.extend(data.stop_events.iter().map(|e| Event::Stop {
            tick: e.y,
            duration: e.duration,
        }));
        events.extend(bmson.scroll_events.iter().map(|e| Event::Scroll {
            tick: e.y,
            rate: e.rate,
        }));

        events.extend(build_bar_lines(data.lines.as_deref(), resolution, &events));

        let bga = &bmson.chart_info.bga;

        for e in &bga.bga_events {
            events.push(Event::Bga {
                tick: e.y,
                layer: BgaLayer::Base,
                resource_id: e.id as u32,
            });
        }
        for e in &bga.layer_events {
            events.push(Event::Bga {
                tick: e.y,
                layer: BgaLayer::Layer,
                resource_id: e.id as u32,
            });
        }
        for e in &bga.poor_events {
            events.push(Event::Bga {
                tick: e.y,
                layer: BgaLayer::Poor,
                resource_id: e.id as u32,
            });
        }

        events.sort_by_key(Event::tick);

        let song_info = build_song_info(bmson);
        let chart_info = build_chart_info(bmson);

        Ok(Chart {
            song: song_info,
            chart: chart_info,
            data: ChartData {
                resolution,
                timing,
                judge_multiplier: data.judge_multiplier,
                life_multiplier: data.life_multiplier,
                events,
                audio_assets,
            },
        })
    }
}

/// 收集所有至少包含一个可演奏音符的脉冲位置。
fn collect_playable_pulses(channels: &[bmson_def::SoundChannel<'_>]) -> BTreeSet<u64> {
    channels
        .iter()
        .flat_map(|ch| ch.note_events.iter())
        .filter(|n| n.x > 0)
        .map(|n| n.y)
        .collect()
}

/// 从 BMSON 谱面数据构建 [`TimingTrack`]。
fn build_timing(data: &bmson_def::ChartData<'_>) -> TimingTrack {
    TimingTrack::new(
        data.init_bpm,
        data.bpm_events.iter().map(build_bpm_change).collect(),
        data.stop_events.iter().map(build_stop_event).collect(),
    )
}

/// 处理音频通道：切片、创建音符/BGM 事件。
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_sound_channels(
    channels: &[bmson_def::SoundChannel<'_>],
    decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    timing: &TimingTrack,
    resolution: u64,
    playable_pulses: &BTreeSet<u64>,
) -> (Vec<AudioAsset>, Vec<Event<BmsonNoteExt>>) {
    let mut audio_assets = Vec::new();
    let mut events = Vec::new();

    for channel in channels {
        let sliced = slice_channel(channel, timing, resolution);

        for ne in &channel.note_events {
            let audio_idx = sliced
                .pulse_to_index
                .get(&ne.y)
                .copied()
                .map(|idx| (idx + audio_assets.len()) as u32);

            if ne.is_bgm() {
                if playable_pulses.contains(&ne.y) {
                    continue;
                }
                if let Some(idx) = audio_idx {
                    events.push(Event::Bgm {
                        tick: ne.y,
                        audio_index: idx,
                    });
                }
            } else {
                let Some((side, lane)) = decode(ne.x) else {
                    continue;
                };
                let kind = if ne.l > 0 {
                    NoteKind::Long { duration: ne.l }
                } else {
                    NoteKind::Normal
                };
                events.push(Event::Note {
                    tick: ne.y,
                    side,
                    lane,
                    kind,
                    audio_index: audio_idx,
                    ext: build_note_ext(ne),
                });
            }
        }

        audio_assets.extend(sliced.assets);
    }

    (audio_assets, events)
}

/// 从 BMSON [`NoteEvent`](bmson_def::NoteEvent) 构建 [`BmsonNoteExt`]。
fn build_note_ext(ne: &bmson_def::NoteEvent) -> BmsonNoteExt {
    BmsonNoteExt {
        vol: ne.vol,
        pan: ne.pan,
        release_sound: ne.up,
        beatoraja_ln_mode: ne.t.map(ln_mode_to_u64),
        ln_type_hint: ne.ln_type_hint.map(ln_type_to_str),
        ln_judge_hint: ne.ln_judge_hint.map(ln_judge_to_str),
        ln_life_hint: ne.ln_life_hint.map(ln_life_to_str),
    }
}

/// 将 `LnMode` 判别值转换为 beatoraja 的数值型 LN 模式值。
const fn ln_mode_to_u64(m: bmson_def::LnMode) -> u64 {
    match m {
        bmson_def::LnMode::Cn => 2,
        bmson_def::LnMode::Hcn => 3,
        _ => 1,
    }
}

/// 将 `LnType` 转换为其 bmson v2 字符串表示。
fn ln_type_to_str(lt: bmson_def::LnType) -> String {
    match lt {
        bmson_def::LnType::Cn => "cn",
        _ => "ln",
    }
    .to_owned()
}

/// 将 `LnJudge` 转换为其 bmson v2 字符串表示。
fn ln_judge_to_str(lj: bmson_def::LnJudge) -> String {
    match lj {
        bmson_def::LnJudge::Ticks => "ticks",
        _ => "normal",
    }
    .to_owned()
}

/// 将 `LnLife` 转换为其 bmson v2 字符串表示。
fn ln_life_to_str(ll: bmson_def::LnLife) -> String {
    match ll {
        bmson_def::LnLife::Ticks => "ticks",
        _ => "normal",
    }
    .to_owned()
}

/// 处理地雷通道：每个通道贡献一个整文件 `AudioAsset`。
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_mine_channels(
    channels: &[bmson_def::MineChannel<'_>],
    decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    audio_assets: &mut Vec<AudioAsset>,
    events: &mut Vec<Event<BmsonNoteExt>>,
) {
    for mc in channels {
        let mine_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: mc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for mn in &mc.notes {
            let Some((side, lane)) = decode(mn.x) else {
                continue;
            };
            events.push(Event::Note {
                tick: mn.y,
                side,
                lane,
                kind: NoteKind::Mine { damage: mn.damage },
                audio_index: Some(mine_audio_idx),
                ext: BmsonNoteExt::default(),
            });
        }
    }
}

/// 处理按键（不可见）通道：结构与地雷通道相同。
#[expect(
    clippy::cast_possible_truncation,
    reason = "audio asset count fits in u32 for practical charts"
)]
fn process_key_channels(
    channels: &[bmson_def::KeyChannel<'_>],
    decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    audio_assets: &mut Vec<AudioAsset>,
    events: &mut Vec<Event<BmsonNoteExt>>,
) {
    for kc in channels {
        let key_audio_idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: kc.name.to_path_buf(),
            start: Duration::ZERO,
            duration: None,
        });

        for kn in &kc.notes {
            let Some((side, lane)) = decode(kn.x) else {
                continue;
            };
            events.push(Event::Note {
                tick: kn.y,
                side,
                lane,
                kind: NoteKind::Invisible,
                audio_index: Some(key_audio_idx),
                ext: BmsonNoteExt::default(),
            });
        }
    }
}

/// 从 BMSON 乐曲信息构建 [`SongInfo`]。
fn build_song_info(bmson: &bmson_def::Bmson<'_>) -> SongInfo {
    SongInfo {
        title: bmson.song_info.title.to_owned(),
        artist: bmson.song_info.artist.to_owned(),
        genre: bmson.song_info.genre.to_owned(),
        subartists: bmson
            .chart_info
            .subartists
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
    }
}

/// 从 BMSON 谱面信息构建 [`ChartInfo`]。
fn build_chart_info(bmson: &bmson_def::Bmson<'_>) -> ChartInfo {
    let bga = &bmson.chart_info.bga;
    let bga_resources: Vec<BgaResource> = bga
        .bga_header
        .iter()
        .map(|h| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "BGA header count fits in u32"
            )]
            BgaResource {
                id: h.id as u32,
                path: h.name.to_path_buf(),
            }
        })
        .collect();

    ChartInfo {
        subtitle: bmson.chart_info.subtitle.to_owned(),
        chart_name: bmson.chart_info.chart_name.to_owned(),
        level: bmson.chart_info.level,
        back_image: bmson
            .chart_info
            .back_image
            .map(|p| p.to_string_lossy().into_owned()),
        eyecatch_image: bmson
            .chart_info
            .eyecatch_image
            .map(|p| p.to_string_lossy().into_owned()),
        banner_image: bmson
            .chart_info
            .banner_image
            .map(|p| p.to_string_lossy().into_owned()),
        preview_music: bmson
            .chart_info
            .preview_music
            .map(|p| p.to_string_lossy().into_owned()),
        bga_resources,
    }
}

/// 从 BMSON [`BpmEvent`] 构建 [`BpmChange`]。
const fn build_bpm_change(e: &BpmEvent) -> BpmChange {
    BpmChange {
        tick: e.y,
        bpm: e.bpm,
    }
}

/// 从 BMSON 停止事件构建 [`StopEvent`]。
const fn build_stop_event(e: &BmsonStopEvent) -> StopEvent {
    StopEvent {
        tick: e.y,
        duration: e.duration,
    }
}

/// 从 BMSON `lines` 字段构建小节线事件。
///
/// `None` → 自动生成 4/4 拍小节线（每隔 `resolution * 4` 个脉冲一条），
/// 范围从 0 到最后一个事件的脉冲位置。
fn build_bar_lines(
    lines: Option<&[bmson_def::BarLine]>,
    resolution: u64,
    events: &[Event<BmsonNoteExt>],
) -> Vec<Event<BmsonNoteExt>> {
    if let Some(vec) = lines {
        return vec.iter().map(|bl| Event::Bar { tick: bl.y }).collect();
    }

    let last_tick = events.last().map_or(0, Event::tick);
    let step = resolution * 4;
    let count = last_tick / step + 1;
    (0..=count).map(|i| Event::Bar { tick: i * step }).collect()
}
