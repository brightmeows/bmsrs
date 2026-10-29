//! BMS → `Chart` 转换处理器。
//!
//! [`BmsProcessor`] 通过应用 [`BmsLayout`] 模式族（例如 [`Bme`]、`Pms`、
//! `Nanasi`）将 `bms_parser::Bms` 转换为格式无关的 `Chart`。
//!
//! 模式族与 [`BmsLayout`] trait 位于本 crate 的 `layout` 模块中。
//!
//! # 管道
//!
//! ```text
//! bms_parser::Bms → BmsProcessor::process::<L>(bms) → Chart<(), NoCustomEvent>
//! ```
//!
//! # 模式族
//!
//! 布局类型决定 BMS `(player, lane)` 通道字节如何解码为音符位置。[`Bme`]
//! 统一覆盖 beat-5k/7k/10k/14k（一张谱面的按键数由其音符实际使用的轨道
//! 决定）。其他族（`Pms`、`PmsBme`、`Nanasi`、`DscOctFp`）各自覆盖同名
//! 模式。[`BmsProcessor::process_default`] 使用 [`Bme`]。
//!
//! # 长音模式
//!
//! BMS 支持两种 LN（长音）记法，自动选择：
//!
//! - **LNOBJ**：当定义了 `#LNOBJ` 时，由指定 WAV 索引配对的常规音符
//!   构成长音。
//! - **LNTYPE 1 (RDM)**：通道 51–69 上的事件按 `(player, lane)` 连续配对。

pub mod custom_event;
mod error;
mod long_note;
mod position;

pub mod layout;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bms_parser::{
    Audio, BmpIndex, BmsBgaLayer, BmsChannel as RawChannel, ChangeOptionIndex, ExRankIndex, LnMode,
    OwnedExWavParams, PlayerMode, Rank, SeekIndex, WavCmdParams, WavIndex,
};
use bms_parser::{Bms, BpmValue, Gameplay, KeyType};
use bmsrs_chart::{
    AudioAsset, AudioIndex, BgaResource, BgaResourceId, BpmChange, BpmLookup, Chart, ChartData,
    ChartInfo, CropRect, Damage, Event, EventKind, Lane, LnJudgeHint, LnLifeHint, LnTypeHint,
    NoteAudio, NoteKind, NoteRole, NoteSide, SongInfo, StopEvent, TimingTrack, TimingTrackError,
    VideoAsset,
};
use thiserror::Error;

pub use crate::error::ProcessWarning;

use crate::custom_event::BmsCustomEvent;
use crate::layout::{Bme, BmsLayout, ChannelPos, DscOctFp, Pms, PmsBme, PmsLayout};

use crate::long_note::{LnPairingResult, PairedLn, pair_lnobj, pair_lntype1, pair_lntype2};
use crate::position::MeasureTable;

/// BMS 处理期间可能发生的错误。
#[derive(Debug, Error)]
pub enum ProcessError {
    /// 计时轨构造失败（初始 BPM 无效或 resolution 为零）。
    ///
    /// 透传 [`TimingTrackError`]，不再在边界上降维翻译——此前
    /// `ZeroResolution` 被译成 `InvalidBpm(0.0)`，报误导性错误。
    #[error(transparent)]
    Timing(#[from] TimingTrackError),
    /// 初始 BPM 为零。
    ///
    /// BMS 的 `#BPM 0` 在部分实现中表示谱面不滚动，但当前时间模型无法
    /// 表示零 BPM；这里保持拒绝，而不是静默回退到 130。
    #[error("initial BPM must not be zero; BMS #BPM 0 is not supported")]
    ZeroBpm,
}

/// BMS 处理器产出的统一事件类型。
///
/// 泛型参数固定为 `((), BmsCustomEvent)`，其中 `BmsCustomEvent`
/// 承载引擎特定事件（BGA 不透明度、ARGB、TEXT 等）。
type BmsEvent = Event<(), BmsCustomEvent>;

/// 将 [`Bms`] 转换为 [`Chart`] 的零大小处理器。
pub struct BmsProcessor;

/// BMS 处理结果。
#[derive(Debug)]
pub struct Processed {
    /// 处理得到的谱面。
    pub chart: Chart<(), BmsCustomEvent>,
    /// 处理过程中收集的可恢复警告。
    pub warnings: Vec<ProcessWarning>,
    /// 未进入核心 `Chart` 的 BMS 游玩参数原始值。
    pub gameplay: BmsGameplay,
    /// 未进入核心 `AudioAsset` 的 BMS 音频参数原始值。
    pub audio: BmsAudio,
}

/// BMS 游玩参数侧车。
///
/// 这些字段的换算规则依赖具体引擎，不进入格式无关的 `Chart`；需要
/// 原始值的消费者从 [`Processed::gameplay`] 读取。
#[derive(Debug, Clone, PartialEq)]
pub struct BmsGameplay {
    /// 游戏模式（`#PLAYER`）。
    pub player: Option<PlayerMode>,
    /// 判定难度（`#RANK`）。
    pub rank: Option<Rank>,
    /// 精细判定难度（`#DEFEXRANK`）。
    pub def_ex_rank: Option<f64>,
    /// 血量槽最大增量（`#TOTAL`）。
    pub total: Option<f64>,
    /// 主音量百分比（`#VOLWAV`）。
    pub vol_wav: Option<f64>,
    /// 按位置的判定宽度覆盖（`#EXRANKxx`）。
    pub ex_rank_defs: BTreeMap<ExRankIndex, f64>,
    /// 引擎特有选项字符串（`#OPTION`）。
    pub option: Option<String>,
    /// 动态选项变更定义（`#CHANGEOPTIONxx`）。
    pub change_option_defs: BTreeMap<ChangeOptionIndex, String>,
}

impl From<&Gameplay> for BmsGameplay {
    fn from(gameplay: &Gameplay) -> Self {
        Self {
            player: gameplay.player,
            rank: gameplay.rank,
            def_ex_rank: gameplay.def_ex_rank,
            total: gameplay.total,
            vol_wav: gameplay.vol_wav,
            ex_rank_defs: gameplay.ex_rank_defs.clone(),
            option: gameplay.option.clone(),
            change_option_defs: gameplay.change_option_defs.clone(),
        }
    }
}

/// BMS 音频参数侧车。
///
/// `#PATH_WAV` 已经应用到 [`AudioAsset::path`](bmsrs_chart::AudioAsset::path)；
/// 这里保留原始前缀与其他未进入核心模型的参数。
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "侧车保留 BMS 原始参数，部分字段无法安全实现 Eq"
)]
#[derive(Debug, Clone, PartialEq)]
pub struct BmsAudio {
    /// `#EXWAV` 的逐 WAV 效果参数。
    pub ex_wav_params: BTreeMap<WavIndex, OwnedExWavParams>,
    /// `#WAVCMD` 全局播放命令。
    pub wav_cmd: Option<WavCmdParams>,
    /// `#CDDA` 音轨引用。
    pub cdda: Option<String>,
    /// `#MIDIFILE` 引用。
    pub midifile: Option<String>,
    /// `#PATH_WAV` 原始目录前缀。
    pub path_wav: Option<String>,
}

impl From<&Audio> for BmsAudio {
    fn from(audio: &Audio) -> Self {
        Self {
            ex_wav_params: audio.ex_wav_params.clone(),
            wav_cmd: audio.wav_cmd.clone(),
            cdda: audio.cdda.clone(),
            midifile: audio.midifile.clone(),
            path_wav: audio.path_wav.clone(),
        }
    }
}

/// BMS 谱面的默认节拍分辨率（每四分音符的脉冲数）。
const RESOLUTION: u64 = 240;

impl BmsProcessor {
    /// 使用显式模式族布局处理 BMS 谱面，并返回处理警告。
    ///
    /// 布局类型决定每个 BMS `(player, lane)` 通道字节如何解码为音符
    /// 位置。可用族见 `layout` 模块。
    ///
    /// # Errors
    ///
    /// 若计时轨构造失败（初始 BPM 缺失或为零/非有限值），返回
    /// [`ProcessError::Timing`]。
    pub fn process<L>(bms: &Bms) -> Result<Processed, ProcessError>
    where
        L: BmsLayout,
    {
        let init_bpm = bms.timing.bpm.unwrap_or(130.0);
        if init_bpm == 0.0 {
            return Err(ProcessError::ZeroBpm);
        }

        let max_measure = find_max_measure(bms);
        let table = MeasureTable::new(max_measure, &bms.messages.measure_lengths, RESOLUTION);
        let mut conv = BmsConverter {
            bms,
            table: &table,
            events: Vec::with_capacity(1024),
            warnings: Vec::new(),
        };

        let (wav_map, audio_assets) = build_audio_assets(&bms.audio);
        let bpm_changes = conv.build_bpm_changes();
        let bpm_lookup = BpmLookup::new(init_bpm, &bpm_changes);

        let mut stops = conv.build_stops_from_defs();
        stops.extend(conv.build_stops_from_stp(&bpm_lookup));
        stops.sort_by_key(|s| s.tick);

        let timing = TimingTrack::new(init_bpm, bpm_changes, stops, RESOLUTION)?;

        let bmp_map = conv.build_bmp_map();

        let (ln_result, consumed) = conv.pair_long_notes();
        let paired_lns = ln_result.paired;
        let unpaired_starts = ln_result.unpaired_starts;

        // 收集未配对长音警告。
        for &(player, lane, tick) in &unpaired_starts {
            conv.warnings
                .push(ProcessWarning::UnterminatedLongNote { player, lane, tick });
        }

        // 小节线优先（优先级 0）。
        conv.events.extend(build_bar_events(&table));

        // 音符（优先级 1）。
        conv.collect_notes::<L>(&wav_map, &paired_lns, &consumed, &unpaired_starts);

        // BGM（优先级 1）。
        conv.collect_bgm(&wav_map);

        // LNOBJ 终点标记作为 BGM 播放（按 BMS 规范）。
        conv.collect_lnobj_bgm(&wav_map, &consumed);

        // BGA（优先级 1）。
        conv.collect_bga(&bmp_map);

        // BPM 变更（优先级 2）。
        conv.collect_bpm_events();

        // 停止事件（优先级 3）—— 重新遍历计时停止事件。
        conv.collect_stop_events(timing.stops());

        // SCROLL 事件（优先级 4）。
        conv.collect_scroll_events();

        // SPEED 事件（优先级 5）。
        conv.collect_speed_events();

        // BMS 引擎特定自定义事件（优先级 6）。
        conv.collect_custom_events(&bmp_map);

        let (song, chart_info) = conv.build_metadata(&bmp_map);
        let ln_type_hint = conv.ln_type_hint();

        let mut data = ChartData {
            timing,
            judge_multiplier: None,
            life_multiplier: None,
            ln_type_hint,
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: conv.events,
            audio_assets,
        };
        data.sort_events();

        // 警告聚合：parser 层的值解析警告在前（按文件位置），processor 层
        // 的引用缺失/钳位类在后。在 build_metadata 的借用结束后取走。
        let mut warnings: Vec<ProcessWarning> = bms
            .messages
            .warnings
            .iter()
            .cloned()
            .map(ProcessWarning::UnparsableValue)
            .collect();
        warnings.extend(conv.warnings);
        let gameplay = BmsGameplay::from(&bms.gameplay);
        let audio = BmsAudio::from(&bms.audio);

        Ok(Processed {
            chart: Chart {
                song,
                chart: chart_info,
                data,
            },
            warnings,
            gameplay,
            audio,
        })
    }

    /// 使用自动检测的 PMS 变体布局处理 BMS 谱面。
    ///
    /// 根据谱面中使用的通道自动选择 [`Pms`]（Standard）或 [`PmsBme`]（BME-type）。
    ///
    /// # Errors
    ///
    /// 若计时轨构造失败（初始 BPM 缺失或为零/非有限值），返回
    /// [`ProcessError::Timing`]。
    pub fn process_pms(bms: &Bms) -> Result<Processed, ProcessError> {
        match PmsLayout::detect(bms) {
            PmsLayout::Standard => Self::process::<Pms>(bms),
            PmsLayout::BmeType => Self::process::<PmsBme>(bms),
        }
    }

    /// 使用默认布局处理 BMS 谱面。
    ///
    /// 若谱面声明了 `#OCT/FP`，使用 [`DscOctFp`]；否则使用 [`Bme`]。
    /// `Bme` 统一覆盖 beat-5k/7k/10k/14k —— 两侧玩家均被映射，因此单人
    /// 谱面只是让 2P 轨道空置，双人谱面则填充它们。`#PLAYER` 头部命令不
    /// 影响映射（与现代引擎一致，均忽略该字段）。
    ///
    /// 若需 PMS、nanasi 或其他族，请以相应布局类型调用
    /// [`process::<Nanasi>`](Self::process)。
    ///
    /// # Errors
    ///
    /// 若计时轨构造失败（初始 BPM 缺失或为零/非有限值），返回
    /// [`ProcessError::Timing`]。
    pub fn process_default(bms: &Bms) -> Result<Processed, ProcessError> {
        if bms.gameplay.oct_fp == Some(true) {
            Self::process::<DscOctFp>(bms)
        } else {
            Self::process::<Bme>(bms)
        }
    }
}

/// BMS → [`Chart`] 转换上下文。
///
/// 持有源文档 [`Bms`]、小节脉冲表 [`MeasureTable`] 以及正在构建的
/// 事件缓冲区，使各 `collect_*` 步骤直接写入内部 `events`，无需调用方
/// 传入 `&mut Vec`。仅供 [`BmsProcessor::process`] 内部使用。
struct BmsConverter<'a> {
    /// 源 BMS 文档。
    bms: &'a Bms,
    /// 小节 → 累计脉冲查找表。
    table: &'a MeasureTable,
    /// 正在构建的事件向量，最终由 [`BmsProcessor::process`] 提取。
    events: Vec<BmsEvent>,
    /// 处理过程中收集的可恢复警告。
    warnings: Vec<ProcessWarning>,
}

impl BmsConverter<'_> {
    /// 判定 LN 模式并配对长音。返回配对结果与已消耗的音符索引。
    fn pair_long_notes(&self) -> (LnPairingResult, HashSet<usize>) {
        if let Some(ln_obj) = self.bms.gameplay.ln_obj {
            let (paired, consumed) = pair_lnobj(&self.bms.messages.note_events, ln_obj, self.table);
            return (
                LnPairingResult {
                    paired,
                    unpaired_starts: Vec::new(),
                },
                consumed,
            );
        }
        if self.bms.gameplay.ln_type == Some(bms_parser::LnType::Type2) {
            return (
                pair_lntype2(&self.bms.messages.long_note_events, self.table),
                HashSet::new(),
            );
        }
        (
            pair_lntype1(&self.bms.messages.long_note_events, self.table),
            HashSet::new(),
        )
    }

    /// 由 `#LNMODE` 头部推导谱面级长音类型提示。
    ///
    /// `LnMode{Ln,Cn,Hcn}` 与 `LnTypeHint{Ln,Cn,Hcn}` 是 1:1 同构枚举。
    /// `None`（未声明 `#LNMODE`）映射为 [`LnTypeHint::Ln`]，与原
    /// `LnTypeHint::default()` 一致但语义显式。
    const fn ln_type_hint(&self) -> LnTypeHint {
        match self.bms.gameplay.ln_mode {
            Some(LnMode::Ln) | None => LnTypeHint::Ln,
            Some(LnMode::Cn) => LnTypeHint::Cn,
            Some(LnMode::Hcn) => LnTypeHint::Hcn,
        }
    }
    /// 收集全部可玩音符到事件向量中。
    #[expect(
        clippy::too_many_lines,
        reason = "普通/不可见/LNOBJ/长音/地雷五种来源共用同一匹配逻辑，拆开反而割裂"
    )]
    fn collect_notes<L: BmsLayout>(
        &mut self,
        wav_map: &BTreeMap<WavIndex, AudioIndex>,
        paired_lns: &[PairedLn],
        consumed: &HashSet<usize>,
        unpaired_starts: &[(u8, u8, u64)],
    ) {
        let push_note = |tick: u64,
                         side,
                         lane,
                         kind: NoteKind,
                         audio: Option<AudioIndex>,
                         ev: &mut Vec<BmsEvent>| {
            if let Some((note_side, note_lane)) =
                ChannelPos::new(side, lane).and_then(L::map_channel)
            {
                ev.push(Event::new(
                    tick,
                    EventKind::Note {
                        side: note_side,
                        lane: note_lane,
                        kind,
                        role: NoteRole::Normal,
                        audio: NoteAudio::from(audio),
                        ext: (),
                    },
                ));
            }
        };

        let mut check_wav =
            |wav_id: &WavIndex, player: u8, lane: u8, tick: u64| -> Option<AudioIndex> {
                let audio = wav_map.get(wav_id).copied();
                // `to_index()` 返回 `None` 时静默跳过警告：非可分解的 WAV 键
                // （如 "ZZ" LNOBJ 标记）不属于 WAV 表定义的范围，不产生
                // 缺失定义警告。
                if audio.is_none()
                    && let Some(key) = wav_id.0.to_index()
                {
                    self.warnings.push(ProcessWarning::MissingWavDefinition {
                        key,
                        player,
                        lane,
                        tick,
                    });
                }
                audio
            };

        // 构建 LN 区间表：(player, lane) → Vec<(start_tick, end_tick)>。
        let ln_ranges = build_ln_ranges(paired_lns);

        // 可玩音符（跳过已消耗的 LNOBJ 配对；可见音符另受 LN 区间抑制）。
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i) {
                continue;
            }
            let tick = self.table.position_to_tick(ne.position);
            match ne.key_type {
                KeyType::Visible => {
                    if is_note_suppressed(ne.player, ne.lane, tick, &ln_ranges, unpaired_starts) {
                        continue;
                    }
                    push_note(
                        tick,
                        ne.player,
                        ne.lane,
                        NoteKind::Normal,
                        check_wav(&ne.wav_id, ne.player, ne.lane, tick),
                        &mut self.events,
                    );
                }
                KeyType::Invisible => push_note(
                    tick,
                    ne.player,
                    ne.lane,
                    NoteKind::Invisible,
                    check_wav(&ne.wav_id, ne.player, ne.lane, tick),
                    &mut self.events,
                ),
            }
        }

        // LNOBJ 模式下 ch51-69 长音通道与 LNOBJ 互斥（memo/10 规范未定义）；
        // 不静默丢弃，作为普通可见音符保留数据。
        if self.bms.gameplay.ln_obj.is_some() {
            for lne in &self.bms.messages.long_note_events {
                let tick = self.table.position_to_tick(lne.position);
                push_note(
                    tick,
                    lne.player,
                    lne.lane,
                    NoteKind::Normal,
                    check_wav(&lne.wav_id, lne.player, lne.lane, tick),
                    &mut self.events,
                );
            }
        }

        // 已配对的长音。
        for ln in paired_lns {
            push_note(
                ln.tick,
                ln.player,
                ln.lane,
                NoteKind::Long {
                    duration: ln.duration,
                },
                check_wav(&ln.wav_id, ln.player, ln.lane, ln.tick),
                &mut self.events,
            );
        }

        // 地雷。
        for me in &self.bms.messages.mine_events {
            push_note(
                self.table.position_to_tick(me.position),
                me.player,
                me.lane,
                NoteKind::Mine {
                    damage: Damage::new(mine_damage(me.raw_value)),
                },
                None,
                &mut self.events,
            );
        }
    }

    /// 收集 BGM 事件到事件向量中。
    fn collect_bgm(&mut self, wav_map: &BTreeMap<WavIndex, AudioIndex>) {
        for be in &self.bms.messages.bgm_events {
            if let Some(&audio) = wav_map.get(&be.wav_id) {
                self.events
                    .push(Event::bgm(self.table.position_to_tick(be.position), audio));
            }
        }
    }

    /// 收集 LNOBJ 终点标记的 BGM 事件。
    ///
    /// 按 BMS 规范，当 [`#LNOBJ`](bms_parser::BmsHeaderGameplay::LnObj)
    /// 终点标记经过判定线时，其 WAV 文件作为 BGM 播放。此函数遍历
    /// [`pair_lnobj`] 返回的 `consumed` 音符索引，为每个终点标记生成一个
    /// BGM 事件。
    fn collect_lnobj_bgm(
        &mut self,
        wav_map: &BTreeMap<WavIndex, AudioIndex>,
        consumed: &HashSet<usize>,
    ) {
        let Some(ln_obj) = self.bms.gameplay.ln_obj else {
            return;
        };
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i)
                && *ne.wav_id == *ln_obj
                && let Some(&audio) = wav_map.get(&ne.wav_id)
            {
                self.events
                    .push(Event::bgm(self.table.position_to_tick(ne.position), audio));
            }
        }
    }

    /// 构建 BPM 事件（用于统一时间线）。
    ///
    /// 无效值（零、NaN、无穷大，判定谓词 [`BpmChange::is_valid`]）
    /// 被跳过并产生警告，与计时轨构造的过滤语义一致。
    fn collect_bpm_events(&mut self) {
        for bc in &self.bms.messages.bpm_changes {
            let tick = self.table.position_to_tick(bc.position);
            let bpm = self.resolve_bpm(bc.value, tick);
            if BpmChange::is_valid(bpm) {
                self.events.push(Event::bpm(tick, bpm));
            } else {
                self.warnings
                    .push(ProcessWarning::InvalidBpmChangeIgnored { tick, bpm });
            }
        }
    }

    /// 构建 SCROLL 变更事件。
    fn collect_scroll_events(&mut self) {
        for se in &self.bms.messages.scroll_events {
            if let Some(&rate) = self.bms.timing.scroll_defs.get(&se.scroll_id) {
                self.events.push(Event::scroll(
                    self.table.position_to_tick(se.position),
                    rate,
                ));
            }
        }
    }

    /// 构建 SPEED（视觉音符间距）关键帧事件。
    fn collect_speed_events(&mut self) {
        for se in &self.bms.messages.speed_events {
            if let Some(&rate) = self.bms.timing.speed_defs.get(&se.speed_id) {
                self.events
                    .push(Event::speed(self.table.position_to_tick(se.position), rate));
            }
        }
    }

    /// 从已构建的停止事件切片生成 Stop 事件。
    fn collect_stop_events(&mut self, stops: &[StopEvent]) {
        for se in stops {
            self.events.push(Event::stop(se.tick, se.duration));
        }
    }

    /// 收集 BMS 引擎特定自定义事件。
    ///
    /// 从 [`Messages::non_event_data`] 读取由 parser 合并且已归一化的通道数据，
    /// 转换为 [`EventKind::Custom`] 变体。每个非 `"00"` 值产生一个事件。
    ///
    /// **归一化保证**：parser 层已按预扫描的 `#BASE` 基数完成所有索引归一化，
    /// 此处查表无需再次归一化（消除了此前 F1/F2 标记的重复归一化）。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "channel values are single-byte bounded (0-255) or u32 resource indices"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "single structured match on channel variant with short branches; extraction would lose clarity"
    )]
    fn collect_custom_events(&mut self, bmp_map: &BTreeMap<BmpIndex, BmpEntry>) {
        let non_event = &self.bms.messages.non_event_data;

        // BGA 图层映射（通道 → 图层）。
        let layer_of = |ch: RawChannel| -> Option<bmsrs_chart::BgaLayer> {
            match ch {
                RawChannel::BgaBaseOpacity | RawChannel::BgaArgbBase => {
                    Some(bmsrs_chart::BgaLayer::Base)
                }
                RawChannel::BgaLayerOpacity | RawChannel::BgaArgbLayer => {
                    Some(bmsrs_chart::BgaLayer::Layer)
                }
                RawChannel::BgaLayer2Opacity | RawChannel::BgaArgbLayer2 => {
                    Some(bmsrs_chart::BgaLayer::Layer2)
                }
                RawChannel::BgaPoorOpacity | RawChannel::BgaArgbPoor => {
                    Some(bmsrs_chart::BgaLayer::Poor)
                }
                _ => None,
            }
        };

        for item in non_event {
            let total = item.values.len() as u32;
            if total == 0 {
                continue;
            }

            for (i, obj) in item.values.iter().enumerate() {
                if obj.as_str() == "00" {
                    continue;
                }
                // 值通道（0B-0E, 97, 98）使用十六进制（01-FF）；
                // 索引通道（99, A0, A1-A5, A6, 05）使用 Base36 索引。
                // 所有值已在 parser 层按 base 归一化，查表无需再次归一化。
                let tick = self.table.position_to_tick(bms_parser::Position::new(
                    item.measure,
                    i as u32,
                    total,
                ));

                let payload = match item.channel {
                    RawChannel::BgaBaseOpacity
                    | RawChannel::BgaLayerOpacity
                    | RawChannel::BgaLayer2Opacity
                    | RawChannel::BgaPoorOpacity => {
                        let Some(layer) = layer_of(item.channel) else {
                            continue;
                        };
                        let Some(opacity) = obj.as_u8_hex() else {
                            continue;
                        };
                        BmsCustomEvent::BgaOpacity { layer, opacity }
                    }
                    RawChannel::BgaArgbBase
                    | RawChannel::BgaArgbLayer
                    | RawChannel::BgaArgbLayer2
                    | RawChannel::BgaArgbPoor => {
                        let Some(layer) = layer_of(item.channel) else {
                            continue;
                        };
                        // 值已归一化，直接查表。
                        let Some((a, r, g, b)) = self
                            .bms
                            .visual
                            .argb_defs
                            .get(&BmpIndex::from(*obj))
                            .cloned()
                            .map(|p| (p.a, p.r, p.g, p.b))
                        else {
                            continue;
                        };
                        BmsCustomEvent::BgaArgb { layer, a, r, g, b }
                    }
                    RawChannel::BgaKeyBound => {
                        // 值已归一化，直接查 bmp_map。
                        let Some(resource_id) =
                            bmp_map.get(&BmpIndex::from(*obj)).map(|e| e.resource_id)
                        else {
                            continue;
                        };
                        BmsCustomEvent::BgaKeyBound { resource_id }
                    }
                    RawChannel::Text => {
                        let index = u64::from(obj.to_index().unwrap_or(0));
                        BmsCustomEvent::TextDisplay {
                            text_index: index as u32,
                        }
                    }
                    RawChannel::Judge => {
                        let index = u64::from(obj.to_index().unwrap_or(0));
                        BmsCustomEvent::JudgeOverride { rank: index }
                    }
                    RawChannel::Option => {
                        let option_id = u64::from(obj.to_index().unwrap_or(0));
                        // 值已归一化，直接查 change_option_defs（F1+F2 已在 parser 解决）。
                        let opt_idx = ChangeOptionIndex::from(*obj);
                        let value = self
                            .bms
                            .gameplay
                            .change_option_defs
                            .get(&opt_idx)
                            .cloned()
                            .unwrap_or_default();
                        BmsCustomEvent::OptionChange { option_id, value }
                    }
                    RawChannel::BgmVolume => {
                        let Some(volume) = obj.as_u8_hex() else {
                            continue;
                        };
                        BmsCustomEvent::BgmVolume { volume }
                    }
                    RawChannel::KeyVolume => {
                        let Some(volume) = obj.as_u8_hex() else {
                            continue;
                        };
                        BmsCustomEvent::KeyVolume { volume }
                    }
                    RawChannel::Seek => {
                        // 值已归一化，直接查 seek_defs。
                        let Some(ms) = self
                            .bms
                            .visual
                            .seek_defs
                            .get(&SeekIndex::from(*obj))
                            .copied()
                        else {
                            continue;
                        };
                        // VideoSeek.position 为 u64（BmsCustomEvent 派生 Eq）；
                        // 毫秒语义非负，钳位后转换。
                        #[expect(
                            clippy::cast_sign_loss,
                            reason = "clamped to non-negative before cast"
                        )]
                        #[expect(clippy::cast_possible_truncation, reason = "seek ms fits in u64")]
                        let position = ms.max(0.0).round() as u64;
                        BmsCustomEvent::VideoSeek { position }
                    }
                    _ => continue,
                };
                self.events
                    .push(Event::new(tick, EventKind::Custom(payload)));
            }
        }
    }

    /// 从 BGA 事件与 BMP 文件定义构建 BGA 事件。
    fn collect_bga(&mut self, bmp_map: &BTreeMap<BmpIndex, BmpEntry>) {
        for be in &self.bms.messages.bga_events {
            if let Some(entry) = bmp_map.get(&be.bmp_id) {
                self.events.push(Event::new(
                    self.table.position_to_tick(be.position),
                    EventKind::Bga {
                        layer: map_bga_layer(be.layer),
                        resource_id: entry.resource_id,
                    },
                ));
            }
        }
    }

    /// 为计时轨构建 BPM 变更事件。
    ///
    /// 先按 `tick` 排序再在有序序列上去重：相邻同值变更保留首个。
    /// 顺序不能反——`messages.bpm_changes` 按（小节，通道）分块累积而非
    /// 按 tick 有序，排序前去重会合并向量相邻但时间不相邻的同值变更。
    fn build_bpm_changes(&mut self) -> Vec<BpmChange> {
        let mut changes: Vec<BpmChange> = self
            .bms
            .messages
            .bpm_changes
            .iter()
            .filter_map(|bc| {
                let tick = self.table.position_to_tick(bc.position);
                let bpm = self.resolve_bpm(bc.value, tick);
                BpmChange::is_valid(bpm).then_some(BpmChange { tick, bpm })
            })
            .collect();
        // 去重必须在按 tick 排序之后：同通道的两个同值变更在向量中相邻
        // （通道内分块），若中间夹着另一通道的变更，排序前去重会把它们
        // 错误合并。
        changes.sort_by_key(|c| c.tick);
        changes.dedup_by(|a, b| (a.bpm - b.bpm).abs() < f64::EPSILON);
        changes
    }

    /// 构建 BMP 索引到 [`BmpEntry`]（稠密 id + 路径 + 可选裁剪）的映射。
    ///
    /// 合并 `#BMP`、`#BGA`、`#@BGA` 三个命名空间的键：
    ///
    /// - 若某 id 在 `#BGA` / `#@BGA` 中有定义，则按 BMS 规范“`#BGA` 优先”，
    ///   路径取裁剪定义的源 BMP（经十进制 `bmp_index` 解析），并附加 [`CropRect`]。
    /// - 否则路径取 `#BMP` 自身，裁剪为 `None`。
    /// - 源路径无法解析的裁剪 id 被跳过（不进入资源表，也不产生事件）。
    ///
    /// `#@BGA`（宽/高形式）归一为 `#BGA`（右下角形式）：
    /// `x2 = sx + w`、`y2 = sy + h`。
    #[expect(clippy::cast_possible_truncation, reason = "BMP count fits in u32")]
    fn build_bmp_map(&self) -> BTreeMap<BmpIndex, BmpEntry> {
        let visual = &self.bms.visual;
        let mut map: BTreeMap<BmpIndex, BmpEntry> = BTreeMap::new();

        // 收集所有涉及的 id（bmp_files ∪ crop_defs ∪ alt_crop_defs）。
        let mut ids: BTreeSet<BmpIndex> = visual.bmp_files.keys().copied().collect();
        ids.extend(visual.crop_defs.keys().copied());
        ids.extend(visual.alt_crop_defs.keys().copied());

        for (i, id) in ids.into_iter().enumerate() {
            // 裁剪定义：#BGA 优先于 #@BGA（同一 id 不太可能同时出现）。
            // 两者的 (CropRect, 源 bmp_index)；#@BGA 的 w/h 归一为右下角。
            let crop_def = visual
                .crop_defs
                .get(&id)
                .map(|p| {
                    (
                        CropRect {
                            x1: p.x1,
                            y1: p.y1,
                            x2: p.x2,
                            y2: p.y2,
                            dx: p.dx,
                            dy: p.dy,
                        },
                        p.bmp_index,
                    )
                })
                .or_else(|| {
                    visual.alt_crop_defs.get(&id).map(|p| {
                        (
                            CropRect {
                                x1: p.sx,
                                y1: p.sy,
                                x2: p.sx + p.w,
                                y2: p.sy + p.h,
                                dx: p.dx,
                                dy: p.dy,
                            },
                            p.bmp_index,
                        )
                    })
                });

            // 路径：有裁剪定义时取源 BMP（#BGA 优先），否则取 #BMP 自身。
            let (crop, path_opt) = match crop_def {
                Some((crop, source_index)) => (
                    Some(crop),
                    resolve_bmp_path(&visual.bmp_files, source_index).cloned(),
                ),
                None => (None, visual.bmp_files.get(&id).cloned()),
            };

            let Some(path) = path_opt else {
                continue;
            };

            map.insert(
                id,
                BmpEntry {
                    resource_id: BgaResourceId::new(i as u32),
                    path,
                    crop,
                },
            );
        }

        map
    }

    /// 将 [`BpmValue`] 解析为具体 BPM 值，对引用值使用 BPM 定义表。
    fn resolve_bpm(&mut self, value: BpmValue, tick: u64) -> f64 {
        match value {
            BpmValue::Absolute(bpm) => bpm,
            BpmValue::Reference(id) => {
                if let Some(bpm) = self.bms.timing.bpm_defs.get(&id).copied() {
                    bpm
                } else {
                    if let Some(key) = id.to_index() {
                        self.warnings
                            .push(ProcessWarning::MissingBpmDefinition { key, tick });
                    }
                    120.0
                }
            }
        }
    }

    /// 从 `#STOPxx` 定义（通道 `09`）构建停止事件。
    ///
    /// 负 STOP 值被钳位到 `0`（跳过/忽略），与 beatoraja/Angolmois 的行为一致。
    #[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    fn build_stops_from_defs(&mut self) -> Vec<StopEvent> {
        self.bms
            .messages
            .stop_events
            .iter()
            .filter_map(|se| {
                let tick = self.table.position_to_tick(se.position);
                if let Some(&raw) = self.bms.timing.stop_defs.get(&se.stop_id) {
                    let duration = (raw / 192.0 * RESOLUTION as f64 * 4.0).round();
                    if duration < 0.0 {
                        self.warnings.push(ProcessWarning::StopDurationClipped {
                            tick,
                            original: duration,
                        });
                    }
                    Some(StopEvent {
                        tick,
                        duration: duration.max(0.0) as u64,
                    })
                } else {
                    if let Some(key) = se.stop_id.to_index() {
                        self.warnings
                            .push(ProcessWarning::MissingStopDefinition { key, tick });
                    }
                    None
                }
            })
            .collect()
    }

    /// 从 `#STP` 头部命令构建停止事件（时长以毫秒计）。
    ///
    /// 负 STOP 值被钳位到 `0`（跳过/忽略），与 beatoraja/Angolmois 的行为一致。
    #[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    fn build_stops_from_stp(&self, bpm_lookup: &BpmLookup<'_>) -> Vec<StopEvent> {
        self.bms
            .messages
            .stp_events
            .iter()
            .map(|stp| {
                let tick = self.table.position_to_tick(stp.position);
                let bpm = bpm_lookup.bpm_at_tick(tick);
                let tick_duration = stp.duration_ms / 1000.0 * bpm / 60.0 * RESOLUTION as f64;
                StopEvent {
                    tick,
                    duration: tick_duration.round().max(0.0) as u64,
                }
            })
            .collect()
    }

    /// 从 BMS 元数据与 BMP 映射构建 [`SongInfo`] 与 [`ChartInfo`]。
    #[expect(clippy::cast_possible_truncation, reason = "play level fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "play level is non-negative")]
    fn build_metadata(&self, bmp_map: &BTreeMap<BmpIndex, BmpEntry>) -> (SongInfo, ChartInfo) {
        let bga_resources: Vec<BgaResource> = bmp_map
            .values()
            .map(|e| BgaResource {
                path: e.path.clone().into(),
                crop: e.crop,
            })
            .collect();
        let video = self.build_video_asset();
        (
            SongInfo {
                title: self.bms.metadata.title.clone().unwrap_or_default(),
                artist: self.bms.metadata.artist.clone().unwrap_or_default(),
                genre: self.bms.metadata.genre.clone().unwrap_or_default(),
            },
            ChartInfo {
                subtitle: self.bms.metadata.subtitle.clone().unwrap_or_default(),
                chart_name: self
                    .bms
                    .display
                    .play_level
                    .map(|l| format!("{l:.0}"))
                    .unwrap_or_default(),
                level: self.bms.display.play_level.map_or(0, |l| l as u64),
                subartists: self
                    .bms
                    .metadata
                    .sub_artist
                    .as_deref()
                    .map(|s| vec![s.to_owned()])
                    .unwrap_or_default(),
                title_image: None,
                back_image: self.bms.display.back_bmp.clone(),
                eyecatch_image: self.bms.display.stage_file.clone(),
                banner_image: self.bms.display.banner.clone(),
                preview_music: self.bms.display.preview.clone(),
                bga_resources,
                video,
                key_count: key_count(&self.events),
            },
        )
    }

    /// 从 `#VIDEOFILE` / `#MOVIE` 及视频参数构建 [`VideoAsset`]。
    ///
    /// `#VIDEOFILE`（循环）优先于 `#MOVIE`（单次）。两者均无则返回 `None`。
    /// `#VIDEOf/s` / `#VIDEOCOLORS` / `#VIDEODLY` 作为可选参数附加；
    /// 其中 `#VIDEOCOLORS` / `#VIDEODLY` 语义为整数，由 `f64` 转为 `u32`。
    #[expect(clippy::cast_possible_truncation, reason = "video params fit in u32")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    fn build_video_asset(&self) -> Option<VideoAsset> {
        let visual = &self.bms.visual;
        let (path, loop_playback) = if let Some(p) = &visual.video_file {
            (p.clone(), true)
        } else {
            (visual.movie.clone()?, false)
        };
        let mut asset = VideoAsset::new(PathBuf::from(path), loop_playback);
        if let Some(fps) = visual.video_fps {
            asset = asset.with_fps(fps);
        }
        if let Some(colors) = visual.video_colors {
            asset = asset.with_colors(colors.max(0.0).round() as u32);
        }
        if let Some(delay) = visual.video_dly {
            asset = asset.with_delay_frames(delay.max(0.0).round() as u32);
        }
        Some(asset)
    }
}

/// 将 BMS 语义层的图层枚举映射为格式无关的 [`bmsrs_chart::BgaLayer`]。
const fn map_bga_layer(layer: BmsBgaLayer) -> bmsrs_chart::BgaLayer {
    match layer {
        BmsBgaLayer::Base => bmsrs_chart::BgaLayer::Base,
        BmsBgaLayer::Poor => bmsrs_chart::BgaLayer::Poor,
        BmsBgaLayer::Layer => bmsrs_chart::BgaLayer::Layer,
        BmsBgaLayer::Layer2 => bmsrs_chart::BgaLayer::Layer2,
    }
}

// 纯转换辅助函数（不依赖 `&Bms` + `&MeasureTable` 组合）

/// 查找任意事件所引用的最大小节号。
///
/// 在 [`BmsConverter`] 构造前调用——其结果是 [`MeasureTable::new`] 的输入。
fn find_max_measure(bms: &Bms) -> u16 {
    let mut max_m = 0u16;

    for ml in &bms.messages.measure_lengths {
        max_m = max_m.max(ml.measure);
    }

    // 遍历所有事件类型，统一提取 position.measure()。
    macro_rules! track_positions {
        ($($events:expr),+ $(,)?) => {
            $(
                for ev in $events {
                    max_m = max_m.max(ev.position.measure());
                }
            )+
        };
    }

    track_positions!(
        &bms.messages.note_events,
        &bms.messages.long_note_events,
        &bms.messages.bgm_events,
        &bms.messages.mine_events,
        &bms.messages.bpm_changes,
        &bms.messages.stop_events,
        &bms.messages.scroll_events,
        &bms.messages.speed_events,
        &bms.messages.bga_events,
        &bms.messages.stp_events,
    );

    // non_event_data 存储 NonEventData，遍历 measure。
    for item in &bms.messages.non_event_data {
        max_m = max_m.max(item.measure);
    }

    max_m.max(1) + 1
}

/// 构建 WAV 音频素材并返回查找表与素材向量。
fn build_audio_assets(audio: &Audio) -> (BTreeMap<WavIndex, AudioIndex>, Vec<AudioAsset>) {
    let mut wav_map = BTreeMap::new();
    let mut audio_assets = Vec::new();
    for (&wav_id, path) in &audio.wav_files {
        #[expect(clippy::cast_possible_truncation, reason = "WAV count fits in u32")]
        let idx = AudioIndex::new(audio_assets.len() as u32);
        let resolved_path = audio.path_wav.as_ref().map_or_else(
            || PathBuf::from(path),
            |prefix| Path::new(prefix).join(path),
        );
        audio_assets.push(AudioAsset {
            path: Arc::from(resolved_path),
            start: Duration::ZERO,
            duration: None,
        });
        wav_map.insert(wav_id, idx);
    }
    (wav_map, audio_assets)
}

/// [`build_bmp_map`](BmsConverter::build_bmp_map) 产出的条目：稠密资源 id、
/// 源文件路径与可选裁剪定义。
struct BmpEntry {
    /// 0 基资源 id（对应 `bga_resources` 的向量下标）。
    resource_id: BgaResourceId,
    /// 源图片文件路径（裁剪定义指向的底层 `#BMP`）。
    path: String,
    /// 裁剪与放置定义（`#BGA` / `#@BGA`）；无则为 `None`。
    crop: Option<CropRect>,
}

/// 将十进制 BMP 编号解析为 `#BMP` 表中的文件路径。
///
/// `#BGA` / `#@BGA` 的 `bmp_index` 字段是源 `#BMP` 的**十进制**编号；
/// 此处在 `bmp_files` 中查找 base36 数值与之相等的键（如 `"01"` → 1）。
fn resolve_bmp_path(bmp_files: &BTreeMap<BmpIndex, String>, bmp_index: u16) -> Option<&String> {
    bmp_files
        .iter()
        .find_map(|(k, v)| (k.to_index() == Some(bmp_index)).then_some(v))
}

/// 构建 LN 区间查找表。
fn build_ln_ranges(paired_lns: &[PairedLn]) -> HashMap<(u8, u8), Vec<(u64, u64)>> {
    let mut ln_ranges: HashMap<(u8, u8), Vec<(u64, u64)>> = HashMap::new();
    for ln in paired_lns {
        ln_ranges
            .entry((ln.player, ln.lane))
            .or_default()
            .push((ln.tick, ln.end_tick));
    }
    ln_ranges
}

/// 从事件流推断可玩按键总数。
///
/// 每侧取 `Lane::Key` 的最大编号，再把两侧相加；没有任何按键音符时
/// 返回 `None`。刮盘、踏板与地雷不计入按键数。
fn key_count(events: &[BmsEvent]) -> Option<u8> {
    let mut p1 = 0u8;
    let mut p2 = 0u8;

    for event in events {
        if let EventKind::Note {
            side,
            lane: Lane::Key(key),
            ..
        } = &event.kind
        {
            let slot = if side.as_u8() == NoteSide::P1.as_u8() {
                &mut p1
            } else {
                &mut p2
            };
            *slot = (*slot).max(key.get());
        }
    }

    let total = p1.saturating_add(p2);
    (total > 0).then_some(total)
}

/// 判断 visible note 是否应被 LN 区间抑制。
fn is_note_suppressed(
    player: u8,
    lane: u8,
    tick: u64,
    ln_ranges: &HashMap<(u8, u8), Vec<(u64, u64)>>,
    unpaired_starts: &[(u8, u8, u64)],
) -> bool {
    if let Some(ranges) = ln_ranges.get(&(player, lane)) {
        for &(start, end) in ranges {
            if tick >= start && tick <= end {
                return true;
            }
        }
    }
    unpaired_starts.contains(&(player, lane, tick))
}

/// 从预计算的小节脉冲表构建对齐的小节事件。
///
/// 适配变拍号（`#xxx02`）：每小节的小节线位于其实际起始脉冲位置，
/// 而非强制等步长 4/4 间距。
fn build_bar_events(table: &MeasureTable) -> Vec<BmsEvent> {
    table
        .bar_ticks()
        .iter()
        .map(|&tick| Event::bar(tick))
        .collect()
}

/// 将地雷原始 base36 值转换为伤害值。
///
/// - `Some(1295)`（`"ZZ"`）→ 即死（`f64::INFINITY`）
/// - `Some(n)` → `n / 2.0`
/// - `None`（解码失败）→ 默认 `1.0`
fn mine_damage(raw: Option<u16>) -> f64 {
    match raw {
        None => 1.0,
        Some(1295) => f64::INFINITY,
        Some(n) => f64::from(n) / 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_kill_value_produces_infinite_damage() {
        assert!(mine_damage(Some(1295)).is_infinite());
    }

    #[test]
    fn normal_value_produces_half_damage() {
        assert!((mine_damage(Some(1)) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn undecodable_value_produces_default_damage() {
        assert!((mine_damage(None) - 1.0).abs() < f64::EPSILON);
    }
}
