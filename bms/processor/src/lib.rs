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
mod long_note;
mod position;

pub mod layout;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bms_parser::{Bms, BpmValue, KeyType, split_2char_values_lenient};
use bms_tokenizer::{BmpIndex, BmsChannel as RawChannel, WavIndex};
use bmsrs_chart::{
    AudioAsset, BgaResource, BpmChange, Chart, ChartData, ChartInfo, Damage, Event, EventKind,
    LnJudgeHint, LnLifeHint, LnTypeHint, NoteKind, SongInfo, StopEvent, TimingTrack,
};
use thiserror::Error;

use crate::custom_event::BmsCustomEvent;
use crate::layout::{Bme, BmsChannel, BmsLayout};

use crate::long_note::{PairedLn, pair_lnobj, pair_lntype1, pair_lntype2};
use crate::position::MeasureTable;

/// BMS 处理期间可能发生的错误。
#[derive(Debug, Error)]
pub enum ProcessError {
    /// 初始 BPM 缺失或无效。
    #[error("init_bpm must be positive, got {0}")]
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
    /// 使用显式模式族布局处理 BMS 谱面。
    ///
    /// 布局类型决定每个 BMS `(player, lane)` 通道字节如何解码为音符
    /// 位置。可用族见 `layout` 模块。
    ///
    /// # Panics
    ///
    /// 若 `init_bpm` 为无效有限值（不应发生，因上方已通过验证），
    /// `TimingTrack::new` 会 panic。
    ///
    /// # Errors
    ///
    /// 若初始 BPM 缺失或非正数，返回 [`ProcessError::InvalidBpm`]。
    #[expect(
        clippy::expect_used,
        clippy::unwrap_in_result,
        reason = "init_bpm was already validated above"
    )]
    pub fn process<L>(bms: &Bms) -> Result<Chart<(), BmsCustomEvent>, ProcessError>
    where
        L: BmsLayout,
    {
        let init_bpm = bms.timing.bpm.unwrap_or(130.0);
        if !init_bpm.is_finite() || init_bpm == 0.0 {
            return Err(ProcessError::InvalidBpm(init_bpm));
        }

        let max_measure = find_max_measure(bms);
        let table = MeasureTable::new(max_measure, &bms.messages.measure_lengths, RESOLUTION);
        let mut conv = BmsConverter {
            bms,
            table: &table,
            events: Vec::with_capacity(1024),
        };

        let (wav_map, audio_assets) = build_audio_assets(&bms.audio.wav_files);
        let bpm_changes = conv.build_bpm_changes();

        let mut stops = conv.build_stops_from_defs();
        stops.extend(conv.build_stops_from_stp(&bpm_changes, init_bpm));
        stops.sort_by_key(|s| s.tick);

        let timing = TimingTrack::new(init_bpm, bpm_changes, stops)
            .expect("init_bpm was already validated above");

        let bmp_map = build_bmp_map(&bms.visual.bmp_files);

        let (paired_lns, consumed) = conv.pair_long_notes();

        // 小节线优先（优先级 0）。
        conv.events.extend(build_bar_events(&table));

        // 音符（优先级 1）。
        conv.collect_notes::<L>(&wav_map, &paired_lns, &consumed);

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
        let events = conv.events;

        Ok(Chart {
            song,
            chart: chart_info,
            data: ChartData {
                resolution: RESOLUTION,
                timing,
                judge_multiplier: 1.0,
                life_multiplier: 1.0,
                ln_type_hint: LnTypeHint::default(),
                ln_judge_hint: LnJudgeHint::default(),
                ln_life_hint: LnLifeHint::default(),
                judge_deltas: None,
                life_deltas: None,
                events,
                audio_assets,
            },
        })
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
    /// 若初始 BPM 缺失或非正数，返回 [`ProcessError::InvalidBpm`]。
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
}

impl BmsConverter<'_> {
    /// 判定 LN 模式并配对长音。返回配对后的长音与已消耗的音符索引。
    fn pair_long_notes(&self) -> (Vec<PairedLn>, BTreeSet<usize>) {
        if let Some(ln_obj) = self.bms.gameplay.ln_obj {
            return pair_lnobj(&self.bms.messages.note_events, ln_obj, self.table);
        }
        if self.bms.gameplay.ln_type == Some(bms_tokenizer::LnType::Type2) {
            return (
                pair_lntype2(&self.bms.messages.long_note_events, self.table),
                BTreeSet::new(),
            );
        }
        (
            pair_lntype1(&self.bms.messages.long_note_events, self.table),
            BTreeSet::new(),
        )
    }

    /// 收集全部可玩音符到事件向量中。
    fn collect_notes<L: BmsLayout>(
        &mut self,
        wav_map: &BTreeMap<WavIndex, u32>,
        paired_lns: &[PairedLn],
        consumed: &BTreeSet<usize>,
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

        // 可见音符（跳过已消耗的 LNOBJ 配对）。
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i) {
                continue;
            }
            if ne.key_type == KeyType::Visible {
                push_note(
                    self.table.position_to_tick(ne.position),
                    ne.player,
                    ne.lane,
                    NoteKind::Normal,
                    wav_map.get(&ne.wav_id).copied(),
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
                    wav_map.get(&ne.wav_id).copied(),
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
                    wav_map.get(&lne.wav_id).copied(),
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
                wav_map.get(&ln.wav_id).copied(),
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
    fn collect_lnobj_bgm(&mut self, wav_map: &BTreeMap<WavIndex, u32>, consumed: &BTreeSet<usize>) {
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
    fn collect_bpm_events(&mut self) {
        for bc in &self.bms.messages.bpm_changes {
            self.events.push(Event::bpm(
                self.table.position_to_tick(bc.position),
                self.resolve_bpm(bc.value),
            ));
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
    /// 从 [`Messages::non_event_data`] 读取由 parser 合并但未转换的通道数据，
    /// 转换为 [`EventKind::Custom`] 变体。每个非 `"00"` 值产生一个事件。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "channel values are single-byte bounded (0-255) or u32 resource indices"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "single structured match on channel variant with short branches; extraction would lose clarity"
    )]
    fn collect_custom_events(&mut self, bmp_map: &BTreeMap<BmpIndex, (u32, String)>) {
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

        for &(measure, ref channel, ref merged) in non_event {
            let objects = split_2char_values_lenient(merged);
            let total = objects.len() as u32;
            if total == 0 {
                continue;
            }

            for (i, obj) in objects.iter().enumerate() {
                if *obj == "00" {
                    continue;
                }
                // 值通道（0B-0E, 97, 98）使用十六进制（01-FF）；
                // 索引通道（99, A0, A1-A5, A6, 05）使用 Base36 索引。
                let tick = self
                    .table
                    .position_to_tick(bms_parser::Position::new(measure, i as u32, total));

                let payload = match channel {
                    RawChannel::BgaBaseOpacity
                    | RawChannel::BgaLayerOpacity
                    | RawChannel::BgaLayer2Opacity
                    | RawChannel::BgaPoorOpacity => {
                        let Some(layer) = layer_of(*channel) else {
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
                        let Some(layer) = layer_of(*channel) else {
                            continue;
                        };
                        let Some((a, r, g, b)) = bms_tokenizer::BmpIndex::try_from(*obj)
                            .ok()
                            .and_then(|idx| self.bms.visual.argb_defs.get(&idx).cloned())
                            .map(|p| (p.a, p.r, p.g, p.b))
                        else {
                            continue;
                        };
                        BmsCustomEvent::BgaArgb { layer, a, r, g, b }
                    }
                    RawChannel::BgaKeyBound => {
                        // 与 collect_bga 一致：经 bmp_map 查表得到 0 基枚举 id，
                        // 而非直接使用 base36 原值，使 resource_id 可在 bga_resources 中查到。
                        let Some(&(resource_id, _)) = bms_tokenizer::BmpIndex::try_from(*obj)
                            .ok()
                            .and_then(|idx| bmp_map.get(&idx))
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
                        let value = bms_tokenizer::ChangeOptionIndex::try_from(*obj)
                            .ok()
                            .and_then(|idx| self.bms.gameplay.change_option_defs.get(&idx).cloned())
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
                        let index = u64::from_str_radix(obj, 36).unwrap_or(0);
                        BmsCustomEvent::VideoSeek { position: index }
                    }
                    _ => continue,
                };
                self.events
                    .push(Event::new(tick, EventKind::Custom(payload)));
            }
        }
    }

    /// 从 BGA 事件与 BMP 文件定义构建 BGA 事件。
    fn collect_bga(&mut self, bmp_map: &BTreeMap<BmpIndex, (u32, String)>) {
        for be in &self.bms.messages.bga_events {
            if let Some(&(resource_id, _)) = bmp_map.get(&be.bmp_id) {
                self.events.push(Event::new(
                    self.table.position_to_tick(be.position),
                    EventKind::Bga {
                        layer: be.layer,
                        resource_id,
                    },
                ));
            }
        }
    }

    /// 为计时轨构建 BPM 变更事件。
    fn build_bpm_changes(&self) -> Vec<BpmChange> {
        self.bms
            .messages
            .bpm_changes
            .iter()
            .map(|bc| BpmChange {
                tick: self.table.position_to_tick(bc.position),
                bpm: self.resolve_bpm(bc.value),
            })
            .collect()
    }

    /// 将 [`BpmValue`] 解析为具体 BPM 值，对引用值使用 BPM 定义表。
    fn resolve_bpm(&self, value: BpmValue) -> f64 {
        match value {
            BpmValue::Absolute(bpm) => bpm,
            BpmValue::Reference(id) => self.bms.timing.bpm_defs.get(&id).copied().unwrap_or(120.0),
        }
    }

    /// 从 `#STOPxx` 定义（通道 `09`）构建停止事件。
    ///
    /// 负 STOP 值被钳位到 `0`（跳过/忽略），与 beatoraja/Angolmois 的行为一致。
    #[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    fn build_stops_from_defs(&self) -> Vec<StopEvent> {
        self.bms
            .messages
            .stop_events
            .iter()
            .filter_map(|se| {
                self.bms
                    .timing
                    .stop_defs
                    .get(&se.stop_id)
                    .map(|&raw| StopEvent {
                        tick: self.table.position_to_tick(se.position),
                        duration: (raw / 192.0 * RESOLUTION as f64 * 4.0).round().max(0.0) as u64,
                    })
            })
            .collect()
    }

    /// 从 `#STP` 头部命令构建停止事件（时长以毫秒计）。
    ///
    /// 负 STOP 值被钳位到 `0`（跳过/忽略），与 beatoraja/Angolmois 的行为一致。
    #[expect(clippy::cast_possible_truncation, reason = "stop duration fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "clamped to non-negative before cast")]
    #[expect(clippy::cast_precision_loss, reason = "resolution fits in f64")]
    fn build_stops_from_stp(&self, bpm_changes: &[BpmChange], init_bpm: f64) -> Vec<StopEvent> {
        self.bms
            .messages
            .stp_events
            .iter()
            .map(|stp| {
                let tick = self.table.position_to_tick(stp.position);
                let bpm = bpm_at_tick(bpm_changes, init_bpm, tick);
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
    fn build_metadata(&self, bmp_map: &BTreeMap<BmpIndex, (u32, String)>) -> (SongInfo, ChartInfo) {
        let bga_resources: Vec<BgaResource> = bmp_map
            .values()
            .map(|(resource_id, path)| BgaResource {
                id: *resource_id,
                path: path.clone().into(),
            })
            .collect();
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
            },
        )
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

    // non_event_data 存储 (measure, channel, merged_string)，遍历 measure。
    for (measure, _, _) in &bms.messages.non_event_data {
        max_m = max_m.max(*measure);
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

/// 构建 BMP 索引到（`resource_id`、文件路径）的映射。
fn build_bmp_map(bmp_files: &BTreeMap<BmpIndex, String>) -> BTreeMap<BmpIndex, (u32, String)> {
    let mut map = BTreeMap::new();
    #[expect(clippy::cast_possible_truncation, reason = "BMP count fits in u32")]
    for (i, (id, path)) in bmp_files.iter().enumerate() {
        map.insert(*id, (i as u32, path.clone()));
    }
    map
}

/// 查找给定脉冲处生效的 BPM（不晚于 `tick` 的最后一次 BPM 变更）。
///
/// `bpm_changes` 必须按 `tick` 升序排列（由
/// [`BmsConverter::build_bpm_changes`] 保证）。与 `bmsrs_player::TimingCache`
/// 的二分查找采用同一前提。
#[expect(
    clippy::indexing_slicing,
    reason = "idx ≥ 1 由 match 分支保证，idx-1 必在界内"
)]
fn bpm_at_tick(bpm_changes: &[BpmChange], init_bpm: f64, tick: u64) -> f64 {
    match bpm_changes.partition_point(|bc| bc.tick <= tick) {
        0 => init_bpm,
        idx => bpm_changes[idx - 1].bpm,
    }
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
