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

mod error;
mod slice;

pub mod layout;

use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bmson_def::{BpmEvent, StopEvent as BmsonStopEvent};
use bmsrs_chart::{
    AudioAsset, AudioIndex, BgaLayer, BgaResource, BgaResourceId, BpmChange, Chart, ChartData,
    ChartInfo, Damage, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint, NoteAudio,
    NoteExt, NoteKind, NoteRole, NoteSide, SongInfo, StopEvent, TimingTrack, TimingTrackError,
};
use thiserror::Error;

pub use crate::error::BmsonProcessWarning;

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
    /// 每音符的 LN 类型提示；beatoraja `t` 与 bmson v2
    /// `ln_type_hint` 都归并到这里，`t` 优先。
    pub ln_type_hint: Option<LnTypeHint>,
    /// 每音符的 LN 判定提示覆盖（bmson v2）。
    pub ln_judge_hint: Option<LnJudgeHint>,
    /// 每音符的 LN 血量提示覆盖（bmson v2）。
    pub ln_life_hint: Option<LnLifeHint>,
}

impl NoteExt for BmsonNoteExt {}

/// BMSON 处理过程中可能出现的错误。
#[derive(Debug, Error)]
pub enum ProcessError {
    /// 计时轨构造失败（初始 BPM 无效或 resolution 为零）。
    ///
    /// 透传 [`TimingTrackError`]，不再在边界上降维翻译——此前
    /// `ZeroResolution` 被译成 `InvalidBpm(0.0)`，报误导性错误。
    #[error(transparent)]
    Timing(#[from] TimingTrackError),
    /// `generic-nkeys` 模式的按键数必须为正且在 `u8` 范围内
    /// （[`Lane::Key`] 的索引上限）。
    #[error("generic-nkeys key count must be in 1..=255, got {0}")]
    InvalidKeyCount(u64),
    /// 谱面引用了不安全的资源路径。
    #[error("unsafe resource path \"{path}\": {reason}")]
    UnsafePath {
        /// 原始路径。
        path: String,
        /// 拒绝原因。
        reason: &'static str,
    },
}

/// 将 [`bmson_def::Bmson`] 转换为 [`Chart<BmsonNoteExt>`] 的零大小处理器。
///
/// 调用 [`process`](Self::process) 时传入一个模式族布局，或调用
/// [`process_default`](Self::process_default) 从 `mode_hint` 中自动选择。
pub struct BmsonProcessor;

/// BMSON 处理结果。
#[derive(Debug)]
pub struct Processed {
    /// 处理得到的谱面。
    pub chart: Chart<BmsonNoteExt>,
    /// 处理过程中收集的可恢复警告。
    pub warnings: Vec<BmsonProcessWarning>,
}

impl BmsonProcessor {
    /// 使用无状态模式族布局处理 BMSON 谱面，并返回处理警告。
    ///
    /// # Errors
    ///
    /// 若计时轨构造失败（初始 BPM 无效或 resolution 为零），返回
    /// [`ProcessError::Timing`]。
    pub fn process<L>(bmson: &bmson_def::Bmson<'_>) -> Result<Processed, ProcessError>
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
    /// 若计时轨构造失败（初始 BPM 无效或 resolution 为零），返回
    /// [`ProcessError::Timing`]。
    pub fn process_nkeys(
        bmson: &bmson_def::Bmson<'_>,
        keys: u8,
    ) -> Result<Processed, ProcessError> {
        let layout = GenericLayout { keys };
        Self::process_body(bmson, &|x| layout.map_x(x))
    }

    /// 处理 BMSON 谱面，根据 `mode_hint` 选择模式族。
    ///
    /// `beat-*` 与 `dj-*` 提示映射到 [`Beat`]；`popn-*` 映射到 [`Pms`]；
    /// `generic-nkeys` 映射到 [`GenericLayout`]（使用给定的按键数）；
    /// 规范外的扩展提示（[`ModeHint::Other`](bmson_def::ModeHint::Other)）
    /// 回退到 [`Beat`]（BMSON 默认值），回退警告记录在
    /// [`Processed::warnings`] 中。
    ///
    /// # Errors
    ///
    /// - 若计时轨构造失败（初始 BPM 无效或 resolution 为零），返回
    ///   [`ProcessError::Timing`]。
    /// - 若 `mode_hint` 为 `generic-nkeys` 但按键数为 0 或超过 255
    ///   （[`Lane::Key`] 的索引上限），返回
    ///   [`ProcessError::InvalidKeyCount`]（否则所有音符因空键位映射
    ///   被静默丢弃）。
    pub fn process_default(bmson: &bmson_def::Bmson<'_>) -> Result<Processed, ProcessError> {
        match &bmson.chart_data.mode_hint {
            bmson_def::ModeHint::Popn5k | bmson_def::ModeHint::Popn9k => {
                Self::process::<Pms>(bmson)
            }
            bmson_def::ModeHint::Generic(n) => {
                let keys = u8::try_from(*n).map_err(|_e| ProcessError::InvalidKeyCount(*n))?;
                if keys == 0 {
                    return Err(ProcessError::InvalidKeyCount(*n));
                }
                Self::process_nkeys(bmson, keys)
            }
            bmson_def::ModeHint::Other(hint) => {
                let mut processed = Self::process::<Beat>(bmson)?;
                processed
                    .warnings
                    .push(BmsonProcessWarning::UnknownModeHint { hint: hint.clone() });
                Ok(processed)
            }
            _ => Self::process::<Beat>(bmson),
        }
    }

    /// 自定义解码逻辑的公开入口（内部使用）。
    fn process_body(
        bmson: &bmson_def::Bmson<'_>,
        decode: &impl Fn(u64) -> Option<(NoteSide, Lane)>,
    ) -> Result<Processed, ProcessError> {
        let bmson_data = &bmson.chart_data;

        let timing = build_timing(bmson_data)?;
        let bga_ids = build_bga_id_map(&bmson.chart_info.bga);
        let mut conv = BmsonConverter {
            bmson,
            decode,
            timing: timing.clone(),
            bga_ids: &bga_ids,
            audio_assets: Vec::new(),
            events: Vec::new(),
            warnings: Vec::new(),
        };

        conv.process_sound_channels()?;
        conv.process_mine_channels()?;
        conv.process_key_channels()?;
        conv.collect_bpm_events();
        conv.collect_stop_events();
        conv.collect_scroll_events();
        conv.collect_bga_events();
        conv.build_bar_lines();

        let song_info = build_song_info(bmson);
        let chart_info = build_chart_info(bmson)?;

        let mut data = ChartData {
            timing,
            judge_multiplier: Some(bmson_data.judge_multiplier),
            life_multiplier: Some(bmson_data.life_multiplier),
            ln_type_hint: ln_type_to_hint(bmson_data.ln_type_hint),
            ln_judge_hint: ln_judge_to_hint(bmson_data.ln_judge_hint),
            ln_life_hint: ln_life_to_hint(bmson_data.ln_life_hint),
            judge_deltas: bmson_data
                .judge_deltas
                .as_ref()
                .map(|d| bmsrs_chart::JudgementDeltas {
                    perfect: d.perfect,
                    great: d.great,
                    good: d.good,
                    miss: d.miss,
                }),
            life_deltas: bmson_data
                .life_deltas
                .as_ref()
                .map(|d| bmsrs_chart::LifeDeltas {
                    perfect: d.perfect,
                    great: d.great,
                    good: d.good,
                    miss: d.miss,
                }),
            events: conv.events,
            audio_assets: conv.audio_assets,
        };
        data.sort_events();

        Ok(Processed {
            chart: Chart {
                song: song_info,
                chart: chart_info,
                data,
            },
            warnings: conv.warnings,
        })
    }
}

/// BMSON → [`Chart`] 转换上下文。
///
/// 持有源文档 [`bmson_def::Bmson`]、解码函数与预计算的计时缓存，
/// 同时拥有 `audio_assets` 和 `events` 两个累加缓冲区，使各转换步骤
/// 以 `&mut self` 方法组织而非自由函数 + `&mut` 参数。仅供
/// [`BmsonProcessor::process_body`] 内部使用。
struct BmsonConverter<'a> {
    /// 源 BMSON 文档。
    bmson: &'a bmson_def::Bmson<'a>,
    /// 通道解码函数（`x` → `(side, lane)`）。
    decode: &'a dyn Fn(u64) -> Option<(NoteSide, Lane)>,
    /// 预计算的计时缓存。
    timing: TimingTrack,
    /// BMSON BGA 原始 id → 稠密资源下标映射。
    bga_ids: &'a BTreeMap<u64, BgaResourceId>,
    /// 正在构建的音频素材向量。
    audio_assets: Vec<AudioAsset>,
    /// 正在构建的事件向量。
    events: Vec<Event<BmsonNoteExt>>,
    /// 处理过程中收集的可恢复警告。
    warnings: Vec<BmsonProcessWarning>,
}

/// 同一 `(side, lane, tick)` 上等待融合的音符。
struct LayeredNote {
    /// 音符位置。
    lane: Lane,
    /// 音符种类；分层长音取第一个长度。
    kind: NoteKind,
    /// 释放音角色；任一层为 `up: true` 时取 `Release`。
    role: NoteRole,
    /// 音频层。
    audio: Vec<AudioIndex>,
    /// 第一层的格式扩展。
    ext: BmsonNoteExt,
}

/// 待合并的单个可判定音符。
struct PendingNote {
    /// 玩家侧。
    side: NoteSide,
    /// 轨道。
    lane: Lane,
    /// 脉冲位置。
    tick: u64,
    /// 音符种类。
    kind: NoteKind,
    /// 游玩角色。
    role: NoteRole,
    /// 音频索引。
    audio: Option<AudioIndex>,
    /// 格式扩展。
    ext: BmsonNoteExt,
}

/// 把 `Lane` 编码为可排序的键，避免给核心类型加 `Ord`。
const fn lane_key(lane: Lane) -> (u8, u8) {
    match lane {
        Lane::Key(key) => (0, key.get()),
        Lane::Scratch(key) => (1, key.get()),
        Lane::FootPedal => (2, 0),
        _ => (3, 0),
    }
}

/// 合并一个可判定音符到分层表。
fn merge_layered_note(
    notes: &mut BTreeMap<(NoteSide, u8, u8, u64), LayeredNote>,
    warnings: &mut Vec<BmsonProcessWarning>,
    note: PendingNote,
) {
    let PendingNote {
        side,
        lane,
        tick,
        kind,
        role,
        audio,
        ext,
    } = note;
    let (lane_kind, lane_index) = lane_key(lane);
    let entry = notes
        .entry((side, lane_kind, lane_index, tick))
        .or_insert_with(|| LayeredNote {
            lane,
            kind,
            role,
            audio: Vec::new(),
            ext,
        });

    if let Some(index) = audio
        && !entry.audio.contains(&index)
    {
        entry.audio.push(index);
    }

    if role == NoteRole::Release {
        entry.role = NoteRole::Release;
    }

    match (entry.kind, kind) {
        (NoteKind::Long { duration: first }, NoteKind::Long { duration: other })
            if first != other =>
        {
            warnings.push(BmsonProcessWarning::LayeredNoteLengthMismatch { tick, first, other });
        }
        (NoteKind::Normal, NoteKind::Long { .. }) => {
            entry.kind = kind;
        }
        _ => {}
    }
}

impl BmsonConverter<'_> {
    /// 收集 BPM 变更事件。
    /// 值为 0 的 BPM 变更被跳过并生成警告。
    fn collect_bpm_events(&mut self) {
        for e in &self.bmson.chart_data.bpm_events {
            if e.bpm == 0.0 {
                self.warnings
                    .push(BmsonProcessWarning::ZeroBpmChangeIgnored { tick: e.y });
            } else {
                self.events.push(Event::bpm(e.y, e.bpm));
            }
        }
    }

    /// 收集停止事件。
    fn collect_stop_events(&mut self) {
        self.events.extend(
            self.bmson
                .chart_data
                .stop_events
                .iter()
                .map(|e| Event::stop(e.y, e.duration)),
        );
    }

    /// 收集 SCROLL 事件。
    fn collect_scroll_events(&mut self) {
        self.events.extend(
            self.bmson
                .scroll_events
                .iter()
                .map(|e| Event::scroll(e.y, e.rate)),
        );
    }

    /// 收集 BGA 事件（三层：Base / Layer / Poor）。
    ///
    /// `resource_id` 经 [`BmsonConverter::bga_ids`] 重编号为稠密下标；
    /// 引用 `bga_header` 中不存在 id 的事件被跳过（与 BMS 侧查表未命中
    /// 行为一致）。
    fn collect_bga_events(&mut self) {
        let bga = &self.bmson.chart_info.bga;
        for (layer, events) in [
            (BgaLayer::Base, &bga.bga_events),
            (BgaLayer::Layer, &bga.layer_events),
            (BgaLayer::Poor, &bga.poor_events),
        ] {
            for e in events {
                let Some(&resource_id) = self.bga_ids.get(&e.id) else {
                    continue;
                };
                self.events
                    .push(Event::new(e.y, EventKind::Bga { layer, resource_id }));
            }
        }
    }

    /// 从小节线定义或 4/4 默认值构建小节线事件。
    fn build_bar_lines(&mut self) {
        let last_tick = self.events.iter().map(Event::tick).max().unwrap_or(0);
        self.events.extend(build_bar_lines(
            self.bmson.chart_data.lines.as_deref(),
            self.timing.resolution(),
            last_tick,
        ));
    }
    /// 处理音频通道：切片、创建音符/BGM 事件。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio asset count fits in u32 for practical charts"
    )]
    fn process_sound_channels(&mut self) -> Result<(), ProcessError> {
        let mut layered: BTreeMap<(NoteSide, u8, u8, u64), LayeredNote> = BTreeMap::new();

        for channel in &self.bmson.chart_data.sound_channels {
            let path = normalize_path(channel.name)?;
            let sliced = slice_channel(channel, &self.timing, &path);

            let channel_playable: HashSet<u64> = channel
                .note_events
                .iter()
                .filter_map(|ne| if ne.is_bgm() { None } else { Some(ne.y) })
                .collect();

            for ne in &channel.note_events {
                let audio_idx = sliced
                    .pulse_to_index
                    .get(&ne.y)
                    .copied()
                    .map(|idx| AudioIndex::new((idx + self.audio_assets.len()) as u32));

                if ne.is_bgm() {
                    if channel_playable.contains(&ne.y) {
                        continue;
                    }
                    if let Some(idx) = audio_idx {
                        self.events.push(Event::bgm(ne.y, idx));
                    }
                } else {
                    let Some((side, lane)) = (self.decode)(ne.x) else {
                        continue;
                    };
                    let kind = if ne.l > 0 {
                        NoteKind::Long { duration: ne.l }
                    } else {
                        NoteKind::Normal
                    };
                    let role = if ne.up == Some(true) {
                        NoteRole::Release
                    } else {
                        NoteRole::Normal
                    };
                    merge_layered_note(
                        &mut layered,
                        &mut self.warnings,
                        PendingNote {
                            side,
                            lane,
                            tick: ne.y,
                            kind,
                            role,
                            audio: audio_idx,
                            ext: build_note_ext(ne),
                        },
                    );
                }
            }

            self.audio_assets.extend(sliced.assets);
        }

        for ((side, _, _, tick), note) in layered {
            self.events.push(Event::new(
                tick,
                EventKind::Note {
                    side,
                    lane: note.lane,
                    kind: note.kind,
                    role: note.role,
                    audio: NoteAudio::from_indices(note.audio),
                    ext: note.ext,
                },
            ));
        }

        Ok(())
    }

    /// 处理地雷通道：每个通道贡献一个整文件 `AudioAsset` 和地雷音符。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio asset count fits in u32 for practical charts"
    )]
    fn process_mine_channels(&mut self) -> Result<(), ProcessError> {
        for mc in &self.bmson.mine_channels {
            let path = normalize_path(mc.name)?;
            let mine_audio_idx = AudioIndex::new(self.audio_assets.len() as u32);
            self.audio_assets.push(AudioAsset {
                path: Arc::from(path),
                start: Duration::ZERO,
                duration: None,
            });

            for mn in &mc.notes {
                let Some((side, lane)) = (self.decode)(mn.x) else {
                    continue;
                };
                self.events.push(Event::new(
                    mn.y,
                    EventKind::Note {
                        side,
                        lane,
                        kind: NoteKind::Mine {
                            damage: Damage::new(mn.damage),
                        },
                        role: NoteRole::Normal,
                        audio: NoteAudio::one(mine_audio_idx),
                        ext: BmsonNoteExt::default(),
                    },
                ));
            }
        }
        Ok(())
    }

    /// 处理按键（不可见）通道：结构与地雷通道相同。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "audio asset count fits in u32 for practical charts"
    )]
    fn process_key_channels(&mut self) -> Result<(), ProcessError> {
        for kc in &self.bmson.key_channels {
            let path = normalize_path(kc.name)?;
            let key_audio_idx = AudioIndex::new(self.audio_assets.len() as u32);
            self.audio_assets.push(AudioAsset {
                path: Arc::from(path),
                start: Duration::ZERO,
                duration: None,
            });

            for kn in &kc.notes {
                let Some((side, lane)) = (self.decode)(kn.x) else {
                    continue;
                };
                self.events.push(Event::new(
                    kn.y,
                    EventKind::Note {
                        side,
                        lane,
                        kind: NoteKind::Invisible,
                        role: NoteRole::Normal,
                        audio: NoteAudio::one(key_audio_idx),
                        ext: BmsonNoteExt::default(),
                    },
                ));
            }
        }
        Ok(())
    }
}

/// 将 BMSON 资源路径规范化为安全的相对路径。
///
/// 拒绝绝对路径、Windows 盘符/UNC 前缀、`..` 父目录引用、NUL 与空路径；
/// 把反斜杠规范成正斜杠，使 Windows 风格路径在 Unix 上也能解析。
fn normalize_path(path: &Path) -> Result<PathBuf, ProcessError> {
    let raw = path.to_string_lossy();
    if raw.contains('\0') {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "NUL byte",
        });
    }
    if raw.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && raw.as_bytes().get(1) == Some(&b':')
    {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "drive prefix",
        });
    }

    let normalized = raw.replace('\\', "/");
    let normalized_path = Path::new(&normalized);
    if normalized_path.is_absolute() {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "absolute path",
        });
    }

    let mut safe = PathBuf::new();
    for component in normalized_path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(ProcessError::UnsafePath {
                    path: raw.into_owned(),
                    reason: "parent directory traversal",
                });
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ProcessError::UnsafePath {
                    path: raw.into_owned(),
                    reason: "absolute path",
                });
            }
        }
    }

    if safe.as_os_str().is_empty() {
        return Err(ProcessError::UnsafePath {
            path: raw.into_owned(),
            reason: "empty path",
        });
    }

    Ok(safe)
}

/// 规范化可选路径并转为 `String`。
fn normalize_optional_path(option: Option<&Path>) -> Result<Option<String>, ProcessError> {
    option
        .map(|raw| normalize_path(raw).map(|safe| safe.to_string_lossy().into_owned()))
        .transpose()
}

/// 从 BMSON 谱面数据构建 [`TimingTrack`]。
///
/// # Errors
///
/// 若 `data.init_bpm` 无效（零或非有限值），返回
/// [`TimingTrackError::InvalidBpm`]。
fn build_timing(data: &bmson_def::ChartData<'_>) -> Result<TimingTrack, TimingTrackError> {
    TimingTrack::new(
        data.init_bpm,
        data.bpm_events.iter().map(build_bpm_change).collect(),
        data.stop_events.iter().map(build_stop_event).collect(),
        data.resolution,
    )
}

/// 从 BMSON [`NoteEvent`](bmson_def::NoteEvent) 构建 [`BmsonNoteExt`]。
fn build_note_ext(ne: &bmson_def::NoteEvent) -> BmsonNoteExt {
    BmsonNoteExt {
        vol: ne.vol,
        pan: ne.pan,
        ln_type_hint: ne
            .t
            .map(ln_mode_to_hint)
            .or_else(|| ne.ln_type_hint.map(ln_type_to_hint)),
        ln_judge_hint: ne.ln_judge_hint.map(ln_judge_to_hint),
        ln_life_hint: ne.ln_life_hint.map(ln_life_to_hint),
    }
}

/// 将 beatoraja `t` 的 LN 模式转换为格式无关提示。
const fn ln_mode_to_hint(m: bmson_def::LnMode) -> LnTypeHint {
    match m {
        bmson_def::LnMode::Cn => LnTypeHint::Cn,
        bmson_def::LnMode::Hcn => LnTypeHint::Hcn,
        _ => LnTypeHint::Ln, // Ln + #[non_exhaustive] fallback
    }
}

/// 将 `bmson_def::LnType` 转换为格式无关的 [`LnTypeHint`]。
const fn ln_type_to_hint(lt: bmson_def::LnType) -> LnTypeHint {
    match lt {
        bmson_def::LnType::Cn => LnTypeHint::Cn,
        bmson_def::LnType::Hcn => LnTypeHint::Hcn,
        _ => LnTypeHint::Ln, // Ln + #[non_exhaustive] fallback
    }
}

/// 将 `bmson_def::LnJudge` 转换为格式无关的 [`LnJudgeHint`]。
const fn ln_judge_to_hint(lj: bmson_def::LnJudge) -> LnJudgeHint {
    match lj {
        bmson_def::LnJudge::Ticks => LnJudgeHint::Ticks,
        _ => LnJudgeHint::Normal,
    }
}

/// 将 `bmson_def::LnLife` 转换为格式无关的 [`LnLifeHint`]。
const fn ln_life_to_hint(ll: bmson_def::LnLife) -> LnLifeHint {
    match ll {
        bmson_def::LnLife::Ticks => LnLifeHint::Ticks,
        _ => LnLifeHint::Normal,
    }
}

/// 建立 BMSON BGA 原始 id 到稠密下标的映射。
///
/// `resource_id == bga_resources` 下标的不变量由本映射保证：`bga_header`
/// 的 id 可能稀疏或巨大，重编号后渲染器可安全按索引取资源表。
/// 重复的原始 id 以最后一次出现者生效（与 schema 注释一致，insert
/// 覆盖）。
fn build_bga_id_map(bga: &bmson_def::BGA<'_>) -> BTreeMap<u64, BgaResourceId> {
    bga.bga_header
        .iter()
        .enumerate()
        .map(|(i, h)| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "BGA header count fits in u32"
            )]
            (h.id, BgaResourceId::new(i as u32))
        })
        .collect()
}

/// 从 BMSON 乐曲信息构建 [`SongInfo`]。
fn build_song_info(bmson: &bmson_def::Bmson<'_>) -> SongInfo {
    SongInfo {
        title: bmson.song_info.title.to_owned(),
        artist: bmson.song_info.artist.to_owned(),
        genre: bmson.song_info.genre.to_owned(),
    }
}

/// 从 BMSON 谱面信息构建 [`ChartInfo`]。
fn build_chart_info(bmson: &bmson_def::Bmson<'_>) -> Result<ChartInfo, ProcessError> {
    let bga = &bmson.chart_info.bga;
    let bga_resources: Vec<BgaResource> = bga
        .bga_header
        .iter()
        .map(|h| {
            Ok(BgaResource {
                path: normalize_path(h.name)?,
                // BMSON 无裁剪/放置概念（图片/视频统一为路径）。
                crop: None,
            })
        })
        .collect::<Result<_, ProcessError>>()?;

    Ok(ChartInfo {
        subtitle: bmson.chart_info.subtitle.to_owned(),
        chart_name: bmson.chart_info.chart_name.to_owned(),
        level: bmson.chart_info.level,
        subartists: bmson
            .chart_info
            .subartists
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        title_image: normalize_optional_path(bmson.chart_info.title_image)?,
        back_image: normalize_optional_path(bmson.chart_info.back_image)?,
        eyecatch_image: normalize_optional_path(bmson.chart_info.eyecatch_image)?,
        banner_image: normalize_optional_path(bmson.chart_info.banner_image)?,
        preview_music: normalize_optional_path(bmson.chart_info.preview_music)?,
        bga_resources,
        // BMSON 无 #VIDEOFILE/#MOVIE 概念。
        video: None,
        key_count: mode_key_count(&bmson.chart_data.mode_hint),
    })
}

/// 从 `mode_hint` 推断可玩按键总数。
fn mode_key_count(mode: &bmson_def::ModeHint) -> Option<u8> {
    use bmson_def::ModeHint;

    match mode {
        ModeHint::Beat5k
        | ModeHint::Dj5kOnly
        | ModeHint::DjRuby
        | ModeHint::Dj5k
        | ModeHint::Popn5k => Some(5),
        ModeHint::Beat7k | ModeHint::Dj7k => Some(7),
        ModeHint::Beat10k | ModeHint::Dj10k => Some(10),
        ModeHint::Beat14k | ModeHint::Dj14k | ModeHint::DjAndromeda => Some(14),
        ModeHint::Popn9k => Some(9),
        ModeHint::Generic(count) => u8::try_from(*count).ok(),
        _ => None,
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
/// 范围从 0 到 `last_tick`。`last_tick` 由调用方从完整事件集（含 BGA）
/// 推算，确保覆盖所有事件类型。
fn build_bar_lines(
    lines: Option<&[bmson_def::BarLine]>,
    resolution: u64,
    last_tick: u64,
) -> Vec<Event<BmsonNoteExt>> {
    if let Some(vec) = lines {
        return vec.iter().map(|bl| Event::bar(bl.y)).collect();
    }

    let step = resolution * 4;
    let count = last_tick / step + 1;
    (0..=count).map(|i| Event::bar(i * step)).collect()
}
