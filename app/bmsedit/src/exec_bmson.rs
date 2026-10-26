//! bmson 后端命令执行。
//!
//! bmson 是 JSON 格式：编辑落在 `bmsrs::bmson::def::Bmson`（v2 root schema）
//! 的结构体字段上，写回经 serde 全量序列化；v0/v1 文件升版为 v2。
//!
//! # 借用模型
//!
//! `Bmson<'a>` 的字符串字段借用 JSON 字节。CLI 传入的新字符串
//! （音频文件名、标题等）无法直接借用，统一经内部编辑入口的
//! **字符串池**处理：原文档与池值包裹为 JSON 数组
//! `[文档, [池值...]]`，serde tuple 反序列化使所有字符串共享
//! 同一份字节的生命周期；写回时只序列化谱面本体，池自然消失。

use std::path::Path;

use bmsrs::bmson::def::SoundChannel;
use bmsrs::bmson::def::{Bmson, NoteEvent};
use serde::Serialize;

use crate::cli::{self, Spot};
use crate::error::BmsEditError;
use crate::lane_map::LaneRef;
use crate::output::Output;

/// 解析 bmson 字节（任意版本升到 v2）。
fn parse(bytes: &[u8]) -> Result<Bmson<'_>, BmsEditError> {
    let json = std::str::from_utf8(bytes)
        .map_err(|err| BmsEditError::Parse(format!("not UTF-8: {err}")))?;
    match bmsrs::bmson::def::DetectedVersion::detect(json) {
        Ok(bmsrs::bmson::def::DetectedVersion::V2) => {
            serde_json::from_str(json).map_err(|err| BmsEditError::Parse(err.to_string()))
        }
        Ok(bmsrs::bmson::def::DetectedVersion::V1) => {
            let v1: bmsrs::bmson::def::v1::Bmson<'_> =
                serde_json::from_str(json).map_err(|err| BmsEditError::Parse(err.to_string()))?;
            Ok(Bmson::from(v1))
        }
        Ok(bmsrs::bmson::def::DetectedVersion::V0) => {
            let v0: bmsrs::bmson::def::v0::Bmson<'_> =
                serde_json::from_str(json).map_err(|err| BmsEditError::Parse(err.to_string()))?;
            let v2 = Bmson::try_from(v0).map_err(|err| BmsEditError::Parse(err.to_string()))?;
            Ok(v2)
        }
        Err(err) => Err(BmsEditError::Parse(err.to_string())),
    }
}

/// 原子写文本。
fn atomic_write(path: &Path, text: &str) -> Result<(), BmsEditError> {
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

/// 序列化写回并重新解析验证。
fn write_back(file: &Path, bmson: &Bmson<'_>) -> Result<(), BmsEditError> {
    let text =
        serde_json::to_string_pretty(bmson).map_err(|err| BmsEditError::Verify(err.to_string()))?;
    atomic_write(file, &text)?;
    let bytes = std::fs::read(file).map_err(|source| BmsEditError::Io {
        path: file.to_owned(),
        source,
    })?;
    parse(&bytes)?;
    Ok(())
}

/// bmson 命令的统一入口。
///
/// 加载文件字节，把 `extras` 中的字符串注入字节池（可从结果借用），
/// 解析为 v2 模型后交给 `f` 编辑；`f` 返回后序列化写回并验证。
fn with_bmson<T>(
    file: &Path,
    extras: &[&str],
    f: impl for<'a> FnOnce(&mut Bmson<'a>, &[&'a str]) -> Result<T, BmsEditError>,
) -> Result<T, BmsEditError> {
    let original = std::fs::read(file).map_err(|source| BmsEditError::Io {
        path: file.to_owned(),
        source,
    })?;
    // 包裹数组：`[原文档, "extra1", ...]`——extra 借池文本，写回时
    // 只序列化谱面本体，池自然消失。
    let mut pooled_bytes = Vec::with_capacity(original.len() + 32);
    pooled_bytes.push(b'[');
    pooled_bytes.extend_from_slice(&original);
    pooled_bytes.push(b',');
    pooled_bytes.push(b'[');
    for (index, extra) in extras.iter().enumerate() {
        if index > 0 {
            pooled_bytes.push(b',');
        }
        let encoded = serde_json::to_string(extra)
            .map_err(|err| BmsEditError::Usage(format!("unencodable string: {err}")))?;
        pooled_bytes.extend_from_slice(encoded.as_bytes());
    }
    pooled_bytes.push(b']');
    pooled_bytes.push(b']');
    let (mut bmson, pooled): (Bmson<'_>, Vec<&str>) = serde_json::from_slice(&pooled_bytes)
        .map_err(|err| BmsEditError::Parse(format!("pool parse: {err}")))?;
    let result = f(&mut bmson, &pooled)?;
    write_back(file, &bmson)?;
    Ok(result)
}

/// 只读访问（无写回）。
fn with_bmson_read<T>(
    file: &Path,
    f: impl FnOnce(&Bmson<'_>) -> Result<T, BmsEditError>,
) -> Result<T, BmsEditError> {
    let bytes = std::fs::read(file).map_err(|source| BmsEditError::Io {
        path: file.to_owned(),
        source,
    })?;
    let bmson = parse(&bytes)?;
    f(&bmson)
}

/// 谱面的脉冲区间序列（第 i 项 = 小节 i 的 `[start, end)`）。
fn measure_bounds(bmson: &Bmson<'_>) -> Vec<(u64, u64)> {
    let res = bmson.chart_data.resolution;
    let default_span = res * 4;
    let Some(lines) = &bmson.chart_data.lines else {
        // 无小节线：均匀 4/4。
        let count = max_y(bmson) / default_span + 8;
        return (0..count)
            .map(|i| (i * default_span, (i + 1) * default_span))
            .collect();
    };
    let mut ys: Vec<u64> = lines.iter().map(|line| line.y).collect();
    if ys.first() != Some(&0) {
        ys.insert(0, 0);
    }
    let mut bounds: Vec<(u64, u64)> = Vec::with_capacity(ys.len() + 1);
    #[expect(
        clippy::indexing_slicing,
        reason = "windows(2) guarantees exactly two elements per slice"
    )]
    for window in ys.windows(2) {
        bounds.push((window[0], window[1]));
    }
    let last_start = *ys.last().unwrap_or(&0);
    bounds.push((last_start, last_start + default_span));
    bounds
}

/// 全部事件的最大脉冲位置。
fn max_y(bmson: &Bmson<'_>) -> u64 {
    let notes = bmson
        .chart_data
        .sound_channels
        .iter()
        .flat_map(|channel| channel.note_events.iter().map(|note| note.y + note.l));
    let bpm = bmson.chart_data.bpm_events.iter().map(|event| event.y);
    let stops = bmson.chart_data.stop_events.iter().map(|event| event.y);
    notes.chain(bpm).chain(stops).max().unwrap_or(0)
}

/// 脉冲 → （小节，约分位置）。
fn y_to_spot(bmson: &Bmson<'_>, y: u64) -> (u16, (u32, u32)) {
    for (index, (start, end)) in measure_bounds(bmson).into_iter().enumerate() {
        if y >= start && y < end {
            let numer = u32::try_from(y - start).unwrap_or(u32::MAX);
            let denom = u32::try_from(end - start).unwrap_or(1);
            let g = gcd(numer.max(1), denom);
            return (
                u16::try_from(index).unwrap_or(u16::MAX),
                (numer / g, denom / g),
            );
        }
    }
    (u16::MAX, (0, 1))
}

/// （小节，位置）→ 脉冲。
fn spot_to_y(bmson: &Bmson<'_>, measure: u16, pos: (u32, u32)) -> Result<u64, BmsEditError> {
    let bounds = measure_bounds(bmson);
    let (start, end) = bounds
        .get(usize::from(measure))
        .copied()
        .ok_or_else(|| BmsEditError::Usage(format!("measure {measure} is outside the chart")))?;
    let span = end - start;
    Ok(start + u64::from(pos.0) * span / u64::from(pos.1))
}

/// 最大公约数。
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// 位置分数格式化。
fn pos_str(numer: u32, denom: u32) -> String {
    if denom == 1 {
        numer.to_string()
    } else {
        format!("{numer}/{denom}")
    }
}

/// lane 显示形式。
fn lane_str(lane: LaneRef) -> String {
    match lane {
        LaneRef::Key(n) => n.to_string(),
        LaneRef::Scratch(_) => "sc".to_owned(),
    }
}

/// bmson `x` 的双向映射（按 `mode_hint` 选择布局）。
mod xmap {
    use crate::error::BmsEditError;
    use crate::lane_map::LaneRef;

    /// lane → bmson `x`。
    pub fn to_x(bmson: &bmsrs::bmson::def::Bmson<'_>, lane: LaneRef) -> Result<u64, BmsEditError> {
        let is_popn = is_popn_hint(bmson);
        let mapped = if is_popn {
            match lane {
                LaneRef::Key(n) if (1..=9).contains(&n) => Some(u64::from(n)),
                _ => None,
            }
        } else {
            lane.to_bmson_beat_x()
        };
        mapped.ok_or_else(|| {
            BmsEditError::Usage(format!(
                "lane {} has no x mapping for this mode",
                lane_str_of(lane)
            ))
        })
    }

    /// bmson `x` → lane。
    pub fn from_x(bmson: &bmsrs::bmson::def::Bmson<'_>, x: u64) -> Option<LaneRef> {
        if is_popn_hint(bmson) {
            LaneRef::from_bmson_popn_x(x)
        } else {
            LaneRef::from_bmson_beat_x(x)
        }
    }

    /// 是否为 popn 系模式提示。
    pub const fn is_popn_hint(bmson: &bmsrs::bmson::def::Bmson<'_>) -> bool {
        matches!(
            bmson.chart_data.mode_hint,
            bmsrs::bmson::def::ModeHint::Popn5k | bmsrs::bmson::def::ModeHint::Popn9k
        )
    }

    /// lane 显示形式。
    fn lane_str_of(lane: LaneRef) -> String {
        match lane {
            LaneRef::Key(n) => n.to_string(),
            LaneRef::Scratch(_) => "sc".to_owned(),
        }
    }
}

/// lane 参数解析。
fn lane_of(s: &str) -> Result<LaneRef, BmsEditError> {
    cli::parse_lane(s).map_err(BmsEditError::Usage)
}

/// 位置解析。
fn spot_of(s: &str) -> Result<Spot, BmsEditError> {
    cli::parse_spot(s).map_err(BmsEditError::Usage)
}

/// 谱面统计信息。
#[derive(Serialize)]
pub struct ChartInfoOut {
    /// 格式标识。
    format: &'static str,
    /// 标题。
    title: String,
    /// 艺术家。
    artist: String,
    /// 流派。
    genre: String,
    /// 模式提示。
    mode: String,
    /// 分辨率。
    resolution: u64,
    /// 可玩音符数。
    notes: usize,
    /// BGM 音符数。
    bgm_notes: usize,
    /// 长音数。
    long_notes: usize,
    /// 音频通道数。
    channels: usize,
    /// BPM 事件数。
    bpm_events: usize,
    /// 停止事件数。
    stop_events: usize,
    /// 小节数。
    measures: u16,
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
    /// 音频文件。
    def: String,
    /// 种类。
    kind: &'static str,
}

/// 时序条目。
#[derive(Serialize)]
pub struct TimingOut {
    /// 小节号。
    measure: u16,
    /// 位置。
    pos: String,
    /// 类别。
    channel: String,
    /// 值。
    value: String,
}

/// `info` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn info(file: &Path, out: &mut Output) -> Result<(), BmsEditError> {
    with_bmson_read(file, |bmson| {
        let mut bgm = 0usize;
        let mut playable = 0usize;
        let mut long = 0usize;
        for channel in &bmson.chart_data.sound_channels {
            for note in &channel.note_events {
                if note.x == 0 {
                    bgm += 1;
                } else {
                    playable += 1;
                    if note.l > 0 {
                        long += 1;
                    }
                }
            }
        }
        let mode = bmson.chart_data.mode_hint.to_string();
        let chart_info = ChartInfoOut {
            format: "bmson",
            title: bmson.song_info.title.to_owned(),
            artist: bmson.song_info.artist.to_owned(),
            genre: bmson.song_info.genre.to_owned(),
            mode,
            resolution: bmson.chart_data.resolution,
            notes: playable,
            bgm_notes: bgm,
            long_notes: long,
            channels: bmson.chart_data.sound_channels.len(),
            bpm_events: bmson.chart_data.bpm_events.len(),
            stop_events: bmson.chart_data.stop_events.len(),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "measure counts below 65536 in practice"
            )]
            measures: measure_bounds(bmson).len() as u16,
        };
        let human = format!(
            "title: {}\nartist: {}\ngenre: {}\nmode: {} (resolution {})\nnotes: {} (bgm {}, ln {})\nchannels: {}\nbpm events: {} / stops: {}\nmeasures: {}",
            chart_info.title,
            chart_info.artist,
            chart_info.genre,
            chart_info.mode,
            chart_info.resolution,
            chart_info.notes,
            chart_info.bgm_notes,
            chart_info.long_notes,
            chart_info.channels,
            chart_info.bpm_events,
            chart_info.stop_events,
            chart_info.measures,
        );
        out.result(human, &chart_info, &[]);
        Ok(())
    })
}

/// `header get` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn header_get(file: &Path, key: Option<&str>, out: &mut Output) -> Result<(), BmsEditError> {
    with_bmson_read(file, |bmson| {
        let pairs: Vec<(String, String)> = vec![
            ("title".to_owned(), bmson.song_info.title.to_owned()),
            ("artist".to_owned(), bmson.song_info.artist.to_owned()),
            ("genre".to_owned(), bmson.song_info.genre.to_owned()),
            ("subtitle".to_owned(), bmson.chart_info.subtitle.to_owned()),
            (
                "chart_name".to_owned(),
                bmson.chart_info.chart_name.to_owned(),
            ),
            ("level".to_owned(), bmson.chart_info.level.to_string()),
        ];
        let filtered: Vec<_> = match key {
            Some(k) => {
                let target = k.to_ascii_lowercase();
                let hit: Vec<_> = pairs
                    .into_iter()
                    .filter(|(name, _)| *name == target)
                    .collect();
                if hit.is_empty() {
                    return Err(BmsEditError::Usage(format!("header `{k}` not found")));
                }
                hit
            }
            None => pairs,
        };
        let json: Vec<serde_json::Value> = filtered
            .iter()
            .map(|(name, value)| serde_json::json!({ "key": name, "value": value }))
            .collect();
        let human = filtered
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");
        out.result(human, &json, &[]);
        Ok(())
    })
}

/// `header set` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn header_set(file: &Path, pairs: &[String], out: &mut Output) -> Result<(), BmsEditError> {
    let parsed = parse_pairs(pairs)?;
    let extras: Vec<&str> = parsed.iter().map(|(_, value)| value.as_str()).collect();
    let keys: Vec<String> = parsed
        .iter()
        .map(|(key, _)| key.to_ascii_lowercase())
        .collect();
    with_bmson(file, &extras, |bmson, pool| {
        for (key, &value) in keys.iter().zip(pool) {
            match key.as_str() {
                "title" => bmson.song_info.title = value,
                "artist" => bmson.song_info.artist = value,
                "genre" => bmson.song_info.genre = value,
                "subtitle" => bmson.chart_info.subtitle = value,
                "chart_name" => bmson.chart_info.chart_name = value,
                _ => {
                    return Err(BmsEditError::Usage(format!(
                        "unknown bmson header `{key}` (title|artist|genre|subtitle|chart_name)"
                    )));
                }
            }
        }
        Ok(())
    })?;
    let applied: Vec<String> = parsed
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    out.result(format!("set {}", applied.join(", ")), &applied, &[]);
    Ok(())
}

/// 解析 `KEY=VALUE` 参数对。
fn parse_pairs(pairs: &[String]) -> Result<Vec<(String, String)>, BmsEditError> {
    pairs
        .iter()
        .map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .ok_or_else(|| BmsEditError::Usage(format!("expected KEY=VALUE, got `{pair}`")))
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
    from_measure: Option<u16>,
    to_measure: Option<u16>,
    lane: Option<&str>,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let lane_filter = lane.map(lane_of).transpose()?;
    with_bmson_read(file, |bmson| {
        let mut notes: Vec<NoteOut> = Vec::new();
        for channel in &bmson.chart_data.sound_channels {
            for note in &channel.note_events {
                let kind = if note.x == 0 {
                    "bgm"
                } else if note.l > 0 {
                    "ln"
                } else {
                    "visible"
                };
                let lane_text =
                    xmap::from_x(bmson, note.x).map_or_else(|| format!("x{}", note.x), lane_str);
                notes.push(NoteOut {
                    measure: y_to_spot(bmson, note.y).0,
                    pos: pos_str(y_to_spot(bmson, note.y).1.0, y_to_spot(bmson, note.y).1.1),
                    lane: lane_text,
                    def: channel.name.display().to_string(),
                    kind,
                });
            }
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
        out.result(human_text, &notes, &[]);
        Ok(())
    })
}

/// `notes add` 的参数集。
pub struct NotesAddArgs<'a> {
    /// 小节号。
    pub measure: u16,
    /// 逻辑 lane。
    pub lane: &'a str,
    /// 位置。
    pub pos: &'a str,
    /// 音频文件。
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
    args: &NotesAddArgs<'_>,
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
    if bgm && ln_to.is_some() {
        return Err(BmsEditError::Usage(
            "bgm notes cannot be long notes".to_owned(),
        ));
    }
    let slot_pos = cli::parse_pos(pos).map_err(BmsEditError::Usage)?;
    let slot_lane = lane_of(lane)?;
    let ln_end = ln_to.map(spot_of).transpose()?.map(|spot| spot.pos);
    let extras = [def];
    with_bmson(file, &extras, |bmson: &mut Bmson<'_>, pool: &[&str]| {
        let Some(def_name) = pool.first().copied() else {
            return Err(BmsEditError::Usage(
                "internal: missing pooled def".to_owned(),
            ));
        };
        let def_path = Path::new(def_name);
        let x = if bgm {
            0
        } else {
            xmap::to_x(bmson, slot_lane)?
        };
        let start_y = spot_to_y(bmson, measure, slot_pos)?;
        let length = match ln_end {
            Some(end) => {
                let end_y = spot_to_y(bmson, measure, end)?;
                if end_y <= start_y {
                    return Err(BmsEditError::Usage(
                        "long note end must be after start".to_owned(),
                    ));
                }
                end_y - start_y
            }
            None => 0,
        };
        let note = NoteEvent {
            x,
            y: start_y,
            l: length,
            c: false,
            t: None,
            up: None,
            ln_type_hint: None,
            ln_judge_hint: None,
            ln_life_hint: None,
            vol: None,
            pan: None,
        };
        let existing = bmson
            .chart_data
            .sound_channels
            .iter_mut()
            .find(|channel| channel.name == def_path);
        match existing {
            Some(channel) => channel.note_events.push(note),
            None => bmson.chart_data.sound_channels.push(SoundChannel {
                name: def_path,
                note_events: vec![note],
            }),
        }
        Ok(())
    })?;
    out.result(
        format!(
            "added {def} at #{measure:03} {}/{} lane {}",
            slot_pos.0,
            slot_pos.1,
            lane_str(slot_lane)
        ),
        &serde_json::json!({ "measure": measure, "lane": lane_str(slot_lane) }),
        &[],
    );
    Ok(())
}

/// `notes remove` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn notes_remove(
    file: &Path,
    measure: u16,
    lane: &str,
    pos: Option<&str>,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let slot_lane = lane_of(lane)?;
    let slot_pos = pos
        .map(cli::parse_pos)
        .transpose()
        .map_err(BmsEditError::Usage)?;
    let mut removed = 0usize;
    with_bmson(file, &[], |bmson, _| {
        let targets: Vec<(usize, usize)> = bmson
            .chart_data
            .sound_channels
            .iter()
            .enumerate()
            .flat_map(|(ci, channel)| {
                channel
                    .note_events
                    .iter()
                    .enumerate()
                    .filter(|(_, note)| note_matches(bmson, note, measure, slot_lane, slot_pos))
                    .map(move |(ni, _)| (ci, ni))
            })
            .collect();
        removed = targets.len();
        if removed == 0 {
            return Err(BmsEditError::Usage(format!(
                "no note to remove at #{measure:03} lane {}",
                lane_str(slot_lane)
            )));
        }
        #[expect(
            clippy::indexing_slicing,
            reason = "indices were collected from the same channels above"
        )]
        for (ci, ni) in targets.into_iter().rev() {
            bmson.chart_data.sound_channels[ci].note_events.remove(ni);
        }
        Ok(())
    })?;
    out.result(
        format!(
            "removed {removed} note(s) at #{measure:03} lane {}",
            lane_str(slot_lane)
        ),
        &serde_json::json!({ "removed": removed }),
        &[],
    );
    Ok(())
}

/// 音符是否匹配过滤条件。
fn note_matches(
    bmson: &Bmson<'_>,
    note: &NoteEvent,
    measure: u16,
    lane: LaneRef,
    pos: Option<(u32, u32)>,
) -> bool {
    let Ok(x) = xmap::to_x(bmson, lane) else {
        return false;
    };
    if note.x != x {
        return false;
    }
    let (note_measure, note_pos) = y_to_spot(bmson, note.y);
    if note_measure != measure {
        return false;
    }
    pos.is_none_or(|p| note_pos == p)
}

/// `notes move` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn notes_move(
    file: &Path,
    lane: &str,
    from: &str,
    to: &str,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let slot_lane = lane_of(lane)?;
    let from_spot = spot_of(from)?;
    let to_spot = spot_of(to)?;
    with_bmson(file, &[], |bmson, _| {
        let mut moved = 0usize;
        let target_x = xmap::to_x(bmson, slot_lane)?;
        let new_y = spot_to_y(bmson, to_spot.measure, to_spot.pos)?;
        let targets: Vec<(usize, u64)> = bmson
            .chart_data
            .sound_channels
            .iter()
            .enumerate()
            .flat_map(|(ci, channel)| {
                channel
                    .note_events
                    .iter()
                    .filter(|note| note.x == target_x)
                    .filter(|note| {
                        let (m, p) = y_to_spot(bmson, note.y);
                        m == from_spot.measure && p == from_spot.pos
                    })
                    .map(move |note| (ci, note.y))
            })
            .collect();
        #[expect(
            clippy::indexing_slicing,
            reason = "indices were collected from the same channels above"
        )]
        for (ci, y) in targets {
            for note in &mut bmson.chart_data.sound_channels[ci].note_events {
                if note.y == y {
                    note.y = new_y;
                    moved += 1;
                }
            }
        }
        if moved == 0 {
            return Err(BmsEditError::Usage(format!(
                "no note at #{:03} {}/{}",
                from_spot.measure, from_spot.pos.0, from_spot.pos.1
            )));
        }
        Ok(())
    })?;
    out.result(
        format!(
            "moved to #{:03} {}/{}",
            to_spot.measure, to_spot.pos.0, to_spot.pos.1
        ),
        &serde_json::json!({ "moved": 1 }),
        &[],
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
    with_bmson(file, &[], |bmson, _| {
        let bounds = measure_bounds(bmson);
        let (src_start, _) = *bounds.get(usize::from(from_measure)).ok_or_else(|| {
            BmsEditError::Usage(format!("measure {from_measure} is outside the chart"))
        })?;
        let (dst_start, _) = *bounds.get(usize::from(dest_measure)).ok_or_else(|| {
            BmsEditError::Usage(format!("measure {dest_measure} is outside the chart"))
        })?;
        let shift = dst_start.cast_signed() - src_start.cast_signed();
        let range_start = spot_to_y(bmson, from_measure, (0, 1))?;
        let range_end = spot_to_y(bmson, to_measure, (1, 1))?;
        let mut clones: Vec<(usize, NoteEvent)> = Vec::new();
        for (ci, channel) in bmson.chart_data.sound_channels.iter().enumerate() {
            for note in &channel.note_events {
                if note.y >= range_start && note.y < range_end {
                    let mut copy = note.clone();
                    copy.y = (i64::try_from(note.y).unwrap_or(i64::MAX) + shift)
                        .max(0)
                        .cast_unsigned();
                    clones.push((ci, copy));
                }
            }
        }
        let count = clones.len();
        #[expect(
            clippy::indexing_slicing,
            reason = "channel indices were collected from the same vector above"
        )]
        for (ci, note) in clones {
            bmson.chart_data.sound_channels[ci].note_events.push(note);
        }
        if count == 0 {
            return Err(BmsEditError::Usage("source range has no notes".to_owned()));
        }
        Ok(())
    })?;
    out.result(
        format!("copied notes to measure {dest_measure}+"),
        &serde_json::json!({ "dest": dest_measure }),
        &[],
    );
    Ok(())
}

/// `timing list` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_list(file: &Path, out: &mut Output) -> Result<(), BmsEditError> {
    with_bmson_read(file, |bmson| {
        let mut rows: Vec<TimingOut> = Vec::new();
        for event in &bmson.chart_data.bpm_events {
            let (measure, pos) = y_to_spot(bmson, event.y);
            rows.push(TimingOut {
                measure,
                pos: pos_str(pos.0, pos.1),
                channel: "BPM".to_owned(),
                value: event.bpm.to_string(),
            });
        }
        for event in &bmson.chart_data.stop_events {
            let (measure, pos) = y_to_spot(bmson, event.y);
            rows.push(TimingOut {
                measure,
                pos: pos_str(pos.0, pos.1),
                channel: "STOP".to_owned(),
                value: format!("{} pulse", event.duration),
            });
        }
        for event in &bmson.scroll_events {
            let (measure, pos) = y_to_spot(bmson, event.y);
            rows.push(TimingOut {
                measure,
                pos: pos_str(pos.0, pos.1),
                channel: "SCROLL".to_owned(),
                value: event.rate.to_string(),
            });
        }
        let human = rows
            .iter()
            .map(|r| format!("#{:03} {} {}={}", r.measure, r.pos, r.channel, r.value))
            .collect::<Vec<_>>()
            .join("\n");
        out.result(human, &rows, &[]);
        Ok(())
    })
}

/// `timing bpm` 命令：设置 `y: 0` 的 BPM 事件。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_bpm(file: &Path, bpm: f64, out: &mut Output) -> Result<(), BmsEditError> {
    with_bmson(file, &[], |bmson, _| {
        if let Some(event) = bmson
            .chart_data
            .bpm_events
            .iter_mut()
            .find(|event| event.y == 0)
        {
            event.bpm = bpm;
        } else {
            let mut events = std::mem::take(&mut bmson.chart_data.bpm_events);
            events.insert(0, bmsrs::bmson::def::BpmEvent { y: 0, bpm });
            bmson.chart_data.bpm_events = events;
        }
        Ok(())
    })?;
    out.result(
        format!("bpm = {bpm}"),
        &serde_json::json!({ "bpm": bpm }),
        &[],
    );
    Ok(())
}

/// `timing stop-add` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_stop_add(
    file: &Path,
    at: &str,
    duration_pulse: u64,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let spot = spot_of(at)?;
    with_bmson(file, &[], |bmson, _| {
        let y = spot_to_y(bmson, spot.measure, spot.pos)?;
        bmson
            .chart_data
            .stop_events
            .push(bmsrs::bmson::def::StopEvent {
                y,
                duration: duration_pulse,
            });
        Ok(())
    })?;
    out.result(
        format!("stop inserted at `{at}` ({duration_pulse} pulse)"),
        &serde_json::json!({ "at": at, "duration": duration_pulse }),
        &[],
    );
    Ok(())
}

/// `timing stop-remove` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn timing_stop_remove(file: &Path, at: &str, out: &mut Output) -> Result<(), BmsEditError> {
    let spot = spot_of(at)?;
    with_bmson(file, &[], |bmson, _| {
        let y = spot_to_y(bmson, spot.measure, spot.pos)?;
        let before = bmson.chart_data.stop_events.len();
        bmson.chart_data.stop_events.retain(|event| event.y != y);
        if bmson.chart_data.stop_events.len() == before {
            return Err(BmsEditError::Usage(format!("no stop at `{at}`")));
        }
        Ok(())
    })?;
    out.result(
        format!("stop removed at `{at}`"),
        &serde_json::json!({ "removed": true }),
        &[],
    );
    Ok(())
}

/// `defs` 类命令：bmson 无定义表。
#[must_use]
pub fn defs_unsupported() -> BmsEditError {
    BmsEditError::Unsupported(
        "bmson has no definition table; notes reference audio files directly".to_owned(),
    )
}

/// 位移全部事件脉冲（小节结构操作的基础）。
fn shift_events_y(bmson: &mut Bmson<'_>, from_pulse: u64, delta: i64) {
    let shift = |y: &mut u64| {
        if *y >= from_pulse {
            *y = ((*y).cast_signed() + delta).max(0).cast_unsigned();
        }
    };
    for channel in &mut bmson.chart_data.sound_channels {
        for note in &mut channel.note_events {
            shift(&mut note.y);
        }
    }
    for event in &mut bmson.chart_data.bpm_events {
        shift(&mut event.y);
    }
    for event in &mut bmson.chart_data.stop_events {
        shift(&mut event.y);
    }
    for event in &mut bmson.scroll_events {
        shift(&mut event.y);
    }
    for channel in &mut bmson.mine_channels {
        for note in &mut channel.notes {
            shift(&mut note.y);
        }
    }
    for channel in &mut bmson.key_channels {
        for note in &mut channel.notes {
            shift(&mut note.y);
        }
    }
    if let Some(lines) = &mut bmson.chart_data.lines {
        for line in lines.iter_mut() {
            shift(&mut line.y);
        }
    }
}

/// `measure insert` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn measure_insert(
    file: &Path,
    at: u16,
    count: u16,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    if count == 0 {
        return Err(BmsEditError::Usage("count must be positive".to_owned()));
    }
    with_bmson(file, &[], |bmson, _| {
        let start = spot_to_y(bmson, at, (0, 1))?;
        let bounds = measure_bounds(bmson);
        let span = bounds
            .get(usize::from(at))
            .map_or(bmson.chart_data.resolution * 4, |(s, e)| e - s);
        shift_events_y(bmson, start, i64::from(count) * span.cast_signed());
        Ok(())
    })?;
    out.result(
        format!("inserted {count} measure(s) before {at}"),
        &serde_json::json!({ "inserted": count, "at": at }),
        &[],
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
    at: u16,
    count: u16,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    if count == 0 {
        return Err(BmsEditError::Usage("count must be positive".to_owned()));
    }
    with_bmson(file, &[], |bmson, _| {
        let start = spot_to_y(bmson, at, (0, 1))?;
        let end = spot_to_y(bmson, at.saturating_add(count), (0, 1))?;
        // 删除区间内的事件。
        let in_range = |y: u64| y >= start && y < end;
        for channel in &mut bmson.chart_data.sound_channels {
            channel.note_events.retain(|note| !in_range(note.y));
        }
        bmson
            .chart_data
            .bpm_events
            .retain(|event| !in_range(event.y));
        bmson
            .chart_data
            .stop_events
            .retain(|event| !in_range(event.y));
        bmson.scroll_events.retain(|event| !in_range(event.y));
        // 后续事件左移。
        shift_events_y(bmson, end, -(end.cast_signed() - start.cast_signed()));
        Ok(())
    })?;
    out.result(
        format!("removed {count} measure(s) from {at}"),
        &serde_json::json!({ "removed": count, "at": at }),
        &[],
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
    from: u16,
    to: u16,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    with_bmson(file, &[], |bmson, _| {
        let src_start = spot_to_y(bmson, from, (0, 1))?;
        let src_end = spot_to_y(bmson, from.saturating_add(1), (0, 1))?;
        let dst_start = spot_to_y(bmson, to, (0, 1))?;
        let shift = dst_start.cast_signed() - src_start.cast_signed();
        let mut clones: Vec<(usize, NoteEvent)> = Vec::new();
        for (ci, channel) in bmson.chart_data.sound_channels.iter().enumerate() {
            for note in &channel.note_events {
                if note.y >= src_start && note.y < src_end {
                    let mut copy = note.clone();
                    copy.y = (i64::try_from(copy.y).unwrap_or(i64::MAX) + shift)
                        .max(0)
                        .cast_unsigned();
                    clones.push((ci, copy));
                }
            }
        }
        let count = clones.len();
        if count == 0 {
            return Err(BmsEditError::Usage(format!("measure {from} has no notes")));
        }
        #[expect(
            clippy::indexing_slicing,
            reason = "channel indices were collected from the same vector above"
        )]
        for (ci, note) in clones {
            bmson.chart_data.sound_channels[ci].note_events.push(note);
        }
        Ok(())
    })?;
    out.result(
        format!("cloned measure {from} -> {to}"),
        &serde_json::json!({ "from": from, "to": to }),
        &[],
    );
    Ok(())
}

/// `transform mirror` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn transform_mirror(file: &Path, out: &mut Output) -> Result<(), BmsEditError> {
    with_bmson(file, &[], |bmson, _| {
        let popn = xmap::is_popn_hint(bmson);
        let mirror = |x: u64| -> u64 {
            if x == 0 {
                return 0;
            }
            if popn {
                match x {
                    1 => 9,
                    2 => 8,
                    3 => 7,
                    4 => 6,
                    6 => 4,
                    7 => 3,
                    8 => 2,
                    9 => 1,
                    other => other,
                }
            } else {
                match x {
                    1 => 7,
                    2 => 6,
                    3 => 5,
                    5 => 3,
                    6 => 2,
                    7 => 1,
                    other => other,
                }
            }
        };
        for channel in &mut bmson.chart_data.sound_channels {
            for note in &mut channel.note_events {
                note.x = mirror(note.x);
            }
        }
        Ok(())
    })?;
    out.result(
        "mirrored".to_owned(),
        &serde_json::json!({ "mirrored": true }),
        &[],
    );
    Ok(())
}

/// `transform sparsify` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn transform_sparsify(file: &Path, grid: u32, out: &mut Output) -> Result<(), BmsEditError> {
    if grid == 0 {
        return Err(BmsEditError::Usage("grid must be positive".to_owned()));
    }
    let mut removed = 0usize;
    with_bmson(file, &[], |bmson, _| {
        let bounds = measure_bounds(bmson);
        for channel in &mut bmson.chart_data.sound_channels {
            let mut keep: Vec<NoteEvent> = Vec::new();
            for note in channel.note_events.drain(..) {
                let on_grid = bounds.iter().any(|(start, end)| {
                    note.y >= *start
                        && note.y < *end
                        && (note.y - start) * u64::from(grid) % (end - start) == 0
                });
                if on_grid {
                    keep.push(note);
                } else {
                    removed += 1;
                }
            }
            channel.note_events = keep;
        }
        Ok(())
    })?;
    if removed == 0 {
        return Err(BmsEditError::Usage(
            "no off-grid notes to remove".to_owned(),
        ));
    }
    out.result(
        format!("removed {removed} off-grid note(s) (grid=1/{grid})"),
        &serde_json::json!({ "removed": removed }),
        &[],
    );
    Ok(())
}

/// `validate` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn validate(file: &Path, out: &mut Output) -> Result<(), BmsEditError> {
    with_bmson_read(file, |bmson| {
        let mut problems: Vec<String> = Vec::new();
        if bmson.chart_data.resolution == 0 {
            problems.push("resolution is zero".to_owned());
        }
        for channel in &bmson.chart_data.sound_channels {
            for note in &channel.note_events {
                if note.l > 0 && note.y.checked_add(note.l).is_none() {
                    problems.push(format!("note y overflow at {}", note.y));
                }
            }
        }
        if problems.is_empty() {
            out.result(
                "valid".to_owned(),
                &serde_json::json!({ "valid": true }),
                &[],
            );
        } else {
            return Err(BmsEditError::Verify(problems.join("; ")));
        }
        Ok(())
    })
}

/// `lint` 命令。
///
/// # Errors
///
/// 文件读取、解析或编辑验证失败时返回 [`BmsEditError`]。
pub fn lint(file: &Path, out: &mut Output) -> Result<(), BmsEditError> {
    with_bmson_read(file, |bmson| {
        let mut findings: Vec<serde_json::Value> = Vec::new();
        // 未引用的音频通道。
        for channel in &bmson.chart_data.sound_channels {
            if channel.note_events.is_empty() {
                findings.push(serde_json::json!({
                    "kind": "empty-channel",
                    "detail": format!("channel {} has no notes", channel.name.display()),
                }));
            }
        }
        // beat 模式的 x 越界。
        if !matches!(
            bmson.chart_data.mode_hint,
            bmsrs::bmson::def::ModeHint::Popn5k | bmsrs::bmson::def::ModeHint::Popn9k
        ) {
            for channel in &bmson.chart_data.sound_channels {
                for note in &channel.note_events {
                    if note.x > 8 {
                        findings.push(serde_json::json!({
                            "kind": "lane-out-of-range",
                            "detail": format!("channel {} has note with x={} (>8)", channel.name.display(), note.x),
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
                .map(|f| f["detail"].as_str().unwrap_or("").to_owned())
                .collect::<Vec<_>>()
                .join("\n")
        };
        out.result(human, &findings, &[]);
        Ok(())
    })
}

/// `timing scroll-add` 命令。
///
/// bmson 的滚动速度是内联事件（`y` + `rate`），无需 BMS 侧的定义表
/// 分配；`rate` 支持负值（反向滚动）。
///
/// # Errors
///
/// 位置越界或写回验证失败时返回 [`BmsEditError`]。
pub fn timing_scroll_add(
    file: &Path,
    at: &str,
    rate: f64,
    out: &mut Output,
) -> Result<(), BmsEditError> {
    let spot = spot_of(at)?;
    with_bmson(file, &[], |bmson, _| {
        let y = spot_to_y(bmson, spot.measure, spot.pos)?;
        bmson
            .scroll_events
            .push(bmsrs::bmson::def::ScrollEvent { y, rate });
        bmson.scroll_events.sort_by_key(|event| event.y);
        Ok(())
    })?;
    out.result(
        format!(
            "scroll {rate} inserted at #{:03} {}/{}",
            spot.measure, spot.pos.0, spot.pos.1
        ),
        &serde_json::json!({ "at": at, "rate": rate }),
        &[],
    );
    Ok(())
}

/// `timing scroll-remove` 命令：移除指定位置的全部滚动速度事件。
///
/// # Errors
///
/// 位置越界或该位置无事件时返回 [`BmsEditError`]。
pub fn timing_scroll_remove(file: &Path, at: &str, out: &mut Output) -> Result<(), BmsEditError> {
    let spot = spot_of(at)?;
    with_bmson(file, &[], |bmson, _| {
        let y = spot_to_y(bmson, spot.measure, spot.pos)?;
        let before = bmson.scroll_events.len();
        bmson.scroll_events.retain(|event| event.y != y);
        if bmson.scroll_events.len() == before {
            return Err(BmsEditError::Usage(format!("no scroll event at `{at}`")));
        }
        Ok(())
    })?;
    out.result(
        format!("scroll removed at `{at}`"),
        &serde_json::json!({ "removed": true }),
        &[],
    );
    Ok(())
}
