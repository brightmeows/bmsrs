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

mod long_note;
mod position;

pub mod layout;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bms_parser::{Bms, BpmValue, KeyType};
use bms_tokenizer::{BmpIndex, WavIndex};
use bmsrs_chart::{
    AudioAsset, BgaResource, BpmChange, Chart, ChartData, ChartInfo, Damage, Event, LnJudgeHint,
    LnLifeHint, LnTypeHint, NoteKind, SongInfo, StopEvent, TimingTrack,
};
use thiserror::Error;

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
/// BMS 格式既无每音符扩展也无自定义事件，故泛型参数固定为
/// `((), NoCustomEvent)`。集中于此以便内部函数统一引用，避免冗长的
/// 全限定签名重复。
type BmsEvent = Event<(), bmsrs_chart::NoCustomEvent>;

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
    /// # Errors
    ///
    /// 若初始 BPM 缺失或非正数，返回 [`ProcessError::InvalidBpm`]。
    pub fn process<L>(bms: &Bms) -> Result<Chart<(), bmsrs_chart::NoCustomEvent>, ProcessError>
    where
        L: BmsLayout,
    {
        let init_bpm = bms.timing.bpm.unwrap_or(130.0);
        if init_bpm.is_nan() || init_bpm == 0.0 {
            return Err(ProcessError::InvalidBpm(init_bpm));
        }

        let max_measure = find_max_measure(bms);
        let table = MeasureTable::new(max_measure, &bms.messages.measure_lengths, RESOLUTION);
        let conv = BmsConverter { bms, table: &table };

        let (wav_map, audio_assets) = build_audio_assets(&bms.audio.wav_files);
        let bpm_changes = conv.build_bpm_changes();

        let mut stops = conv.build_stops_from_defs();
        stops.extend(conv.build_stops_from_stp(&bpm_changes, init_bpm));
        stops.sort_by_key(|s| s.tick);

        let timing = TimingTrack::new(init_bpm, bpm_changes, stops);

        let bmp_map = build_bmp_map(&bms.visual.bmp_files);
        let bga_resources = bmp_map
            .values()
            .map(|(resource_id, path)| BgaResource {
                id: *resource_id,
                path: path.clone().into(),
            })
            .collect();

        let (paired_lns, consumed) = conv.pair_long_notes();
        let mut events = Vec::new();

        // 小节线优先（优先级 0）。
        events.extend(build_bar_events(&table));

        // 音符（优先级 1）。
        conv.collect_notes::<L>(&wav_map, &paired_lns, &consumed, &mut events);

        // BGM（优先级 1）。
        conv.collect_bgm(&wav_map, &mut events);

        // LNOBJ 终点标记作为 BGM 播放（按 BMS 规范）。
        conv.collect_lnobj_bgm(&wav_map, &consumed, &mut events);

        // BGA（优先级 1）。
        conv.collect_bga(&bmp_map, &mut events);

        // BPM 变更（优先级 2）。
        conv.collect_bpm_events(&mut events);

        // 停止事件（优先级 3）—— 重新遍历计时停止事件。
        for se in &timing.stops {
            events.push(Event::Stop {
                tick: se.tick,
                duration: se.duration,
            });
        }

        // SCROLL 事件（优先级 4）。
        conv.collect_scroll_events(&mut events);

        // SPEED 事件（优先级 5）。
        conv.collect_speed_events(&mut events);

        // 排序规则收敛于 Event::sort_key（同脉冲子序约定见其文档）。
        events.sort_by_key(BmsEvent::sort_key);

        let (song, chart_info) = conv.build_metadata();

        Ok(Chart {
            song,
            chart: ChartInfo {
                bga_resources,
                ..chart_info
            },
            data: ChartData {
                resolution: RESOLUTION,
                timing,
                judge_multiplier: 1.0,
                life_multiplier: 1.0,
                ln_type_hint: LnTypeHint::default(),
                ln_judge_hint: LnJudgeHint::default(),
                ln_life_hint: LnLifeHint::default(),
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
    pub fn process_default(
        bms: &Bms,
    ) -> Result<Chart<(), bmsrs_chart::NoCustomEvent>, ProcessError> {
        Self::process::<Bme>(bms)
    }
}

/// BMS → [`Chart`] 转换上下文。
///
/// 持有在多个转换步骤间共享的源文档 [`Bms`] 与预计算的小节脉冲表
/// [`MeasureTable`]，使各步骤以方法形式组织，避免 `&Bms` 与
/// `&MeasureTable` 在每个辅助函数签名中重复。仅供
/// [`BmsProcessor::process`] 内部使用。
struct BmsConverter<'a> {
    /// 源 BMS 文档。
    bms: &'a Bms,
    /// 小节 → 累计脉冲查找表。
    table: &'a MeasureTable,
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
        &self,
        wav_map: &BTreeMap<WavIndex, u32>,
        paired_lns: &[PairedLn],
        consumed: &BTreeSet<usize>,
        events: &mut Vec<BmsEvent>,
    ) {
        let push_note =
            |tick: u64, side, lane, kind: NoteKind, audio: Option<u32>, ev: &mut Vec<BmsEvent>| {
                if let Some((note_side, note_lane)) =
                    BmsChannel::new(side, lane).and_then(L::map_channel)
                {
                    ev.push(Event::Note {
                        tick,
                        side: note_side,
                        lane: note_lane,
                        kind,
                        audio_index: audio,
                        ext: (),
                    });
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
                    events,
                );
            }
        }

        // 不可见音符（按键音）。
        for ne in &self.bms.messages.note_events {
            if ne.key_type == KeyType::Invisible {
                push_note(
                    self.table.position_to_tick(ne.position),
                    ne.player,
                    ne.lane,
                    NoteKind::Invisible,
                    wav_map.get(&ne.wav_id).copied(),
                    events,
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
                events,
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
                events,
            );
        }
    }

    /// 收集 BGM 事件到事件向量中。
    fn collect_bgm(&self, wav_map: &BTreeMap<WavIndex, u32>, events: &mut Vec<BmsEvent>) {
        for be in &self.bms.messages.bgm_events {
            if let Some(&audio) = wav_map.get(&be.wav_id) {
                events.push(Event::Bgm {
                    tick: self.table.position_to_tick(be.position),
                    audio_index: audio,
                });
            }
        }
    }

    /// 收集 LNOBJ 终点标记的 BGM 事件。
    ///
    /// 按 BMS 规范，当 [`#LNOBJ`](bms_tokenizer::BmsHeaderGameplay::LnObj)
    /// 终点标记经过判定线时，其 WAV 文件作为 BGM 播放。此函数遍历
    /// [`pair_lnobj`] 返回的 `consumed` 音符索引，为每个终点标记生成一个
    /// BGM 事件。
    fn collect_lnobj_bgm(
        &self,
        wav_map: &BTreeMap<WavIndex, u32>,
        consumed: &BTreeSet<usize>,
        events: &mut Vec<BmsEvent>,
    ) {
        let Some(ln_obj) = self.bms.gameplay.ln_obj else {
            return;
        };
        for (i, ne) in self.bms.messages.note_events.iter().enumerate() {
            if consumed.contains(&i)
                && *ne.wav_id == *ln_obj
                && let Some(&audio) = wav_map.get(&ne.wav_id)
            {
                events.push(Event::Bgm {
                    tick: self.table.position_to_tick(ne.position),
                    audio_index: audio,
                });
            }
        }
    }

    /// 构建 BPM 事件（用于统一时间线）。
    fn collect_bpm_events(&self, events: &mut Vec<BmsEvent>) {
        for bc in &self.bms.messages.bpm_changes {
            events.push(Event::Bpm {
                tick: self.table.position_to_tick(bc.position),
                bpm: self.resolve_bpm(bc.value),
            });
        }
    }

    /// 构建 SCROLL 变更事件。
    fn collect_scroll_events(&self, events: &mut Vec<BmsEvent>) {
        for se in &self.bms.messages.scroll_events {
            if let Some(&rate) = self.bms.timing.scroll_defs.get(&se.scroll_id) {
                events.push(Event::Scroll {
                    tick: self.table.position_to_tick(se.position),
                    rate,
                });
            }
        }
    }

    /// 构建 SPEED（视觉音符间距）关键帧事件。
    fn collect_speed_events(&self, events: &mut Vec<BmsEvent>) {
        for se in &self.bms.messages.speed_events {
            if let Some(&rate) = self.bms.timing.speed_defs.get(&se.speed_id) {
                events.push(Event::Speed {
                    tick: self.table.position_to_tick(se.position),
                    rate,
                });
            }
        }
    }

    /// 从 BGA 事件与 BMP 文件定义构建 BGA 事件。
    fn collect_bga(&self, bmp_map: &BTreeMap<BmpIndex, (u32, String)>, events: &mut Vec<BmsEvent>) {
        for be in &self.bms.messages.bga_events {
            if let Some(&(resource_id, _)) = bmp_map.get(&be.bmp_id) {
                events.push(Event::Bga {
                    tick: self.table.position_to_tick(be.position),
                    layer: be.layer,
                    resource_id,
                });
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

    /// 从 BMS 元数据构建 [`SongInfo`] 与 [`ChartInfo`]。
    #[expect(clippy::cast_possible_truncation, reason = "play level fits in u64")]
    #[expect(clippy::cast_sign_loss, reason = "play level is non-negative")]
    fn build_metadata(&self) -> (SongInfo, ChartInfo) {
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
                ..ChartInfo::default()
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
        .map(|&tick| Event::Bar { tick })
        .collect()
}
