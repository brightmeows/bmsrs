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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bms_parser::{Bms, BpmValue, KeyType};
use bms_tokenizer::{
    BmpIndex, BmsChannel as RawChannel, ChangeOptionIndex, LnMode, SeekIndex, WavIndex,
};
use bmsrs_chart::{
    AudioAsset, BgaResource, BpmChange, BpmLookup, Chart, ChartData, ChartInfo, CropRect, Damage,
    Event, EventKind, LnJudgeHint, LnLifeHint, LnTypeHint, NoteKind, SongInfo, StopEvent,
    TimingTrack, VideoAsset,
};
use thiserror::Error;

pub use crate::error::ProcessWarning;

use crate::custom_event::BmsCustomEvent;
use crate::layout::{Bme, BmsChannel, BmsLayout, Pms, PmsBme, PmsLayout};

use crate::long_note::{LnPairingResult, PairedLn, pair_lnobj, pair_lntype1, pair_lntype2};
use crate::position::MeasureTable;

/// BMS 处理期间可能发生的错误。
#[derive(Debug, Error)]
pub enum ProcessError {
    /// 初始 BPM 无效（零、NaN 或无穷大；允许负值用于逆走谱面）。
    #[error("init_bpm must be non-zero finite, got {0}")]
    InvalidBpm(f64),
}

/// BMS 处理器产出的统一事件类型。
///
/// 泛型参数固定为 `((), BmsCustomEvent)`，其中 `BmsCustomEvent`
/// 承载引擎特定事件（BGA 不透明度、ARGB、TEXT 等）。
type BmsEvent = Event<(), BmsCustomEvent>;

/// 将 [`Bms`] 转换为 [`Chart`] 的零大小处理器。
pub struct BmsProcessor;

/// BMS 谱面的默认节拍分辨率（每四分音符的脉冲数）。
const RESOLUTION: u64 = 240;

impl BmsProcessor {
    /// 使用显式模式族布局处理 BMS 谱面，并收集处理警告。
    ///
    /// 布局类型决定每个 BMS `(player, lane)` 通道字节如何解码为音符
    /// 位置。可用族见 `layout` 模块。
    ///
    /// # Errors
    ///
    /// 若初始 BPM 缺失或为零/非有限值，返回 [`ProcessError::InvalidBpm`]。
    pub fn process_with_warnings<L>(
        bms: &Bms,
    ) -> Result<(Chart<(), BmsCustomEvent>, Vec<ProcessWarning>), ProcessError>
    where
        L: BmsLayout,
    {
        let init_bpm = bms.timing.bpm.unwrap_or(130.0);

        let max_measure = find_max_measure(bms);
        let table = MeasureTable::new(max_measure, &bms.messages.measure_lengths, RESOLUTION);
        let mut conv = BmsConverter {
            bms,
            table: &table,
            events: Vec::with_capacity(1024),
            warnings: Vec::new(),
        };

        let (wav_map, audio_assets) = build_audio_assets(&bms.audio.wav_files);
        let bpm_changes = conv.build_bpm_changes();
        let bpm_lookup = BpmLookup::new(init_bpm, &bpm_changes);

        let mut stops = conv.build_stops_from_defs();
        stops.extend(conv.build_stops_from_stp(&bpm_lookup));
        stops.sort_by_key(|s| s.tick);

        let timing = TimingTrack::new(init_bpm, bpm_changes, stops, RESOLUTION)
            .map_err(|e| ProcessError::InvalidBpm(e.bpm()))?;

        let bmp_map = conv.build_bmp_map();

        let (ln_result, consumed) = conv.pair_long_notes();
        let paired_lns = ln_result.paired;
        let unpaired_starts = ln_result.unpaired_starts;

        // 收集未配对长音警告。
        for &(player, lane, _) in &unpaired_starts {
            conv.warnings
                .push(ProcessWarning::UnterminatedLongNote { player, lane });
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

        // 排序规则收敛于 Event::sort_key（同脉冲子序约定见其文档）。
        conv.events.sort_by_key(BmsEvent::sort_key);

        let (song, chart_info) = conv.build_metadata(&bmp_map);
        let ln_type_hint = conv.ln_type_hint();

        let mut data = ChartData {
            resolution: RESOLUTION,
            timing,
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            ln_type_hint,
            ln_judge_hint: LnJudgeHint::default(),
            ln_life_hint: LnLifeHint::default(),
            judge_deltas: None,
            life_deltas: None,
            events: conv.events,
            audio_assets,
        };
        data.sort_events();

        Ok((
            Chart {
                song,
                chart: chart_info,
                data,
            },
            conv.warnings,
        ))
    }

    /// 使用显式模式族布局处理 BMS 谱面。
    ///
    /// 布局类型决定每个 BMS `(player, lane)` 通道字节如何解码为音符
    /// 位置。可用族见 `layout` 模块。
    ///
    /// # Errors
    ///
    /// 若初始 BPM 缺失或为零/非有限值，返回 [`ProcessError::InvalidBpm`]。
    pub fn process<L>(bms: &Bms) -> Result<Chart<(), BmsCustomEvent>, ProcessError>
    where
        L: BmsLayout,
    {
        Self::process_with_warnings::<L>(bms).map(|(chart, _)| chart)
    }

    /// 使用自动检测的 PMS 变体布局处理 BMS 谱面。
    ///
    /// 根据谱面中使用的通道自动选择 [`Pms`]（Standard）或 [`PmsBme`]（BME-type）。
    ///
    /// # Errors
    ///
    /// 若初始 BPM 缺失或为零/非有限值，返回 [`ProcessError::InvalidBpm`]。
    pub fn process_pms(bms: &Bms) -> Result<Chart<(), BmsCustomEvent>, ProcessError> {
        match PmsLayout::detect(bms) {
            PmsLayout::Standard => Self::process::<Pms>(bms),
            PmsLayout::BmeType => Self::process::<PmsBme>(bms),
        }
    }

    /// 使用默认的 [`Bme`] 布局处理 BMS 谱面。
    ///
    /// `Bme` 统一覆盖 beat-5k/7k/10k/14k —— 两侧玩家均被映射，因此单人
    /// 谱面只是让 2P 轨道空置，双人谱面则填充它们。`#PLAYER` 头部命令不
    /// 影响映射（与现代引擎一致，均忽略该字段）。
    ///
    /// 若需 PMS、nanasi 或其他族，请以相应布局类型调用
    /// [`process::<Nanasi>`](Self::process)。
    ///
    /// # Errors
    ///
    /// 若初始 BPM 缺失或为零/非有限值，返回 [`ProcessError::InvalidBpm`]。
    pub fn process_default(bms: &Bms) -> Result<Chart<(), BmsCustomEvent>, ProcessError> {
        Self::process::<Bme>(bms)
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
        if self.bms.gameplay.ln_type == Some(bms_tokenizer::LnType::Type2) {
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
    fn collect_notes<L: BmsLayout>(
        &mut self,
        wav_map: &BTreeMap<WavIndex, u32>,
        paired_lns: &[PairedLn],
        consumed: &HashSet<usize>,
        unpaired_starts: &[(u8, u8, u64)],
    ) {
        let push_note =
            |tick: u64, side, lane, kind: NoteKind, audio: Option<u32>, ev: &mut Vec<BmsEvent>| {
                if let Some((note_side, note_lane)) =
                    BmsChannel::new(side, lane).and_then(L::map_channel)
                {
                    ev.push(Event::new(
                        tick,
                        EventKind::Note {
                            side: note_side,
                            lane: note_lane,
                            kind,
                            audio_index: audio,
                            ext: (),
                        },
                    ));
                }
            };

        let mut check_wav = |wav_id: &WavIndex| -> Option<u32> {
            let audio = wav_map.get(wav_id).copied();
            // `to_index()` 返回 `None` 时静默跳过警告：非可分解的 WAV 键
            // （如 "ZZ" LNOBJ 标记）不属于 WAV 表定义的范围，不产生
            // 缺失定义警告。
            if audio.is_none()
                && let Some(key) = wav_id.0.to_index()
            {
                self.warnings
                    .push(ProcessWarning::MissingWavDefinition { key });
            }
            audio
        };

        // 构建 LN 区间表：(player, lane) → Vec<(start_tick, end_tick)>。
        let ln_ranges = build_ln_ranges(paired_lns);

        // 可见音符（跳过已消耗的 LNOBJ 配对与 LN 区间内的音符）。
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i) {
                continue;
            }
            if ne.key_type == KeyType::Visible {
                let tick = self.table.position_to_tick(ne.position);
                if is_note_suppressed(ne.player, ne.lane, tick, &ln_ranges, unpaired_starts) {
                    continue;
                }
                push_note(
                    tick,
                    ne.player,
                    ne.lane,
                    NoteKind::Normal,
                    check_wav(&ne.wav_id),
                    &mut self.events,
                );
            }
        }

        // 不可见音符（按键音，跳过已消耗的 LNOBJ 配对）。
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i) {
                continue;
            }
            if ne.key_type == KeyType::Invisible {
                push_note(
                    self.table.position_to_tick(ne.position),
                    ne.player,
                    ne.lane,
                    NoteKind::Invisible,
                    check_wav(&ne.wav_id),
                    &mut self.events,
                );
            }
        }

        // LNOBJ 模式下 ch51-69 长音通道与 LNOBJ 互斥（memo/10 规范未定义）；
        // 不静默丢弃，作为普通可见音符保留数据。
        if self.bms.gameplay.ln_obj.is_some() {
            for lne in &self.bms.messages.long_note_events {
                push_note(
                    self.table.position_to_tick(lne.position),
                    lne.player,
                    lne.lane,
                    NoteKind::Normal,
                    check_wav(&lne.wav_id),
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
                check_wav(&ln.wav_id),
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
                    damage: Damage::new(me.damage),
                },
                None,
                &mut self.events,
            );
        }
    }

    /// 收集 BGM 事件到事件向量中。
    fn collect_bgm(&mut self, wav_map: &BTreeMap<WavIndex, u32>) {
        for be in &self.bms.messages.bgm_events {
            if let Some(&audio) = wav_map.get(&be.wav_id) {
                self.events
                    .push(Event::bgm(self.table.position_to_tick(be.position), audio));
            }
        }
    }

    /// 收集 LNOBJ 终点标记的 BGM 事件。
    ///
    /// 按 BMS 规范，当 [`#LNOBJ`](bms_tokenizer::BmsHeaderGameplay::LnObj)
    /// 终点标记经过判定线时，其 WAV 文件作为 BGM 播放。此函数遍历
    /// [`pair_lnobj`] 返回的 `consumed` 音符索引，为每个终点标记生成一个
    /// BGM 事件。
    fn collect_lnobj_bgm(&mut self, wav_map: &BTreeMap<WavIndex, u32>, consumed: &HashSet<usize>) {
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
    /// 值为 0 的 BPM 变更被跳过并生成警告。
    fn collect_bpm_events(&mut self) {
        for bc in &self.bms.messages.bpm_changes {
            let tick = self.table.position_to_tick(bc.position);
            let bpm = self.resolve_bpm(bc.value);
            if bpm == 0.0 {
                self.warnings
                    .push(ProcessWarning::ZeroBpmChangeIgnored { tick });
            } else {
                self.events.push(Event::bpm(tick, bpm));
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
    /// **归一化保证**：parser 层已按 `detected_base` 完成所有索引归一化，
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
                if obj == "00" {
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
                        let Ok(opacity) = u8::from_str_radix(obj, 16) else {
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
                        let Some((a, r, g, b)) = BmpIndex::try_from(obj.as_str())
                            .ok()
                            .and_then(|idx| self.bms.visual.argb_defs.get(&idx).cloned())
                            .map(|p| (p.a, p.r, p.g, p.b))
                        else {
                            continue;
                        };
                        BmsCustomEvent::BgaArgb { layer, a, r, g, b }
                    }
                    RawChannel::BgaKeyBound => {
                        // 值已归一化，直接查 bmp_map。
                        let Some(resource_id) = BmpIndex::try_from(obj.as_str())
                            .ok()
                            .and_then(|idx| bmp_map.get(&idx).map(|e| e.resource_id))
                        else {
                            continue;
                        };
                        BmsCustomEvent::BgaKeyBound { resource_id }
                    }
                    RawChannel::Text => {
                        let index = u64::from_str_radix(obj, 36).unwrap_or(0);
                        BmsCustomEvent::TextDisplay {
                            text_index: index as u32,
                        }
                    }
                    RawChannel::Judge => {
                        let index = u64::from_str_radix(obj, 36).unwrap_or(0);
                        BmsCustomEvent::JudgeOverride { rank: index }
                    }
                    RawChannel::Option => {
                        let option_id = u64::from_str_radix(obj, 36).unwrap_or(0);
                        // 值已归一化，直接查 change_option_defs（F1+F2 已在 parser 解决）。
                        let opt_idx = ChangeOptionIndex::try_from(obj.as_str()).ok();
                        let value = opt_idx
                            .and_then(|id| self.bms.gameplay.change_option_defs.get(&id).cloned())
                            .unwrap_or_default();
                        BmsCustomEvent::OptionChange { option_id, value }
                    }
                    RawChannel::BgmVolume => {
                        let Ok(volume) = u8::from_str_radix(obj, 16) else {
                            continue;
                        };
                        BmsCustomEvent::BgmVolume { volume }
                    }
                    RawChannel::KeyVolume => {
                        let Ok(volume) = u8::from_str_radix(obj, 16) else {
                            continue;
                        };
                        BmsCustomEvent::KeyVolume { volume }
                    }
                    RawChannel::Seek => {
                        // 值已归一化，直接查 seek_defs。
                        let Some(ms) = SeekIndex::try_from(obj.as_str())
                            .ok()
                            .and_then(|idx| self.bms.visual.seek_defs.get(&idx).copied())
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
                        layer: be.layer,
                        resource_id: entry.resource_id,
                    },
                ));
            }
        }
    }

    /// 为计时轨构建 BPM 变更事件。
    ///
    /// 相邻同值 BPM 变更会被去重（首个保留）。
    fn build_bpm_changes(&mut self) -> Vec<BpmChange> {
        let mut changes: Vec<BpmChange> = self
            .bms
            .messages
            .bpm_changes
            .iter()
            .filter_map(|bc| {
                let tick = self.table.position_to_tick(bc.position);
                let bpm = self.resolve_bpm(bc.value);
                if bpm == 0.0 {
                    None
                } else {
                    Some(BpmChange { tick, bpm })
                }
            })
            .collect();
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
                    resource_id: i as u32,
                    path,
                    crop,
                },
            );
        }

        map
    }

    /// 将 [`BpmValue`] 解析为具体 BPM 值，对引用值使用 BPM 定义表。
    fn resolve_bpm(&mut self, value: BpmValue) -> f64 {
        match value {
            BpmValue::Absolute(bpm) => bpm,
            BpmValue::Reference(id) => {
                if let Some(bpm) = self.bms.timing.bpm_defs.get(&id).copied() {
                    bpm
                } else {
                    if let Some(key) = id.to_index() {
                        self.warnings
                            .push(ProcessWarning::MissingBpmDefinition { key });
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
                            computed_duration: duration,
                        });
                    }
                    Some(StopEvent {
                        tick,
                        duration: duration.max(0.0) as u64,
                    })
                } else {
                    if let Some(key) = se.stop_id.to_index() {
                        self.warnings
                            .push(ProcessWarning::MissingStopDefinition { key });
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
                id: e.resource_id,
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
                subartists: self
                    .bms
                    .metadata
                    .sub_artist
                    .as_deref()
                    .map(|s| vec![s.to_owned()])
                    .unwrap_or_default(),
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
                back_image: self.bms.display.back_bmp.clone(),
                eyecatch_image: self.bms.display.stage_file.clone(),
                banner_image: self.bms.display.banner.clone(),
                preview_music: self.bms.display.preview.clone(),
                bga_resources,
                video,
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

// 纯转换辅助函数（不依赖 `&Bms` + `&MeasureTable` 组合）

/// 查找任意事件所引用的最大小节号。
///
/// 在 [`BmsConverter`] 构造前调用——其结果是 [`MeasureTable::new`] 的输入。
fn find_max_measure(bms: &Bms) -> u16 {
    let mut max_m = 0u16;

    for ml in &bms.messages.measure_lengths {
        max_m = max_m.max(ml.measure);
    }

    // 遍历所有事件类型，统一提取 position.measure。
    macro_rules! track_positions {
        ($($events:expr),+ $(,)?) => {
            $(
                for ev in $events {
                    max_m = max_m.max(ev.position.measure);
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
fn build_audio_assets(
    wav_files: &BTreeMap<WavIndex, String>,
) -> (BTreeMap<WavIndex, u32>, Vec<AudioAsset>) {
    let mut wav_map = BTreeMap::new();
    let mut audio_assets = Vec::new();
    for (&wav_id, path) in wav_files {
        #[expect(clippy::cast_possible_truncation, reason = "WAV count fits in u32")]
        let idx = audio_assets.len() as u32;
        audio_assets.push(AudioAsset {
            path: Arc::from(PathBuf::from(path.as_str())),
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
    /// 0 基资源 id（对应 [`BgaResource::id`]）。
    resource_id: u32,
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
