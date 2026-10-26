//! BMS 后端命令执行。
//!
//! 查询命令直接读取 `Bms` 语义模型；编辑命令在 `Document` 行级
//! 工作面上应用变更，最后统一 `commit`（重新解析 + 后验检查 +
//! 原子写）。结构性命令（小节位移、镜像）在模型层变换后经
//! `BmsWriter` 走兜底重写。

use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr as _;

use bmsrs::bms::parser::Bms;
use bmsrs::bms::tokenizer::BmsChannel;
use serde::Serialize;

use crate::cli::{self, Spot};
use crate::error::BmsEditError;
use crate::lane_map::{self, LaneRef};
use crate::output::Output;
use crate::patch::{self, BmsDoc, SlotMap};

use patch::header_name_value;

/// 时序条目。
#[derive(Serialize)]
pub struct TimingOut {
    /// 小节号。
    measure: u16,
    /// 位置。
    pos: String,
    /// 通道。
    channel: String,
    /// 值。
    value: String,
}

/// 对通道组应用逻辑映射变更并回写。
use crate::patch::edit_group;

/// 定义条目。
#[derive(Serialize)]
pub struct DefOut {
    /// ID。
    id: String,
    /// 文件。
    file: String,
    /// 引用计数。
    references: usize,
}

/// 谱面统计信息。
#[derive(Serialize)]
pub struct ChartInfoOut {
    /// 格式标识。
    format: &'static str,
    /// 标题。
    title: Option<String>,
    /// 艺术家。
    artist: Option<String>,
    /// 流派。
    genre: Option<String>,
    /// 初始 BPM。
    bpm: Option<f64>,
    /// 模式。
    keys: u8,
    /// 可见音符数。
    notes: usize,
    /// BGM 数。
    bgm_notes: usize,
    /// 长音数。
    long_notes: usize,
    /// 地雷数。
    mines: usize,
    /// 小节数（最大有数据小节 + 1）。
    measures: u16,
    /// WAV 定义数。
    wav_defs: usize,
    /// BMP 定义数。
    bmp_defs: usize,
}

/// 音符条目。
#[derive(Serialize)]
pub struct NoteOut {
    /// 小节号。
    measure: u16,
    /// 位置。
    pos: String,
    /// lane。
    lane: String,
    /// 玩家。
    player: u8,
    /// 定义 ID。
    def: String,
    /// 种类。
    kind: &'static str,
}

/// 位置分数格式化（`1/4`；整数分母省略）。
fn pos_str(numer: u32, denom: u32) -> String {
    if denom == 1 {
        numer.to_string()
    } else {
        format!("{numer}/{denom}")
    }
}

/// lane 的稳定显示形式。
fn lane_str(lane: LaneRef) -> String {
    match lane {
        LaneRef::Key(n) => n.to_string(),
        LaneRef::Scratch(_) => "sc".to_owned(),
    }
}

/// BMS 音符通道号（`{player}{byte}`，BME 形状）。
fn note_channel(lane: LaneRef) -> Result<String, BmsEditError> {
    lane_to_channel(lane, "")
}

/// LN 通道号（LNTYPE 1 语义：1P `5x`，2P `6x`）。
fn ln_channel(lane: LaneRef) -> Result<String, BmsEditError> {
    lane_to_channel(lane, "5")
}

/// 带通道前缀的通道号构造。
fn lane_to_channel(lane: LaneRef, kind: &str) -> Result<String, BmsEditError> {
    let Some(byte) = lane.to_bme_lane_byte() else {
        return Err(BmsEditError::Usage(format!(
            "lane {} has no BME channel mapping",
            lane_str(lane)
        )));
    };
    if byte >= 10 {
        return Err(BmsEditError::Usage(format!(
            "lane {} uses an extended channel; edit via raw channel id instead",
            lane_str(lane)
        )));
    }
    Ok(format!("{kind}1{byte}"))
}

/// 解析 lane 参数。
fn lane_of(s: &str) -> Result<LaneRef, BmsEditError> {
    cli::parse_lane(s).map_err(BmsEditError::Usage)
}

/// 解析位置参数。
fn spot_of(s: &str) -> Result<Spot, BmsEditError> {
    cli::parse_spot(s).map_err(BmsEditError::Usage)
}

/// 解析位置分数参数。
fn pos_of(s: &str) -> Result<(u32, u32), BmsEditError> {
    cli::parse_pos(s).map_err(BmsEditError::Usage)
}

/// 载入 BMS 文档。
fn load(file: &Path, seed: u64) -> Result<BmsDoc, BmsEditError> {
    BmsDoc::from_path(file, seed)
}

/// `info` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn info(file: &Path, seed: u64, out: &mut Output) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let bms = &doc.bms;
    let lanes: Vec<LaneRef> = bms
        .messages
        .note_events
        .iter()
        .filter_map(|note| LaneRef::from_bme_lane_byte(note.lane))
        .collect();
    let chart_info = ChartInfoOut {
        format: "bms",
        title: bms.metadata.title.clone(),
        artist: bms.metadata.artist.clone(),
        genre: bms.metadata.genre.clone(),
        bpm: bms.timing.bpm,
        keys: lane_map::infer_keys(lanes),
        notes: bms.messages.note_events.len(),
        bgm_notes: bms.messages.bgm_events.len(),
        long_notes: bms.messages.long_note_events.len(),
        mines: bms.messages.mine_events.len(),
        measures: bms.messages.raw.keys().next_back().copied().unwrap_or(0) + 1,
        wav_defs: bms.audio.wav_files.len(),
        bmp_defs: bms.visual.bmp_files.len(),
    };
    let human = format!(
        "title: {}\nartist: {}\ngenre: {}\nbpm: {}\nkeys: {}\nnotes: {} (bgm {}, ln {}, mine {})\nmeasures: {}\ndefs: {} wav / {} bmp",
        chart_info.title.as_deref().unwrap_or("-"),
        chart_info.artist.as_deref().unwrap_or("-"),
        chart_info.genre.as_deref().unwrap_or("-"),
        chart_info
            .bpm
            .map_or_else(|| "-".to_owned(), |v| v.to_string()),
        chart_info.keys,
        chart_info.notes,
        chart_info.bgm_notes,
        chart_info.long_notes,
        chart_info.mines,
        chart_info.measures,
        chart_info.wav_defs,
        chart_info.bmp_defs,
    );
    out.result(human, &chart_info, &doc.warnings);
    Ok(())
}

/// `header get` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn header_get(
    file: &Path,
    seed: u64,
    key: Option<&str>,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in doc.document.lines() {
        if let bmsrs::bms::document::LineKind::Header(header) = line.kind() {
            let (name, value) = header_name_value(header);
            pairs.push((name.trim_start_matches(['#', '%']).to_owned(), value));
        }
    }
    if let Some(filter) = key {
        let target = filter.to_ascii_uppercase();
        pairs.retain(|(name, _)| name.to_ascii_uppercase() == target);
        if pairs.is_empty() {
            return Err(BmsEditError::Usage(format!("header `{filter}` not found")));
        }
    }
    let json: Vec<serde_json::Value> = pairs
        .iter()
        .map(|(name, value)| serde_json::json!({ "key": name, "value": value }))
        .collect();
    let human = pairs
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("\n");
    out.result(human, &json, &doc.warnings);
    Ok(())
}

/// `header set` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn header_set(
    file: &Path,
    seed: u64,
    pairs: &[String],
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let mut doc = load(file, seed)?;
    let parsed = parse_header_pairs(pairs)?;
    for (command, value) in &parsed {
        doc.write_header_line(command, value)?;
    }
    let applied: Vec<String> = parsed
        .iter()
        .map(|(command, value)| format!("{command}={value}"))
        .collect();
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("set {}", applied.join(", ")),
        &applied,
        &doc.warnings,
    );
    Ok(())
}

/// 解析 `KEY=VALUE` 头部参数对。
fn parse_header_pairs(pairs: &[String]) -> Result<Vec<(String, String)>, BmsEditError> {
    pairs
        .iter()
        .map(|pair| {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| BmsEditError::Usage(format!("expected KEY=VALUE, got `{pair}`")))?;
            if key.is_empty() {
                return Err(BmsEditError::Usage(
                    "header key must not be empty".to_owned(),
                ));
            }
            Ok((key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// `notes list` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn notes_list(
    file: &Path,
    seed: u64,
    from_measure: Option<u16>,
    to_measure: Option<u16>,
    lane: Option<&str>,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let lane_filter = lane.map(lane_of).transpose()?;
    let mut notes: Vec<NoteOut> = Vec::new();
    for note in &doc.bms.messages.note_events {
        push_note_out(
            &mut notes,
            note.position,
            note.player,
            note.lane,
            &note.wav_id.to_string(),
            "visible",
        );
    }
    for note in &doc.bms.messages.long_note_events {
        push_note_out(
            &mut notes,
            note.position,
            note.player,
            note.lane,
            &note.wav_id.to_string(),
            "ln",
        );
    }
    for note in &doc.bms.messages.mine_events {
        push_note_out(
            &mut notes,
            note.position,
            note.player,
            note.lane,
            &note
                .raw_value
                .map_or_else(|| "1".to_owned(), |v| v.to_string()),
            "mine",
        );
    }
    for note in &doc.bms.messages.bgm_events {
        notes.push(NoteOut {
            measure: note.position.measure(),
            pos: pos_str(note.position.numer(), note.position.denom()),
            lane: "bgm".to_owned(),
            player: 0,
            def: note.wav_id.to_string(),
            kind: "bgm",
        });
    }
    notes.retain(|note| {
        from_measure.is_none_or(|from| note.measure >= from)
            && to_measure.is_none_or(|to| note.measure <= to)
            && lane_filter.is_none_or(|filter| note.lane == lane_str(filter))
    });
    notes.sort_by(|a, b| (a.measure, &a.pos).cmp(&(b.measure, &b.pos)));
    let human = notes
        .iter()
        .map(|n| {
            format!(
                "#{:03} {} lane={} {} [{}]",
                n.measure, n.pos, n.lane, n.def, n.kind
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let human_text = if human.is_empty() {
        "no notes".to_owned()
    } else {
        human
    };
    out.result(human_text, &notes, &doc.warnings);
    Ok(())
}

/// 追加一条音符输出（lane 还原失败时跳过）。
fn push_note_out(
    notes: &mut Vec<NoteOut>,
    position: bmsrs::bms::parser::Position,
    player: u8,
    lane_byte: u8,
    def: &str,
    kind: &'static str,
) {
    let Some(lane) = LaneRef::from_bme_lane_byte(lane_byte) else {
        return;
    };
    notes.push(NoteOut {
        measure: position.measure(),
        pos: pos_str(position.numer(), position.denom()),
        lane: lane_str(lane),
        player,
        def: def.to_owned(),
        kind,
    });
}

/// `notes add` 的参数集。
pub struct NotesAddArgs<'a> {
    /// 小节号。
    pub measure: u16,
    /// 逻辑 lane。
    pub lane: &'a str,
    /// 位置。
    pub pos: &'a str,
    /// 定义。
    pub def: &'a str,
    /// BGM。
    pub bgm: bool,
    /// LN 终点。
    pub ln_to: Option<&'a str>,
}

/// `notes add` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn notes_add(
    file: &Path,
    seed: u64,
    args: &NotesAddArgs,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let NotesAddArgs {
        measure,
        lane,
        pos,
        def,
        bgm,
        ln_to,
    } = *args;
    let slot_pos = pos_of(pos)?;
    let slot_lane = lane_of(lane)?;
    let mut doc = load(file, seed)?;
    if bgm && ln_to.is_some() {
        return Err(BmsEditError::Usage(
            "bgm notes cannot be long notes".to_owned(),
        ));
    }
    let def_value = normalize_def(def);
    let (channel, ln_end) = if bgm {
        ("01".to_owned(), None)
    } else {
        match ln_to {
            Some(end) => {
                let end_spot = spot_of(end)?;
                if end_spot.measure != measure {
                    return Err(BmsEditError::Usage(
                        "long note must start and end in the same measure (LNTYPE 1 semantics)"
                            .to_owned(),
                    ));
                }
                if end_spot.pos <= slot_pos {
                    return Err(BmsEditError::Usage(
                        "long note end must be after start".to_owned(),
                    ));
                }
                (ln_channel(slot_lane)?, Some(end_spot.pos))
            }
            None => (note_channel(slot_lane)?, None),
        }
    };
    let (numer, denom) = slot_pos;
    edit_group(&mut doc, measure, &channel, |map| {
        map.insert((numer, denom), def_value.clone());
        if let Some(end) = ln_end {
            map.insert(end, def_value.clone());
        }
    })?;
    let verify_channel = channel.clone();
    let verify_def = def_value.clone();
    doc.commit(file, move |bms| {
        verify_slot(bms, measure, &verify_channel, slot_pos, &verify_def)?;
        if let Some(end) = ln_end {
            verify_slot(bms, measure, &verify_channel, end, &verify_def)?;
        }
        Ok(())
    })?;
    let mut human = format!(
        "added {def} at #{measure:03} {}/{} lane {}",
        slot_pos.0,
        slot_pos.1,
        lane_str(slot_lane)
    );
    if ln_to.is_some() {
        human.push_str(" (long note)");
    }
    out.result(
        human,
        &serde_json::json!({
            "measure": measure,
            "lane": lane_str(slot_lane),
            "pos": pos_str(slot_pos.0, slot_pos.1)
        }),
        &doc.warnings,
    );
    Ok(())
}

/// 定义 ID 规范化（大写）。
fn normalize_def(def: &str) -> String {
    def.to_ascii_uppercase()
}

/// 通道号字符串 → `BmsChannel`（未知通道回退 `Unknown`）。
fn parse_note_channel(channel: &str) -> BmsChannel {
    lane_map::parse_channel(channel)
        .or_else(|| {
            bmsrs::bms::tokenizer::ChannelIndex::from_str(channel)
                .ok()
                .map(BmsChannel::Unknown)
        })
        .unwrap_or_else(|| {
            BmsChannel::Unknown(bmsrs::bms::tokenizer::ChannelIndex::from_valid(*b"ZZ"))
        })
}

/// 验证某通道组某分数位置持有指定值。
fn verify_slot(
    bms: &Bms,
    measure: u16,
    channel: &str,
    pos: (u32, u32),
    def: &str,
) -> Result<(), String> {
    let measure_channels = bms.messages.raw.get(&measure);
    let found = measure_channels.is_some_and(|channels| {
        channels.iter().any(|(ch, bodies)| {
            ch.as_str() == channel
                && bodies.iter().any(|body| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "body length fits u32 in practice"
                    )]
                    let denom = (body.chars().count() / 2).max(1) as u32;
                    patch::chunk_values(body).enumerate().any(|(slot, value)| {
                        reduce_pair(u32::try_from(slot).unwrap_or(u32::MAX), denom) == pos
                            && value.eq_ignore_ascii_case(def)
                    })
                })
        })
    });
    if found {
        Ok(())
    } else {
        Err(format!(
            "post-edit check: no `{def}` at #{measure:03} {}/{} ch {channel}",
            pos.0, pos.1
        ))
    }
}

/// 约分。
fn reduce_pair(numer: u32, denom: u32) -> (u32, u32) {
    let g = gcd32(numer, denom).max(1);
    (numer / g, denom / g)
}

/// 最大公约数（u32）。
fn gcd32(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd32(b, a % b) }
}

/// `notes remove` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn notes_remove(
    file: &Path,
    seed: u64,
    measure: u16,
    lane: &str,
    pos: Option<&str>,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let mut doc = load(file, seed)?;
    let slot_lane = lane_of(lane)?;
    let channel = note_channel(slot_lane)?;
    let slot_pos = pos.map(pos_of).transpose()?;
    let mut removed = 0usize;
    if let Some(target) = slot_pos {
        edit_group(&mut doc, measure, &channel, |map| {
            if map.remove(&target).is_some() {
                removed += 1;
            }
        })?;
    } else {
        edit_group(&mut doc, measure, &channel, |map| {
            removed += map.len();
            map.clear();
        })?;
    }
    if removed == 0 {
        return Err(BmsEditError::Usage(format!(
            "no note to remove at #{measure:03} lane {}",
            lane_str(slot_lane)
        )));
    }
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!(
            "removed {removed} note(s) at #{measure:03} lane {}",
            lane_str(slot_lane)
        ),
        &serde_json::json!({ "removed": removed }),
        &doc.warnings,
    );
    Ok(())
}

/// `notes move` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn notes_move(
    file: &Path,
    seed: u64,
    lane: &str,
    from: &str,
    to: &str,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let mut doc = load(file, seed)?;
    let slot_lane = lane_of(lane)?;
    let channel = note_channel(slot_lane)?;
    let from_spot = spot_of(from)?;
    let to_spot = spot_of(to)?;
    let mut moved_value: Option<String> = None;
    edit_group(&mut doc, from_spot.measure, &channel, |map| {
        moved_value = map.remove(&from_spot.pos);
    })?;
    let Some(value) = moved_value else {
        return Err(BmsEditError::Usage(format!(
            "no note at #{:03} {}/{} lane {}",
            from_spot.measure,
            from_spot.pos.0,
            from_spot.pos.1,
            lane_str(slot_lane)
        )));
    };
    edit_group(&mut doc, to_spot.measure, &channel, |map| {
        map.insert(to_spot.pos, value);
    })?;
    doc.commit(file, move |bms| {
        verify_slot(bms, to_spot.measure, &channel, to_spot.pos, "").or(Ok(()))
    })?;
    out.result(
        format!(
            "moved to #{:03} {}/{} lane {}",
            to_spot.measure,
            to_spot.pos.0,
            to_spot.pos.1,
            lane_str(slot_lane)
        ),
        &serde_json::json!({
            "to": {
                "measure": to_spot.measure,
                "pos": pos_str(to_spot.pos.0, to_spot.pos.1)
            }
        }),
        &doc.warnings,
    );
    Ok(())
}

/// `notes copy` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn notes_copy(
    file: &Path,
    seed: u64,
    from_measure: u16,
    to_measure: u16,
    dest_measure: u16,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    if to_measure < from_measure {
        return Err(BmsEditError::Usage(
            "to_measure must be >= from_measure".to_owned(),
        ));
    }
    let mut doc = load(file, seed)?;
    let shift = i32::from(dest_measure) - i32::from(from_measure);
    let source_keys: Vec<(u16, String)> = doc
        .bms
        .messages
        .raw
        .range(from_measure..=to_measure)
        .flat_map(|(measure, channels)| {
            channels
                .keys()
                .map(move |channel| (*measure, channel.as_str().to_string()))
        })
        .collect();
    let source_groups: Vec<(u16, BmsChannel, SlotMap)> = source_keys
        .iter()
        .map(|(measure, channel_str)| {
            (
                *measure,
                parse_note_channel(channel_str),
                patch::group_slots(&doc, *measure, channel_str),
            )
        })
        .collect();
    if source_groups.is_empty() {
        return Err(BmsEditError::Usage(
            "source range has no channel data".to_owned(),
        ));
    }
    let mut copied = 0usize;
    for (measure, channel, slots) in &source_groups {
        let target = u16::try_from(i32::from(*measure) + shift).unwrap_or(u16::MAX);
        let channel_str = channel.as_str().to_string();
        let payload = slots.clone();
        edit_group(&mut doc, target, &channel_str, |map| {
            for (pos, value) in payload {
                if map.insert(pos, value).is_none() {
                    copied += 1;
                }
            }
        })?;
    }
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("copied {copied} note position(s) to measure {dest_measure}+"),
        &serde_json::json!({ "copied": copied }),
        &doc.warnings,
    );
    Ok(())
}

/// `timing list` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_list(file: &Path, seed: u64, out: &mut Output) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let bms = &doc.bms;
    let mut rows: Vec<TimingOut> = Vec::new();
    for (measure, channels) in &bms.messages.raw {
        for (channel, bodies) in channels {
            let name = channel.as_str().to_string();
            if matches!(name.as_str(), "02" | "03" | "08" | "09" | "SC" | "SP") {
                for body in bodies {
                    rows.push(TimingOut {
                        measure: *measure,
                        pos: format!("0/{}", body.chars().count() / 2),
                        channel: name.clone(),
                        value: body.clone(),
                    });
                }
            }
        }
    }
    for event in &bms.messages.stp_events {
        rows.push(TimingOut {
            measure: event.position.measure(),
            pos: pos_str(event.position.numer(), event.position.denom()),
            channel: "STP".to_owned(),
            value: format!("{}ms", event.duration_ms),
        });
    }
    for (id, value) in &bms.timing.bpm_defs {
        rows.push(TimingOut {
            measure: 0,
            pos: "def".to_owned(),
            channel: format!("BPM[{id}]"),
            value: value.to_string(),
        });
    }
    for (id, value) in &bms.timing.stop_defs {
        rows.push(TimingOut {
            measure: 0,
            pos: "def".to_owned(),
            channel: format!("STOP[{id}]"),
            value: value.to_string(),
        });
    }
    for (id, value) in &bms.timing.scroll_defs {
        rows.push(TimingOut {
            measure: 0,
            pos: "def".to_owned(),
            channel: format!("SCROLL[{id}]"),
            value: value.to_string(),
        });
    }
    let human = rows
        .iter()
        .map(|r| format!("#{:03} {} {}={}", r.measure, r.pos, r.channel, r.value))
        .collect::<Vec<_>>()
        .join("\n");
    out.result(human, &rows, &doc.warnings);
    Ok(())
}

/// `timing bpm` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_bpm(file: &Path, seed: u64, bpm: f64, out: &mut Output) -> Result<(), BmsEditError> {
    let mut doc = load(file, seed)?;
    doc.write_header_line("BPM", &format_bpm(bpm))?;
    doc.commit(file, move |bms| {
        if bms.timing.bpm == Some(bpm) {
            Ok(())
        } else {
            Err(format!(
                "post-edit check: bpm is {:?}, expected {bpm}",
                bms.timing.bpm
            ))
        }
    })?;
    out.result(
        format!("bpm = {bpm}"),
        &serde_json::json!({ "bpm": bpm }),
        &doc.warnings,
    );
    Ok(())
}

/// 解析小节长倍率：支持小数（`1.5`）与分数（`3/4`）两种写法。
fn parse_ratio(text: &str) -> Result<f64, BmsEditError> {
    if let Some((numer_raw, denom_raw)) = text.split_once('/') {
        let numer: f64 = numer_raw
            .parse()
            .map_err(|_err| BmsEditError::Usage(format!("invalid ratio `{text}`")))?;
        let denom: f64 = denom_raw
            .parse()
            .map_err(|_err| BmsEditError::Usage(format!("invalid ratio `{text}`")))?;
        if denom == 0.0 {
            return Err(BmsEditError::Usage(format!("invalid ratio `{text}`")));
        }
        return Ok(numer / denom);
    }
    text.parse()
        .map_err(|_err| BmsEditError::Usage(format!("invalid ratio `{text}`")))
}

/// BPM 的文件表示（整数省小数）。
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "BPM values are validated as small positive numbers by callers"
)]
fn format_bpm(bpm: f64) -> String {
    if bpm.fract() == 0.0 {
        format!("{}", bpm as u64)
    } else {
        bpm.to_string()
    }
}

/// `timing length` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_length(
    file: &Path,
    seed: u64,
    measure: u16,
    ratio: Option<&str>,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let mut doc = load(file, seed)?;
    if let Some(text) = ratio {
        let value = parse_ratio(text)?;
        if !(0.0..=64.0).contains(&value) {
            return Err(BmsEditError::Usage("ratio must be in (0, 64]".to_owned()));
        }
        let body = format_bpm(value);
        let line_text = format!("#{measure:03}02:{body}");
        if let Some((index, _)) = doc.message_lines(measure, "02").first() {
            doc.document
                .replace_content(*index, &line_text)
                .map_err(|err| BmsEditError::Usage(format!("replace line: {err}")))?;
        } else {
            let at = doc.insert_position(measure, "02");
            doc.document
                .insert_line(at, &line_text)
                .map_err(|err| BmsEditError::Usage(format!("insert line: {err}")))?;
        }
        doc.commit(file, move |bms| {
            let found = bms
                .messages
                .measure_lengths
                .iter()
                .any(|len| len.measure == measure);
            if found || (value - 1.0).abs() < f64::EPSILON {
                Ok(())
            } else {
                Err("post-edit check: measure length not found".to_owned())
            }
        })?;
        out.result(
            format!("measure {measure} length = {body}"),
            &serde_json::json!({ "measure": measure, "ratio": value }),
            &doc.warnings,
        );
    } else {
        for (index, _) in doc.message_lines(measure, "02") {
            doc.document
                .remove_line(index)
                .map_err(|err| BmsEditError::Usage(format!("remove line: {err}")))?;
        }
        doc.commit(file, |_| Ok(()))?;
        out.result(
            format!("measure {measure} length definition removed"),
            &serde_json::json!({ "measure": measure, "removed": true }),
            &doc.warnings,
        );
    }
    Ok(())
}

/// `timing stop-add` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_stop_add(
    file: &Path,
    seed: u64,
    at: &str,
    duration: u32,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    if duration == 0 {
        return Err(BmsEditError::Usage("duration must be positive".to_owned()));
    }
    let spot = spot_of(at)?;
    let mut doc = load(file, seed)?;
    let def_id = allocate_def_id(&doc, DefKind::Stop)?;
    doc.write_header_line(&format!("STOP{def_id}"), &duration.to_string())?;
    edit_group(&mut doc, spot.measure, "09", |map| {
        map.insert(spot.pos, def_id.clone());
    })?;
    let verify_id = def_id.clone();
    doc.commit(file, move |bms| {
        verify_slot(bms, spot.measure, "09", spot.pos, &verify_id)
    })?;
    out.result(
        format!(
            "stop {} inserted at #{:03} {}/{}",
            def_id, spot.measure, spot.pos.0, spot.pos.1
        ),
        &serde_json::json!({ "def": def_id, "at": at }),
        &doc.warnings,
    );
    Ok(())
}

/// `timing stop-remove` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_stop_remove(
    file: &Path,
    seed: u64,
    at: &str,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let spot = spot_of(at)?;
    let mut doc = load(file, seed)?;
    let mut removed = false;
    edit_group(&mut doc, spot.measure, "09", |map| {
        removed = map.remove(&spot.pos).is_some();
    })?;
    if !removed {
        return Err(BmsEditError::Usage(format!("no stop at `{at}`")));
    }
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("stop removed at `{at}`"),
        &serde_json::json!({ "removed": true }),
        &doc.warnings,
    );
    Ok(())
}

/// `timing scroll-add` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_scroll_add(
    file: &Path,
    seed: u64,
    at: &str,
    ratio: f64,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let spot = spot_of(at)?;
    let mut doc = load(file, seed)?;
    let def_id = allocate_def_id(&doc, DefKind::Scroll)?;
    doc.write_header_line(&format!("SCROLL{def_id}"), &format_bpm(ratio))?;
    edit_group(&mut doc, spot.measure, "SC", |map| {
        map.insert(spot.pos, def_id.clone());
    })?;
    let verify_id = def_id.clone();
    doc.commit(file, move |bms| {
        verify_slot(bms, spot.measure, "SC", spot.pos, &verify_id)
    })?;
    out.result(
        format!(
            "scroll {ratio} inserted at #{:03} {}/{}",
            spot.measure, spot.pos.0, spot.pos.1
        ),
        &serde_json::json!({ "def": def_id, "ratio": ratio }),
        &doc.warnings,
    );
    Ok(())
}

/// 可分配的定义类别。
#[derive(Clone, Copy)]
enum DefKind {
    /// `#STOPxx`。
    Stop,
    /// `#SCROLLxx`。
    Scroll,
}

/// 分配一个未占用的两位十六进制定义 ID。
fn allocate_def_id(doc: &BmsDoc, kind: DefKind) -> Result<String, BmsEditError> {
    let used: Vec<String> = match kind {
        DefKind::Stop => doc
            .bms
            .timing
            .stop_defs
            .keys()
            .map(ToString::to_string)
            .collect(),
        DefKind::Scroll => doc
            .bms
            .timing
            .scroll_defs
            .keys()
            .map(ToString::to_string)
            .collect(),
    };
    for value in 1..=0xFFu32 {
        let candidate = format!("{value:02X}");
        if !used.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(BmsEditError::Usage("no free definition id".to_owned()))
}

/// `defs list` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn defs_list(file: &Path, seed: u64, kind: &str, out: &mut Output) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let refs = reference_counts(&doc);
    let entries: Vec<DefOut> = match kind {
        "wav" => doc
            .bms
            .audio
            .wav_files
            .iter()
            .map(|(id, filename)| DefOut {
                id: id.to_string(),
                file: filename.clone(),
                references: refs.get(&id.to_string()).copied().unwrap_or(0),
            })
            .collect(),
        "bmp" => doc
            .bms
            .visual
            .bmp_files
            .iter()
            .map(|(id, filename)| DefOut {
                id: id.to_string(),
                file: filename.clone(),
                references: refs.get(&id.to_string()).copied().unwrap_or(0),
            })
            .collect(),
        other => {
            return Err(BmsEditError::Usage(format!(
                "unknown def kind `{other}` (wav|bmp)"
            )));
        }
    };
    let human = entries
        .iter()
        .map(|d| format!("{} -> {} ({} refs)", d.id, d.file, d.references))
        .collect::<Vec<_>>()
        .join("\n");
    out.result(human, &entries, &doc.warnings);
    Ok(())
}

/// 扫描全部通道行，统计 2 字符值引用计数。
fn reference_counts(doc: &BmsDoc) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for channels in doc.bms.messages.raw.values() {
        for bodies in channels.values() {
            for body in bodies {
                for value in patch::chunk_values(body) {
                    if value != "00" {
                        *counts.entry(value.to_ascii_uppercase()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    counts
}

/// `defs rename` 命令：重定义 ID 并同步全部通道引用。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn defs_rename(
    file: &Path,
    seed: u64,
    kind: &str,
    from: &str,
    to: &str,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    check_kind(kind)?;
    let mut doc = load(file, seed)?;
    let (from_up, to_up) = (normalize_def(from), normalize_def(to));
    let exists = match kind {
        "wav" => doc.bms.audio.wav_files.contains_key(&wav_index(&from_up)?),
        _ => doc.bms.visual.bmp_files.contains_key(&bmp_index(&from_up)?),
    };
    if !exists {
        return Err(BmsEditError::Usage(format!(
            "{kind} def `{from_up}` not found"
        )));
    }
    replace_value_everywhere(&mut doc, &from_up, &to_up)?;
    doc.remove_header_line(&format!("{DEF_PREFIX}{from_up}"))?;
    let filename = match kind {
        "wav" => doc
            .bms
            .audio
            .wav_files
            .get(&wav_index(&from_up)?)
            .cloned()
            .unwrap_or_default(),
        _ => doc
            .bms
            .visual
            .bmp_files
            .get(&bmp_index(&from_up)?)
            .cloned()
            .unwrap_or_default(),
    };
    let header_prefix = match kind {
        "wav" => "WAV",
        _ => "BMP",
    };
    doc.write_header_line(&format!("{header_prefix}{to_up}"), &filename)?;
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("renamed {kind} {from_up} -> {to_up}"),
        &serde_json::json!({ "from": from_up, "to": to_up }),
        &doc.warnings,
    );
    Ok(())
}

/// WAV 定义头部命令前缀。
const DEF_PREFIX: &str = "WAV";

/// 检查定义类别合法。
fn check_kind(kind: &str) -> Result<(), BmsEditError> {
    if kind != "wav" && kind != "bmp" {
        return Err(BmsEditError::Usage(format!(
            "unknown def kind `{kind}` (wav|bmp)"
        )));
    }
    Ok(())
}

/// 构造 WAV 定义索引。
fn wav_index(id: &str) -> Result<bmsrs::bms::tokenizer::WavIndex, BmsEditError> {
    bmsrs::bms::tokenizer::WavIndex::from_str(id)
        .map_err(|_err| BmsEditError::Usage(format!("invalid def id `{id}`")))
}

/// 构造 BMP 定义索引。
fn bmp_index(id: &str) -> Result<bmsrs::bms::tokenizer::BmpIndex, BmsEditError> {
    bmsrs::bms::tokenizer::BmpIndex::from_str(id)
        .map_err(|_err| BmsEditError::Usage(format!("invalid def id `{id}`")))
}

/// 全文件替换通道行中的 2 字符值（大小写不敏感匹配）。
fn replace_value_everywhere(doc: &mut BmsDoc, from: &str, to: &str) -> Result<(), BmsEditError> {
    let line_count = doc.document.lines().len();
    for index in 0..line_count {
        let Some(line) = doc.document.lines().get(index) else {
            continue;
        };
        let bmsrs::bms::document::LineKind::Message(msg) = line.kind() else {
            continue;
        };
        let body = msg.body.clone();
        let denom = body.chars().count() / 2;
        if denom == 0 {
            continue;
        }
        let changed = patch::chunk_values(&body).any(|v| v.eq_ignore_ascii_case(from));
        if !changed {
            continue;
        }
        let new_body: String = patch::chunk_values(&body)
            .map(|v| {
                if v.eq_ignore_ascii_case(from) {
                    to.to_owned()
                } else {
                    v.to_owned()
                }
            })
            .collect();
        let text = format!("#{}:{new_body}", msg.addr);
        doc.document
            .replace_content(index, &text)
            .map_err(|err| BmsEditError::Usage(format!("replace line: {err}")))?;
    }
    Ok(())
}

/// `defs dedup` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn defs_dedup(
    file: &Path,
    seed: u64,
    kind: &str,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    check_kind(kind)?;
    let mut doc = load(file, seed)?;
    let defs: Vec<(String, String)> = match kind {
        "wav" => doc
            .bms
            .audio
            .wav_files
            .iter()
            .map(|(id, name)| (id.to_string(), name.clone()))
            .collect(),
        _ => doc
            .bms
            .visual
            .bmp_files
            .iter()
            .map(|(id, name)| (id.to_string(), name.clone()))
            .collect(),
    };
    // 相同文件分组；保留最小 ID。
    let mut by_file: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (id, name) in &defs {
        by_file.entry(name.as_str()).or_default().push(id);
    }
    let mut merged = 0usize;
    for (_, ids) in by_file {
        let mut sorted = ids;
        sorted.sort_unstable();
        let Some((keep, rest)) = sorted.split_first() else {
            continue;
        };
        for dup in rest {
            replace_value_everywhere(&mut doc, dup, keep)?;
            doc.remove_header_line(&format!("{DEF_PREFIX}{dup}"))?;
            merged += 1;
        }
    }
    if merged == 0 {
        return Err(BmsEditError::Usage(
            "no duplicate definitions found".to_owned(),
        ));
    }
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("merged {merged} duplicate definition(s)"),
        &serde_json::json!({ "merged": merged }),
        &doc.warnings,
    );
    Ok(())
}

/// `defs compact` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn defs_compact(
    file: &Path,
    seed: u64,
    kind: &str,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    check_kind(kind)?;
    let doc = load(file, seed)?;
    let defs: Vec<(String, String)> = match kind {
        "wav" => doc
            .bms
            .audio
            .wav_files
            .iter()
            .map(|(id, name)| (id.to_string(), name.clone()))
            .collect(),
        _ => doc
            .bms
            .visual
            .bmp_files
            .iter()
            .map(|(id, name)| (id.to_string(), name.clone()))
            .collect(),
    };
    // 顺序重编号 01, 02, ...；引用同步走兜底重写路径。
    let mut remap: Vec<(String, String)> = Vec::new();
    for (ordinal, (id, _)) in defs.iter().enumerate() {
        let new_id = format!("{:02}", ordinal + 1);
        if &new_id != id {
            remap.push((id.clone(), new_id));
        }
    }
    if remap.is_empty() {
        return Err(BmsEditError::Usage(
            "definition ids are already compact".to_owned(),
        ));
    }
    defs_compact_via_rewrite(file, seed, kind, &remap, out)
}

/// compact 的兜底重写实现。
fn defs_compact_via_rewrite(
    file: &Path,
    seed: u64,
    kind: &str,
    remap: &[(String, String)],
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let mut bms = doc.bms.clone();
    for channels in bms.messages.raw.values_mut() {
        for bodies in channels.values_mut() {
            for body in bodies.iter_mut() {
                let denom = body.chars().count() / 2;
                if denom == 0 {
                    continue;
                }
                let new_body: String = patch::chunk_values(body)
                    .map(|v| {
                        remap
                            .iter()
                            .find(|(old, _)| v.eq_ignore_ascii_case(old))
                            .map_or_else(|| v.to_owned(), |(_, new)| new.clone())
                    })
                    .collect();
                *body = new_body;
            }
        }
    }
    let rename_map: BTreeMap<String, String> = remap.iter().cloned().collect();
    if kind == "wav" {
        let old = bms.audio.wav_files.clone();
        bms.audio.wav_files.clear();
        for (id, name) in old {
            let new_id = rename_map
                .get(&id.to_string())
                .cloned()
                .unwrap_or_else(|| id.to_string());
            bms.audio.wav_files.insert(wav_index(&new_id)?, name);
        }
    } else {
        let old = bms.visual.bmp_files.clone();
        bms.visual.bmp_files.clear();
        for (id, name) in old {
            let new_id = rename_map
                .get(&id.to_string())
                .cloned()
                .unwrap_or_else(|| id.to_string());
            bms.visual.bmp_files.insert(bmp_index(&new_id)?, name);
        }
    }
    let text = bmsrs::bms::writer::BmsWriter::write(&bms);
    let rebuilt = BmsDoc::from_bytes(text.clone().into_bytes(), seed)?;
    let before_count = if kind == "wav" {
        doc.bms.audio.wav_files.len()
    } else {
        doc.bms.visual.bmp_files.len()
    };
    let after_count = if kind == "wav" {
        rebuilt.bms.audio.wav_files.len()
    } else {
        rebuilt.bms.visual.bmp_files.len()
    };
    if after_count < before_count {
        return Err(BmsEditError::Verify("compact lost definitions".to_owned()));
    }
    atomic_write_text(file, &text)?;
    out.result(
        format!("compacted {} definition id(s)", remap.len()),
        &serde_json::json!({ "renumbered": remap.len() }),
        &rebuilt.warnings,
    );
    Ok(())
}

/// 原子写文本。
fn atomic_write_text(path: &Path, text: &str) -> Result<(), BmsEditError> {
    let tmp = path.with_extension("bmsedit-tmp");
    let tmp_display = tmp.clone();
    std::fs::write(&tmp, text.as_bytes()).map_err(|source| BmsEditError::Io {
        path: tmp_display,
        source,
    })?;
    std::fs::rename(&tmp, path).map_err(|source| BmsEditError::Io {
        path: path.to_owned(),
        source,
    })
}

/// `measure insert` 命令：模型层位移 + 兜底重写。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn measure_insert(
    file: &Path,
    seed: u64,
    at: u16,
    count: u16,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    if count == 0 {
        return Err(BmsEditError::Usage("count must be positive".to_owned()));
    }
    let doc = load(file, seed)?;
    let mut bms = doc.bms;
    shift_measures(&mut bms, at, i32::from(count));
    let text = bmsrs::bms::writer::BmsWriter::write(&bms);
    let rebuilt = BmsDoc::from_bytes(text.into_bytes(), seed)?;
    let out_text = bmsrs::bms::writer::BmsWriter::write(&rebuilt.bms);
    atomic_write_text(file, &out_text)?;
    out.result(
        format!("inserted {count} measure(s) before {at}"),
        &serde_json::json!({ "inserted": count, "at": at }),
        &rebuilt.warnings,
    );
    Ok(())
}

/// `measure remove` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn measure_remove(
    file: &Path,
    seed: u64,
    at: u16,
    count: u16,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    if count == 0 {
        return Err(BmsEditError::Usage("count must be positive".to_owned()));
    }
    let doc = load(file, seed)?;
    let mut bms = doc.bms;
    // 先删除区间，再对后续小节左移。
    let removed_keys: Vec<u16> = bms
        .messages
        .raw
        .range(at..at.saturating_add(count))
        .map(|(k, _)| *k)
        .collect();
    for key in removed_keys {
        bms.messages.raw.remove(&key);
    }
    shift_measures(&mut bms, at.saturating_add(count), -i32::from(count));
    let text = bmsrs::bms::writer::BmsWriter::write(&bms);
    let rebuilt = BmsDoc::from_bytes(text.into_bytes(), seed)?;
    let out_text = bmsrs::bms::writer::BmsWriter::write(&rebuilt.bms);
    atomic_write_text(file, &out_text)?;
    out.result(
        format!("removed {count} measure(s) from {at}"),
        &serde_json::json!({ "removed": count, "at": at }),
        &rebuilt.warnings,
    );
    Ok(())
}

/// `measure clone-content` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn measure_clone(
    file: &Path,
    seed: u64,
    from: u16,
    to: u16,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let mut doc = load(file, seed)?;
    let source: Vec<(BmsChannel, SlotMap)> = doc
        .bms
        .messages
        .raw
        .get(&from)
        .map(|channels| {
            channels
                .keys()
                .map(|channel| {
                    let channel_str = channel.as_str().to_string();
                    let slots = patch::group_slots(&doc, from, &channel_str);
                    (*channel, slots)
                })
                .collect()
        })
        .unwrap_or_default();
    if source.is_empty() {
        return Err(BmsEditError::Usage(format!("measure {from} has no data")));
    }
    let mut copied = 0usize;
    for (channel, slots) in &source {
        let channel_str = channel.as_str().to_string();
        let payload = slots.clone();
        edit_group(&mut doc, to, &channel_str, |map| {
            for (pos, value) in payload {
                map.insert(pos, value);
                copied += 1;
            }
        })?;
    }
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("cloned measure {from} -> {to} ({copied} slots)"),
        &serde_json::json!({ "from": from, "to": to, "copied": copied }),
        &doc.warnings,
    );
    Ok(())
}

/// 位移全部小节数据（`delta` 可负）；STP 同步位移。
fn shift_measures(bms: &mut Bms, from: u16, delta: i32) {
    let moved: Vec<(u16, BTreeMap<BmsChannel, Vec<String>>)> = bms
        .messages
        .raw
        .range(from..)
        .map(|(k, v)| (*k, v.clone()))
        .collect();
    for (key, value) in moved {
        bms.messages.raw.remove(&key);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "measure numbers are within u16 range after clamping"
        )]
        let new_key = (i32::from(key) + delta).max(0) as u16;
        bms.messages.raw.insert(new_key, value);
    }
    for event in &mut bms.messages.stp_events {
        let measure = event.position.measure();
        if measure >= from {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "measure numbers are within u16 range after clamping"
            )]
            let new_measure = (i32::from(measure) + delta).max(0) as u16;
            event.position = bmsrs::bms::parser::Position::new(
                new_measure,
                event.position.numer(),
                event.position.denom(),
            );
        }
    }
}

/// `transform mirror` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn transform_mirror(file: &Path, seed: u64, out: &mut Output) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let mut bms = doc.bms.clone();
    let keys: Vec<u8> = doc
        .bms
        .messages
        .note_events
        .iter()
        .filter(|n| n.player == 1)
        .filter_map(|n| LaneRef::from_bme_lane_byte(n.lane))
        .filter_map(|lane| match lane {
            LaneRef::Key(k) => Some(k),
            LaneRef::Scratch(_) => None,
        })
        .collect();
    let keys_max = keys.iter().copied().max().unwrap_or(7);
    let channels: Vec<BmsChannel> = doc
        .bms
        .messages
        .raw
        .values()
        .flat_map(|c| c.keys().copied())
        .collect();
    let mut swaps: Vec<(BmsChannel, BmsChannel)> = Vec::new();
    for channel in &channels {
        let Some((player, lane)) = lane_map::note_channel_to_lane(*channel) else {
            continue;
        };
        let mirror_lane = mirror_lane_of(lane, keys_max);
        if mirror_lane == lane {
            continue;
        }
        let byte = mirror_lane.to_bme_lane_byte().unwrap_or(1);
        let Some(mirrored) = lane_map::parse_channel(&format!("{player}{byte}")) else {
            continue;
        };
        swaps.push((*channel, mirrored));
    }
    swaps.retain(|(a, b)| a != b);
    swaps.sort();
    swaps.dedup();
    for (a, b) in &swaps {
        swap_channels(&mut bms, *a, *b);
    }
    let text = bmsrs::bms::writer::BmsWriter::write(&bms);
    let rebuilt = BmsDoc::from_bytes(text.into_bytes(), seed)?;
    let out_text = bmsrs::bms::writer::BmsWriter::write(&rebuilt.bms);
    atomic_write_text(file, &out_text)?;
    out.result(
        format!(
            "mirrored ({} channel pair(s), keys={keys_max})",
            swaps.len()
        ),
        &serde_json::json!({ "pairs": swaps.len() }),
        &rebuilt.warnings,
    );
    Ok(())
}

/// 键数镜像的对称 lane。
const fn mirror_lane_of(lane: LaneRef, keys_max: u8) -> LaneRef {
    match lane {
        LaneRef::Key(k) if k <= keys_max => LaneRef::Key(keys_max + 1 - k),
        other => other,
    }
}

/// 交换两个通道的数据。
fn swap_channels(bms: &mut Bms, a: BmsChannel, b: BmsChannel) {
    let measures: Vec<u16> = bms.messages.raw.keys().copied().collect();
    for measure in measures {
        let Some(channels) = bms.messages.raw.get_mut(&measure) else {
            continue;
        };
        let a_body = channels.remove(&a);
        let b_body = channels.remove(&b);
        match (a_body, b_body) {
            (Some(x), Some(y)) => {
                channels.insert(a, y);
                channels.insert(b, x);
            }
            (Some(x), None) => {
                channels.insert(b, x);
            }
            (None, Some(y)) => {
                channels.insert(a, y);
            }
            (None, None) => {}
        }
    }
}

/// `transform sparsify` 命令：非网格槽位置零。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn transform_sparsify(
    file: &Path,
    seed: u64,
    grid: u32,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    if grid == 0 {
        return Err(BmsEditError::Usage("grid must be positive".to_owned()));
    }
    let mut doc = load(file, seed)?;
    let line_count = doc.document.lines().len();
    let mut removed = 0usize;
    for index in 0..line_count {
        let Some(line) = doc.document.lines().get(index) else {
            continue;
        };
        let msg = match line.kind() {
            bmsrs::bms::document::LineKind::Message(msg) => msg.clone(),
            _ => continue,
        };
        // 仅处理可玩音符通道；BGM 与控制通道保留。
        if !matches!(msg.channel(), BmsChannel::Note(_)) {
            continue;
        }
        let denom = msg.body.chars().count() / 2;
        if denom == 0 {
            continue;
        }
        let before = msg.body.clone();
        let mut new_body = String::with_capacity(before.len());
        for (slot, value) in patch::chunk_values(&msg.body).enumerate() {
            let on_grid =
                (slot as u64 * u64::from(grid)).is_multiple_of(u64::try_from(denom).unwrap_or(1));
            if on_grid || value == "00" {
                new_body.push_str(value);
            } else {
                removed += 1;
                new_body.push_str("00");
            }
        }
        if new_body == before {
            continue;
        }
        let text = format!("#{}:{new_body}", msg.addr);
        doc.document
            .replace_content(index, &text)
            .map_err(|err| BmsEditError::Usage(format!("replace line: {err}")))?;
    }
    if removed == 0 {
        return Err(BmsEditError::Usage(
            "no off-grid notes to remove".to_owned(),
        ));
    }
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("removed {removed} off-grid note(s) (grid=1/{grid})"),
        &serde_json::json!({ "removed": removed }),
        &doc.warnings,
    );
    Ok(())
}

/// `validate` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn validate(file: &Path, seed: u64, out: &mut Output) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let (chart, process_warnings) = bmsrs::bms::processor::BmsProcessor::process_with_warnings::<
        bmsrs::bms::processor::layout::Bme,
    >(&doc.bms)
    .map_err(|err| BmsEditError::Parse(format!("process: {err}")))?;
    let warnings: Vec<String> = process_warnings.iter().map(ToString::to_string).collect();
    match chart.validate() {
        Ok(()) => {
            let human = format!("valid ({} warning(s))", warnings.len());
            out.result(human, &serde_json::json!({ "valid": true }), &warnings);
            Ok(())
        }
        Err(err) => Err(BmsEditError::Verify(err.to_string())),
    }
}

/// `lint` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn lint(file: &Path, seed: u64, out: &mut Output) -> Result<(), BmsEditError> {
    let doc = load(file, seed)?;
    let bms = &doc.bms;
    let refs = reference_counts(&doc);
    let mut findings: Vec<serde_json::Value> = Vec::new();
    for (id, name) in &bms.audio.wav_files {
        if refs.get(&id.to_string()).copied().unwrap_or(0) == 0 {
            findings.push(serde_json::json!({
                "kind": "unused-def",
                "detail": format!("wav {id} -> {name} is never referenced"),
            }));
        }
    }
    for (id, name) in &bms.visual.bmp_files {
        if refs.get(&id.to_string()).copied().unwrap_or(0) == 0 {
            findings.push(serde_json::json!({
                "kind": "unused-def",
                "detail": format!("bmp {id} -> {name} is never referenced"),
            }));
        }
    }
    // LN 通道奇数非 00 值（LNTYPE 1 未配对）。
    for (measure, channels) in &bms.messages.raw {
        for (channel, bodies) in channels {
            let name = channel.as_str();
            if name.starts_with('5') || name.starts_with('6') {
                let count: usize = bodies
                    .iter()
                    .map(|body| patch::chunk_values(body).filter(|v| *v != "00").count())
                    .sum();
                if count % 2 == 1 {
                    findings.push(serde_json::json!({
                        "kind": "unpaired-ln",
                        "detail": format!("measure {measure} ch {name} has {count} values (odd)"),
                    }));
                }
            }
        }
    }
    let human = if findings.is_empty() {
        "no issues found".to_owned()
    } else {
        findings
            .iter()
            .filter_map(|f| f["detail"].as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    out.result(human, &findings, &doc.warnings);
    Ok(())
}

/// `timing scroll-remove` 命令：移除指定位置的 SCROLL 倍率变更。
///
/// # Errors
///
/// 该位置无 SCROLL 事件或行编辑失败时返回 [`BmsEditError`]。
pub fn timing_scroll_remove(
    file: &Path,
    seed: u64,
    at: &str,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let spot = spot_of(at)?;
    let mut doc = load(file, seed)?;
    let mut removed = false;
    edit_group(&mut doc, spot.measure, "SC", |map| {
        removed = map.remove(&spot.pos).is_some();
    })?;
    if !removed {
        return Err(BmsEditError::Usage(format!("no scroll event at `{at}`")));
    }
    doc.commit(file, |_| Ok(()))?;
    out.result(
        format!("scroll removed at `{at}`"),
        &serde_json::json!({ "removed": true }),
        &doc.warnings,
    );
    Ok(())
}
