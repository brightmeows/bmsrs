//! BMS 通道消息数据的结构化类型。
//!
//! 本模块定义从 `#xxxYY:values` 行中提取的事件类型，以及同时持有原始
//! 拼接值与已解析事件向量的 [`Messages`] 容器。

// usize→u32 截断是安全的：BMS 每个通道每小节的值数 <4B。
// 见 event_pos 辅助函数。

use std::collections::BTreeMap;

use bms_tokenizer::{
    BmpIndex, BmsBase, BmsChannel, BpmIndex, ScrollIndex, SpeedIndex, StopIndex, WavIndex,
};
use bmsrs_chart::BgaLayer;

// 位置

/// BMS 小节内的位置，以分数 `numer / denom` 表示。
///
/// 对于一个小节内有 N 个值的通道，第 i 个值（从 0 开始）对应
/// `numer = i`、`denom = N`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// 小节编号（0–999）。
    pub measure: u16,
    /// 分子（此对象在小节内的索引）。
    pub numer: u32,
    /// 分母（此通道在此小节中的对象总数）。
    pub denom: u32,
}

impl Position {
    /// 创建一个新位置。
    #[inline]
    #[must_use]
    pub const fn new(measure: u16, numer: u32, denom: u32) -> Self {
        Self {
            measure,
            numer,
            denom,
        }
    }

    /// 返回小节内的小数位置，即 `numer / denom`。
    #[inline]
    #[must_use]
    pub fn fraction(self) -> f64 {
        if self.denom == 0 {
            0.0
        } else {
            f64::from(self.numer) / f64::from(self.denom)
        }
    }
}

// BGM

/// 一个 BGM（背景音乐）音符 —— 通道 `01`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgmEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 指向 `#WAV` 表的引用。
    pub wav_id: WavIndex,
}

// 可玩音符

/// 可玩音符是否在游玩区域可见。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// 可见音符（通道 `11`–`19`、`21`–`29`）。
    Visible,
    /// 不可见 / "key" 音符（通道 `31`–`39`、`41`–`49`）。
    Invisible,
}

/// 一个可玩音符（通道 `11`–`19`、`21`–`29`、`31`–`39`、`41`–`49`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 玩家编号（1 或 2）。
    pub player: u8,
    /// 轨道 / 按键编号（1–9）。
    pub lane: u8,
    /// 音符是否可见或不可见。
    pub key_type: KeyType,
    /// 指向 `#WAV` 表的引用。
    pub wav_id: WavIndex,
}

// 长音

/// 一个长音 / charge-note（通道 `51`–`59`、`61`–`69`）。
///
/// 长音类型（LN / CN / HCN）由谱面级的
/// [`LnType`](bms_tokenizer::LnType) 与 [`LnMode`](bms_tokenizer::LnMode)
/// 头部命令决定，而非按事件存储。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongNoteEvent {
    /// 小节内的位置（按住的起点）。
    pub position: Position,
    /// 玩家编号（1 或 2）。
    pub player: u8,
    /// 轨道 / 按键编号（1–9）。
    pub lane: u8,
    /// 指向 `#WAV` 表的引用。
    pub wav_id: WavIndex,
}

// 地雷

/// 一个地雷音符（通道 `D1`–`D9`、`E1`–`E9`）。
///
/// 伤害值由 BMS 双字符索引推导而来：`damage = base36_value / 2.0`，
/// 其中 `ZZ` = 即死。
#[derive(Debug, Clone, PartialEq)]
pub struct MineEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 玩家编号（1 或 2）。
    pub player: u8,
    /// 轨道 / 按键编号（1–9）。
    pub lane: u8,
    /// miss 时造成的伤害（0.0 = 无伤害，`f64::INFINITY` = 即死）。
    pub damage: f64,
}

// BPM 变更

/// BPM 变更事件的值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BpmValue {
    /// 绝对 BPM 值（通道 `03`）。
    Absolute(f64),
    /// 指向 `#BPMxx` 定义的引用（通道 `08`）。
    Reference(BpmIndex),
}

/// 一个 BPM 变更事件（通道 `03`、`08`）。
#[derive(Debug, Clone, PartialEq)]
pub struct BpmChange {
    /// 小节内的位置。
    pub position: Position,
    /// 新的 BPM 值或引用。
    pub value: BpmValue,
}

// 停止

/// 一个引用 `#STOPxx` 定义的停止 / 暂停事件（通道 `09`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 指向 `#STOP` 表的引用。
    pub stop_id: StopIndex,
}

// 滚动

/// 一个滚动速度倍率事件（通道 `SC`）。
///
/// 引用 `#SCROLLxx` 定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 指向 `#SCROLL` 表的引用。
    pub scroll_id: ScrollIndex,
}

/// 一个视觉音符间距关键帧事件（通道 `SP`）。
///
/// 引用 `#SPEEDxx` 定义。关键帧之间的间距因子通过线性插值计算。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 指向 `#SPEED` 表的引用。
    pub speed_id: SpeedIndex,
}

// BGA 事件

/// 一个 BGA 显示事件（通道 `04`、`05`、`06`、`07`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgaEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 此事件所针对的 BGA 图层。
    pub layer: BgaLayer,
    /// 指向 `#BMP` 表的引用。
    pub bmp_id: BmpIndex,
}

// 小节长度

/// 一个小节长度变更（通道 `02`）。
///
/// 该值是相对于标准 4/4 小节的比值：
/// - `1.0` = 4/4（标准）
/// - `0.75` = 3/4
/// - `2.0` = 8/4
/// - `0.015625` = 1/64
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasureLength {
    /// 小节编号。
    pub measure: u16,
    /// 长度比值（1.0 = 标准 4/4 小节）。
    pub length_ratio: f64,
}

// 基于头部的定位停止

/// 一个基于位置的停止事件（`#STP` 头部命令）。
///
/// 与引用 `#STOPxx` 定义的通道 `09` 停止不同，`#STP` 内联指定精确的
/// 位置与时长。位置使用 1000 作为分母（小节的 1/1000）。
#[derive(Debug, Clone, PartialEq)]
pub struct StpEvent {
    /// 小节内的位置。
    pub position: Position,
    /// 停止时长（毫秒）。
    pub duration_ms: f64,
}

/// 非事件通道的合并数据（仅用于 BMS 引擎特定事件的延迟解析）。
///
/// 由最终化阶段在匹配到非事件通道时填充，处理器（`bms-processor`）
/// 读取此数据转换为 [`EventKind::Custom`](bmsrs_chart::EventKind::Custom)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEventData {
    /// 小节号。
    pub measure: u16,
    /// 通道类型。
    pub channel: BmsChannel,
    /// 合并后的通道值字符串。
    pub data: String,
}

// 消息容器

/// 原始与已解析通道消息数据的容器。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Messages {
    /// 按（小节 → 通道）存储的原始值字符串，保留各行独立。
    ///
    /// 同一 `(measure, channel)` 的多行作为 `Vec` 中的独立条目按文件
    /// 顺序存储。在 [`finalize`](Self::finalize) 期间，非 BGM 通道会被
    /// **按位置合并**（后续行覆盖先前的非 `00` 位置），而 BGM 行则独立
    /// 处理以支持多声部。
    pub raw: BTreeMap<u16, BTreeMap<BmsChannel, Vec<String>>>,

    /// 从通道 `01` 解析出的 BGM 事件。
    pub bgm_events: Vec<BgmEvent>,
    /// 从通道 `11`–`19`、`21`–`29`、`31`–`39`、`41`–`49` 解析出的可玩
    /// 音符事件。
    pub note_events: Vec<NoteEvent>,
    /// 从通道 `51`–`59`、`61`–`69` 解析出的长音事件。
    pub long_note_events: Vec<LongNoteEvent>,
    /// 从通道 `D1`–`D9`、`E1`–`E9` 解析出的地雷事件。
    pub mine_events: Vec<MineEvent>,
    /// 从通道 `03`、`08` 解析出的 BPM 变更事件。
    pub bpm_changes: Vec<BpmChange>,
    /// 从通道 `09` 解析出的停止事件。
    pub stop_events: Vec<StopEvent>,
    /// 从通道 `SC` 解析出的滚动速度事件。
    pub scroll_events: Vec<ScrollEvent>,
    /// 从通道 `SP` 解析出的视觉音符间距关键帧事件。
    pub speed_events: Vec<SpeedEvent>,
    /// 从通道 `04`–`07` 解析出的 BGA 显示事件。
    pub bga_events: Vec<BgaEvent>,
    /// 从通道 `02` 解析出的小节长度变更。
    pub measure_lengths: Vec<MeasureLength>,
    /// 基于头部命令的位置停止（`#STP`）。
    pub stp_events: Vec<StpEvent>,

    /// 非事件通道的合并数据（仅用于 BMS 引擎特定事件的延迟解析）。
    ///
    /// 每个条目包含小节号、通道类型与合并后的通道值字符串，由
    /// `finalize_merged` 在匹配到非事件通道时填充。处理器（`bms-processor`）
    /// 读取此数据转换为 [`EventKind::Custom`](bmsrs_chart::EventKind::Custom)。
    pub non_event_data: Vec<NonEventData>,
}

/// 将枚举索引 `i` 转换为 [`Position`]，集中处理 `usize → u32` 截断期望。
///
/// BMS 每通道每小节的内容数量（对象行数 × 每行值数）在实际谱面中
/// 远小于 `u32::MAX`，截断是安全的。
#[expect(
    clippy::cast_possible_truncation,
    reason = "BMS per-channel per-measure value count fits in u32"
)]
const fn event_pos(i: usize, measure: u16, denom: u32) -> Position {
    Position::new(measure, i as u32, denom)
}

impl Messages {
    /// 将一条消息的值追加到 `raw` 存储。
    ///
    /// 同一 `(measure, channel)` 的每一行作为内部 `Vec` 的独立条目存储，
    /// 保留文件顺序。在接收全部消息后调用
    /// [`finalize`](Self::finalize)；它将按位置合并非 BGM 通道，然后
    /// 解析事件。
    pub fn concat_raw<C: AsRef<str>>(&mut self, msg: &bms_tokenizer::BmsMessage<C>) {
        self.raw
            .entry(msg.track())
            .or_default()
            .entry(msg.channel())
            .or_default()
            .push(msg.body.as_ref().to_owned());
    }

    /// 从原始多行存储中完成事件解析的最终化。
    ///
    /// 必须在通过 [`concat_raw`](Self::concat_raw) 添加**全部**通道消息
    /// 之后调用。对于每个 `(measure, channel)`：
    ///
    /// - **BGM**（`ch01`）：每行独立处理，产生带各行分母的事件（支持
    ///   多声部）。
    /// - **`ch02`**（小节长度）：仅使用**最后一**行（类标量通道最后胜出）。
    /// - **`ch03`**（通过十六进制变更 BPM）：与其他可玩通道一样进行
    ///   按位置合并。`"00"` 条目被过滤掉（休止 = 无 BPM 变更）。
    /// - **所有其他通道**（含 `chA6` 选项）：所有行在解析前通过
    ///   `merge_channel` 进行**按位置合并**，遵循后续行覆盖非 `"00"`
    ///   位置而 `"00"` 保留的规格规则。
    ///
    /// 先清除已有解析结果再重新解析，因此可安全重复调用。
    ///
    /// `base` 控制索引归一化：在标准 Base36 模式下索引被转为大写以进行
    /// 不区分大小写的查找；在 Base62 模式下保留原始大小写。
    pub fn finalize(&mut self, base: BmsBase) {
        // 清除已有的解析结果，使 finalize 可安全重复调用。
        self.bgm_events.clear();
        self.note_events.clear();
        self.long_note_events.clear();
        self.mine_events.clear();
        self.bpm_changes.clear();
        self.stop_events.clear();
        self.scroll_events.clear();
        self.speed_events.clear();
        self.bga_events.clear();
        self.measure_lengths.clear();
        self.non_event_data.clear();

        let raw = std::mem::take(&mut self.raw);

        for (&measure, channels) in &raw {
            for (&channel, lines) in channels {
                self.finalize_channel(channel, lines, measure, base);
            }
        }

        self.raw = raw;
    }

    /// 将单个通道的多行数据分派到对应的最终化方法。
    fn finalize_channel(
        &mut self,
        channel: BmsChannel,
        lines: &[String],
        measure: u16,
        base: BmsBase,
    ) {
        match channel {
            // BGM：每行独立（多声部）。
            BmsChannel::Bgm => self.finalize_bgm_lines(lines, measure, base),
            // 小节长度 / 选项：最后一行胜出。
            BmsChannel::MeasureLength => self.finalize_measure_length_line(lines, measure),
            // 所有其他通道：按位置合并后解析事件。
            _ => self.finalize_merged(channel, lines, measure, base),
        }
    }

    /// 处理 BGM 通道的最终化：每行独立处理（多声部）。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "BMS measure value count fits in u32"
    )]
    fn finalize_bgm_lines(&mut self, lines: &[String], measure: u16, base: BmsBase) {
        for line in lines {
            let objects = split_2char_values_lenient(line);
            let total_objects = objects.len() as u32;
            self.push_bgm_full(line, measure, total_objects, base);
        }
    }

    /// 处理小节长度通道的最终化：仅使用最后一行。
    fn finalize_measure_length_line(&mut self, lines: &[String], measure: u16) {
        if let Some(last) = lines.last() {
            self.push_measure_length(last, measure);
        }
    }

    /// 处理其他通道的最终化：按位置合并后按事件类型分发。
    #[expect(
        clippy::cast_possible_truncation,
        reason = "BMS measure value count fits in u32"
    )]
    fn finalize_merged(
        &mut self,
        channel: BmsChannel,
        lines: &[String],
        measure: u16,
        base: BmsBase,
    ) {
        let merged = if lines.len() <= 1 {
            lines.first().cloned().unwrap_or_default()
        } else {
            merge_channel(lines)
        };
        let objects = split_2char_values_lenient(&merged);
        let total_objects = objects.len() as u32;

        match channel {
            BmsChannel::BpmChange => self.push_bpm_absolute_full(&merged, measure, total_objects),
            BmsChannel::ExtendedBpm => {
                self.push_bpm_reference_full(&merged, measure, total_objects, base);
            }
            BmsChannel::BgaBase => {
                self.push_bga_full(&merged, measure, BgaLayer::Base, total_objects, base);
            }
            BmsChannel::BgaPoor => {
                self.push_bga_full(&merged, measure, BgaLayer::Poor, total_objects, base);
            }
            BmsChannel::BgaLayer => {
                self.push_bga_full(&merged, measure, BgaLayer::Layer, total_objects, base);
            }
            BmsChannel::BgaLayer2 => {
                self.push_bga_full(&merged, measure, BgaLayer::Layer2, total_objects, base);
            }
            BmsChannel::Stop => self.push_stop_full(&merged, measure, total_objects, base),
            BmsChannel::Scroll => self.push_scroll_full(&merged, measure, total_objects, base),
            BmsChannel::Speed => self.push_speed_full(&merged, measure, total_objects, base),
            BmsChannel::Note(note_ch) => {
                if let Some(ch) = note_ch.as_u8_hex() {
                    self.dispatch_note_channel(&merged, measure, ch, total_objects, base);
                }
            }
            // 非事件通道 —— 保留合并数据供处理器转换为 BmsCustomEvent。
            BmsChannel::BgaBaseOpacity
            | BmsChannel::BgaLayerOpacity
            | BmsChannel::BgaLayer2Opacity
            | BmsChannel::BgaPoorOpacity
            | BmsChannel::BgmVolume
            | BmsChannel::KeyVolume
            | BmsChannel::Text
            | BmsChannel::Judge
            | BmsChannel::BgaArgbBase
            | BmsChannel::BgaArgbLayer
            | BmsChannel::BgaArgbLayer2
            | BmsChannel::BgaArgbPoor
            | BmsChannel::BgaKeyBound
            | BmsChannel::Seek
            | BmsChannel::Option
            | BmsChannel::Unknown(_) => {
                // 保留合并后的字符串以供处理器延迟解析。
                self.non_event_data.push(NonEventData {
                    measure,
                    channel,
                    data: merged,
                });
            }
            // BGM 与 MeasureLength 在此不可达（已在 finalize_channel
            // 的早期分支中处理），保留分支以满足 exhaustiveness。
            BmsChannel::Bgm | BmsChannel::MeasureLength => {}
        }
    }
}

// 通道合并（按位置，遵循规格）

/// 使用按位置合并语义合并同一 `(measure, channel)` 的多条 BMS 消息行：
///
/// - 行按**文件顺序**处理（后续行 = 更高优先级）。
/// - 某位置上的非 `"00"` 值会**覆盖**原有值。
/// - `"00"` 值**保留**现有值（无操作）。
///
/// 最终分辨率（对象总数）为所有行计数的**最大值**。每行的值按比例映射
/// 到此网格上：`dest_position = src_position × max_count / line_count`。
///
/// # Panics
///
/// 当 `lines` 为空时触发 panic（调用方必须自行保护）。
#[expect(
    clippy::indexing_slicing,
    reason = "loop guard ensures access is within bounds"
)]
pub fn merge_channel(lines: &[String]) -> String {
    debug_assert!(!lines.is_empty(), "merge_channel called with empty lines");

    // 将每行解析为 2 字符值块。
    let parsed: Vec<Vec<String>> = lines
        .iter()
        .map(|line| {
            split_2char_values_lenient(line)
                .into_iter()
                .map(String::from)
                .collect()
        })
        .collect();

    // 查找最大计数（最终分辨率）。
    let max_count = parsed.iter().map(Vec::len).max().unwrap_or(1);
    if max_count == 0 {
        return String::new();
    }

    // 以最大分辨率初始化结果缓冲区，全部为 "00"。
    let mut result: Vec<&str> = vec!["00"; max_count];

    // 按文件顺序处理各行。后续行优先级更高，
    // 但 "00" 为无操作（保留现有值）。
    for line_vals in &parsed {
        let line_count = line_vals.len();
        if line_count == 0 {
            continue;
        }
        // 将此行网格上的每个位置映射到最大网格上。
        for (j, val) in line_vals.iter().enumerate() {
            let dest = j * max_count / line_count;
            if val != "00" {
                // dest < max_count 由构造保证
                result[dest] = val;
            }
            // "00" → 跳过（保留现有值）
        }
    }

    result.concat()
}

// 内部解析辅助函数

/// 将 BMS 消息值字符串按宽松解析拆分为 2 字符块：无效字符被静默跳过，
/// 每两个连续的有效 Base62 字符组成一个块。末尾单个有效字符被丢弃。
///
/// 此逻辑镜像了分词器此前的 `parse_body_objects` 逻辑，因此无论在哪个
/// 阶段执行拆分，事件解析的一致性都能得到保证。
///
/// # Panics
///
/// 从逻辑上不会 panic（Base62 字符集是 ASCII 子集，`from_utf8`  不会失败）。
/// 使用 `expect` 作为安全断言。
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "while-loop guard ensures i < len and i+1 < len before indexing"
)]
#[expect(
    clippy::expect_used,
    reason = "two base62 chars are valid ASCII by the is_base62 guard"
)]
pub fn split_2char_values_lenient(values: &str) -> Vec<&str> {
    let bytes = values.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    let len = bytes.len();
    while i < len {
        if BmsBase::Base62.is_valid_char(bytes[i])
            && i + 1 < len
            && BmsBase::Base62.is_valid_char(bytes[i + 1])
        {
            let chunk =
                std::str::from_utf8(&bytes[i..i + 2]).expect("two base62 chars are valid ASCII");
            result.push(chunk);
            i += 2;
        } else {
            i += 1;
        }
    }
    result
}

/// 将 BMS 双字符地雷索引解码为伤害值。
///
/// 该索引被解释为 Base36 值（除非进制覆盖）：
/// - `"00"` → [`None`]（该位置无地雷）
/// - `"01"` → `Some(0.5)`
/// - `"ZZ"` → `Some(f64::INFINITY)`（BMS 规格定义的即死）
/// - 所有其他值 → `Some(base36_value / 2.0)`
fn decode_mine_damage(val: &str, base: BmsBase) -> Option<f64> {
    if val == "00" {
        return None;
    }
    let parsed = if base == BmsBase::Base62 {
        val.parse::<u16>()
            .ok()
            .or_else(|| BmsBase::Base36.decode(val))
    } else {
        let upper = val.to_ascii_uppercase();
        BmsBase::Base36.decode(&upper)
    };
    match parsed {
        Some(1295) => Some(f64::INFINITY),
        Some(n) => Some(f64::from(n) / 2.0),
        None => Some(1.0),
    }
}

impl Messages {
    /// 从完整拼接的值中解析 BGM 事件（ch 01）。
    fn push_bgm_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.bgm_events.push(BgmEvent {
                position: event_pos(i, measure, total_objects),
                wav_id: WavIndex::from(wav_id.normalize(base)),
            });
        }
    }

    /// 从原始值解析小节长度变更（ch 02）。
    ///
    /// 该值是相对于 4/4 的比值：`1.0` = 4/4，`0.75` = 3/4，
    /// `2.0` = 8/4。非数值或零值被静默忽略（小节默认为 4/4）。
    fn push_measure_length(&mut self, values: &str, measure: u16) {
        if let Ok(ratio) = values.trim().parse::<f64>()
            && ratio > 0.0
            && ratio.is_finite()
        {
            self.measure_lengths.push(MeasureLength {
                measure,
                length_ratio: ratio,
            });
        }
    }

    /// 从十六进制值解析绝对 BPM 变更（ch 03）。
    ///
    /// `"00"` 值表示休止（无 BPM 变更），按 BMS 规格被**跳过** —— 将
    /// `BpmValue::Absolute(0.0)` 传递到下游会导致计时轨除以零。
    fn push_bpm_absolute_full(&mut self, values: &str, measure: u16, total_objects: u32) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // "00" = 休止 / 无 BPM 变更 —— 完全跳过。
            if val == "00" {
                continue;
            }
            // 通道 03 的值为十六进制整数（01-FF）
            let Ok(bpm_val) = u8::from_str_radix(val, 16) else {
                continue;
            };
            self.bpm_changes.push(BpmChange {
                position: event_pos(i, measure, total_objects),
                value: BpmValue::Absolute(f64::from(bpm_val)),
            });
        }
    }

    /// 从完整拼接的值中解析 BPM 引用变更（ch 08）。
    fn push_bpm_reference_full(
        &mut self,
        values: &str,
        measure: u16,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // "00" = 休止 / 无 BPM 变更，与通道 03 的 absolute BPM 行为一致。
            if val == "00" {
                continue;
            }
            let Ok(bpm_id) = val.parse::<BpmIndex>() else {
                continue;
            };
            self.bpm_changes.push(BpmChange {
                position: event_pos(i, measure, total_objects),
                value: BpmValue::Reference(BpmIndex::from(bpm_id.normalize(base))),
            });
        }
    }

    /// 从完整拼接的值中解析停止事件（ch 09）。
    fn push_stop_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            if val == "00" {
                continue;
            }
            let Ok(stop_id) = val.parse::<StopIndex>() else {
                continue;
            };
            self.stop_events.push(StopEvent {
                position: event_pos(i, measure, total_objects),
                stop_id: StopIndex::from(stop_id.normalize(base)),
            });
        }
    }

    /// 从完整拼接的值中解析滚动事件（ch SC）。
    fn push_scroll_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            if val == "00" {
                continue;
            }
            let Ok(scroll_id) = val.parse::<ScrollIndex>() else {
                continue;
            };
            self.scroll_events.push(ScrollEvent {
                position: event_pos(i, measure, total_objects),
                scroll_id: ScrollIndex::from(scroll_id.normalize(base)),
            });
        }
    }

    /// 从完整拼接的值中解析速度关键帧事件（ch SP）。
    fn push_speed_full(&mut self, values: &str, measure: u16, total_objects: u32, base: BmsBase) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            if val == "00" {
                continue;
            }
            let Ok(speed_id) = val.parse::<SpeedIndex>() else {
                continue;
            };
            self.speed_events.push(SpeedEvent {
                position: event_pos(i, measure, total_objects),
                speed_id: SpeedIndex::from(speed_id.normalize(base)),
            });
        }
    }

    /// 从完整拼接的值中解析 BGA 显示事件（ch 04–07）。
    fn push_bga_full(
        &mut self,
        values: &str,
        measure: u16,
        layer: BgaLayer,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            let Ok(bmp_id) = val.parse::<BmpIndex>() else {
                continue;
            };
            self.bga_events.push(BgaEvent {
                position: event_pos(i, measure, total_objects),
                layer,
                bmp_id: BmpIndex::from(bmp_id.normalize(base)),
            });
        }
    }

    /// 从完整拼接的值中解析可玩音符事件（ch 11–49）。
    ///
    /// WAV 索引为 `"00"` 的条目会被过滤掉 —— 它们表示"无音符"
    ///（静音步）位置，不应产生事件。
    #[expect(
        clippy::too_many_arguments,
        reason = "BMS event parsing requires channel context: measure, player, lane, type, plus base for normalization"
    )]
    fn push_playable_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        key_type: KeyType,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // "00" = 无音符 —— 完全跳过。这必须在解析之前发生，
            // 因为 "00" 是一个有效索引，但在所有 BMS 通道中其语义
            // 都是"该位置无对象"。
            if val == "00" {
                continue;
            }
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.note_events.push(NoteEvent {
                position: event_pos(i, measure, total_objects),
                player,
                lane,
                key_type,
                wav_id: WavIndex::from(wav_id.normalize(base)),
            });
        }
    }

    /// 从完整拼接的值中解析长音事件（ch 51–69）。
    fn push_long_note_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            if val == "00" {
                continue;
            }
            let Ok(wav_id) = val.parse::<WavIndex>() else {
                continue;
            };
            self.long_note_events.push(LongNoteEvent {
                position: event_pos(i, measure, total_objects),
                player,
                lane,
                wav_id: WavIndex::from(wav_id.normalize(base)),
            });
        }
    }

    /// 从完整拼接的值中解析地雷事件（ch D1–E9）。
    ///
    /// 双字符索引将伤害编码为 Base36 值：
    /// `damage = value / 2.0`，其中 `ZZ`（1295）= 即死。
    /// `"00"` 条目被过滤掉（该位置无地雷）。
    fn push_mine_full(
        &mut self,
        values: &str,
        measure: u16,
        player: u8,
        lane: u8,
        total_objects: u32,
        base: BmsBase,
    ) {
        for (i, val) in split_2char_values_lenient(values).into_iter().enumerate() {
            // "00" = 无地雷 —— 完全跳过（也作为 decode_mine_damage 的快速路径）。
            if val == "00" {
                continue;
            }
            let Some(damage) = decode_mine_damage(val, base) else {
                continue;
            };
            self.mine_events.push(MineEvent {
                position: event_pos(i, measure, total_objects),
                player,
                lane,
                damage,
            });
        }
    }

    /// 将音符通道的原始十六进制值分发到对应的处理器。
    ///
    /// 当 [`BmsChannel::Note`] 变体具有可解码的十六进制通道值时，从
    /// [`finalize`](Self::finalize) 调用。
    fn dispatch_note_channel(
        &mut self,
        values: &str,
        measure: u16,
        ch: u8,
        total_objects: u32,
        base: BmsBase,
    ) {
        match ch {
            0x11..=0x19 => {
                self.push_playable_full(
                    values,
                    measure,
                    1,
                    ch - 0x10,
                    KeyType::Visible,
                    total_objects,
                    base,
                );
            }
            0x21..=0x29 => {
                self.push_playable_full(
                    values,
                    measure,
                    2,
                    ch - 0x20,
                    KeyType::Visible,
                    total_objects,
                    base,
                );
            }
            0x31..=0x39 => {
                self.push_playable_full(
                    values,
                    measure,
                    1,
                    ch - 0x30,
                    KeyType::Invisible,
                    total_objects,
                    base,
                );
            }
            0x41..=0x49 => {
                self.push_playable_full(
                    values,
                    measure,
                    2,
                    ch - 0x40,
                    KeyType::Invisible,
                    total_objects,
                    base,
                );
            }
            0x51..=0x59 => {
                self.push_long_note_full(values, measure, 1, ch - 0x50, total_objects, base);
            }
            0x61..=0x69 => {
                self.push_long_note_full(values, measure, 2, ch - 0x60, total_objects, base);
            }
            0xD1..=0xD9 => {
                self.push_mine_full(values, measure, 1, ch - 0xD0, total_objects, base);
            }
            0xE1..=0xE9 => {
                self.push_mine_full(values, measure, 2, ch - 0xE0, total_objects, base);
            }
            _ => { /* Note 变体中的非音符十六进制 —— 仅保留在 raw 中 */ }
        }
    }
}
