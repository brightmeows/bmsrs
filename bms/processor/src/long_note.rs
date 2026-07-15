//! BMS 谱面的长音配对。
//!
//! BMS 有三种 LN（长音）记法：
//!
//! - **LNTYPE 1 (RDM)**：通道 51–59 / 61–69 上的事件。索引为 `"00"` 的
//!   条目被跳过（该位置无长音）。同一 `(player, lane)` 上连续的非 `"00"`
//!   事件构成起止对。
//! - **LNTYPE 2 (MGQ)**：相同通道上的事件。当连续非 `"00"` 条目出现时
//!   长音持续，在下一个 `"00"` 处闭合。
//! - **LNOBJ**：指定的 WAV 索引标记长音终点。长音起点为常规音符事件；
//!   匹配的终点为同一 `(player, lane)` 上后续带有 `#LNOBJ` WAV 的音符。

use std::collections::{BTreeMap, HashMap, HashSet};

use bms_parser::{LongNoteEvent, NoteEvent};
use bms_tokenizer::{LnObjIndex, WavIndex};

use crate::position::MeasureTable;

/// 已配对的长音：起始脉冲、脉冲时长与 WAV 索引。
pub struct PairedLn {
    /// 长音起点的绝对脉冲。
    pub tick: u64,
    /// 脉冲时长（终点脉冲 - 起点脉冲）。
    pub duration: u64,
    /// 长音起点事件的 WAV 索引。
    pub wav_id: WavIndex,
    /// 玩家编号（1 或 2）。
    pub player: u8,
    /// 原始轨道（1-9）。
    pub lane: u8,
}

/// 若 WAV 索引表示空 / 无音符位置（BMS 记法中的 `"00"` 索引），返回
/// `true`。
#[inline]
fn is_empty_index(idx: WavIndex) -> bool {
    idx.as_str() == "00"
}

/// 从通道 51-69 配对 LNTYPE 1 (RDM) 长音事件。
///
/// 索引为 `"00"` 的条目被**过滤掉**（它们表示间隔）。剩余的非 `"00"`
/// 事件构成连续的起止对：第一个事件为长音起点，第二个为终点，第三个
/// 为下一个起点，依此类推。
pub fn pair_lntype1(events: &[LongNoteEvent], table: &MeasureTable) -> Vec<PairedLn> {
    // 按 (player, lane) 分组，过滤掉 "00" 条目。
    // 使用 BTreeMap：配对结果需按 (player, lane) 有序输出，依赖方假定其顺序确定。
    let mut groups: BTreeMap<(u8, u8), Vec<&LongNoteEvent>> = BTreeMap::new();
    for ev in events {
        if is_empty_index(ev.wav_id) {
            continue;
        }
        groups.entry((ev.player, ev.lane)).or_default().push(ev);
    }

    let mut result = Vec::new();

    for (&(player, lane), group) in &groups {
        let mut sorted = group.clone();
        sorted.sort_by_key(|ev| (ev.position.measure, ev.position.numer));

        // 以连续对消费事件：第一个 = 起点，第二个 = 终点。
        let mut iter = sorted.into_iter();
        while let (Some(start), Some(end)) = (iter.next(), iter.next()) {
            let start_tick = table.position_to_tick(start.position);
            let end_tick = table.position_to_tick(end.position);

            result.push(PairedLn {
                tick: start_tick,
                duration: end_tick.saturating_sub(start_tick),
                wav_id: start.wav_id,
                player,
                lane,
            });
        }
    }

    result
}

/// 从通道 51-69 配对 LNTYPE 2 (MGQ) 长音事件。
///
/// 在 MGQ 记法中，当连续非 `"00"` 条目出现时长音持续，在**下一个**
/// `"00"` 条目处闭合。每个 `"00"` 条目充当任何活跃长音的释放点。
///
/// 每个 `(player, lane)` 分组的算法：
/// 1. 按排序顺序扫描事件。
/// 2. 长音外：跳过 `"00"` 条目；第一个非 `"00"` = 长音起点。
/// 3. 长音内：第一个 `"00"` 条目 = 长音终点 → 生成配对。
/// 4. 若分组结束时仍有活跃长音，将其丢弃（无配对终点 → 无长音）。
pub fn pair_lntype2(events: &[LongNoteEvent], table: &MeasureTable) -> Vec<PairedLn> {
    // 按 (player, lane) 分组 —— 保留包括 "00" 在内的全部条目。
    // 使用 BTreeMap：配对结果需按 (player, lane) 有序输出，依赖方假定其顺序确定。
    let mut groups: BTreeMap<(u8, u8), Vec<&LongNoteEvent>> = BTreeMap::new();
    for ev in events {
        groups.entry((ev.player, ev.lane)).or_default().push(ev);
    }

    let mut result = Vec::new();

    for (&(player, lane), group) in &groups {
        let mut sorted = group.clone();
        sorted.sort_by_key(|ev| (ev.position.measure, ev.position.numer));

        let mut in_ln = false;
        let mut start_ev: Option<&LongNoteEvent> = None;

        for ev in sorted {
            if in_ln {
                // 长音内："00" 条目结束长音。
                if is_empty_index(ev.wav_id) {
                    if let Some(start) = start_ev.take() {
                        let start_tick = table.position_to_tick(start.position);
                        let end_tick = table.position_to_tick(ev.position);
                        result.push(PairedLn {
                            tick: start_tick,
                            duration: end_tick.saturating_sub(start_tick),
                            wav_id: start.wav_id,
                            player,
                            lane,
                        });
                    }
                    in_ln = false;
                }
                // 长音内的非 "00"：长音继续（无状态变化）。
            } else if !is_empty_index(ev.wav_id) {
                // 长音外：第一个非 "00" 开始一个长音。
                in_ln = true;
                start_ev = Some(ev);
            }
        }
        // 若分组结束时仍有活跃长音，静默丢弃
        // （未终止 —— 无可用配对终点）。
    }

    result
}

/// 从常规音符事件配对 LNOBJ 长音。
///
/// 扫描 `note_events` 中 `wav_id` 匹配 `#LNOBJ` 标记的音符。每个此类
/// 音符为一个长音终点；同一 `(player, lane)` 上前一个音符（非 LNOBJ
/// 标记）为长音起点。
///
/// 返回配对后的长音与已消耗音符事件的索引（起点与终点均包含），以便
/// 调用者将它们移除。
pub fn pair_lnobj(
    note_events: &[NoteEvent],
    ln_obj: LnObjIndex,
    table: &MeasureTable,
) -> (Vec<PairedLn>, HashSet<usize>) {
    // 按 (player, lane) 索引音符以查找前导音符。
    let mut last_by_lane: HashMap<(u8, u8), (usize, &NoteEvent)> = HashMap::new();
    let mut paired = Vec::new();
    let mut consumed = HashSet::new();

    for (i, ev) in note_events.iter().enumerate() {
        // 比较底层 BmsIndex 值（标准 BMS 中不区分大小写）。
        if *ev.wav_id == *ln_obj {
            // 此为长音终点。查找同一 (player, lane) 上的前导音符。
            if let Some(&(start_idx, start_ev)) = last_by_lane.get(&(ev.player, ev.lane)) {
                let start_tick = table.position_to_tick(start_ev.position);
                let end_tick = table.position_to_tick(ev.position);

                paired.push(PairedLn {
                    tick: start_tick,
                    duration: end_tick.saturating_sub(start_tick),
                    wav_id: start_ev.wav_id,
                    player: ev.player,
                    lane: ev.lane,
                });
                consumed.insert(start_idx);
                consumed.insert(i);
                last_by_lane.remove(&(ev.player, ev.lane));
            }
        } else {
            last_by_lane.insert((ev.player, ev.lane), (i, ev));
        }
    }

    (paired, consumed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_parser::Position;
    use bms_tokenizer::LnObjIndex;

    fn make_table() -> MeasureTable {
        MeasureTable::new(4, &[], 240)
    }

    fn ln_event(player: u8, lane: u8, measure: u16, numer: u32, wav: &str) -> LongNoteEvent {
        LongNoteEvent {
            position: Position::new(measure, numer, 8),
            player,
            lane,
            wav_id: wav.parse().unwrap(),
        }
    }

    fn note_event(player: u8, lane: u8, measure: u16, numer: u32, wav: &str) -> NoteEvent {
        NoteEvent {
            position: Position::new(measure, numer, 8),
            player,
            lane,
            key_type: bms_parser::KeyType::Visible,
            wav_id: wav.parse().unwrap(),
        }
    }

    #[test]
    fn lntype1_pairs_consecutive_events() {
        let events = vec![ln_event(1, 1, 0, 0, "AA"), ln_event(1, 1, 1, 0, "BB")];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 1);
        let ln = &result[0];
        assert_eq!(ln.tick, 0);
        assert_eq!(ln.duration, 960); // 分辨率 240 下的 1 小节
    }

    #[test]
    fn lntype1_multiple_pairs_same_lane() {
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 4, "BB"),
            ln_event(1, 1, 1, 0, "CC"),
            ln_event(1, 1, 1, 4, "DD"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].tick, 0);
        assert_eq!(result[1].tick, 960);
    }

    #[test]
    fn lntype1_odd_event_unpaired() {
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 1, 0, "BB"),
            ln_event(1, 1, 2, 0, "CC"), // 未配对
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn lntype1_different_lanes_independent() {
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 2, 0, 0, "BB"),
            ln_event(1, 1, 1, 0, "CC"),
            ln_event(1, 2, 1, 0, "DD"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].lane, 1);
        assert_eq!(result[1].lane, 2);
    }

    #[test]
    fn lnobj_pairs_start_and_end() {
        let ln_obj: LnObjIndex = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "AA"),
            note_event(1, 1, 1, 0, "FF"), // LNOBJ 标记
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert_eq!(paired.len(), 1);
        assert_eq!(paired[0].tick, 0);
        assert_eq!(paired[0].duration, 960);
        assert_eq!(paired[0].wav_id, "AA".parse().unwrap());
        assert_eq!(consumed.len(), 2);
    }

    #[test]
    fn lnobj_consumed_notes_excluded_from_regular() {
        let ln_obj: LnObjIndex = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "AA"),
            note_event(1, 1, 1, 0, "FF"),
            note_event(1, 2, 0, 0, "BB"), // 常规音符，未被消耗
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert_eq!(paired.len(), 1);
        assert!(consumed.contains(&0));
        assert!(consumed.contains(&1));
        assert!(!consumed.contains(&2));
    }

    #[test]
    fn lnobj_unmatched_marker_produces_no_pair() {
        let ln_obj: LnObjIndex = "FF".parse().unwrap();
        let notes = vec![
            note_event(1, 1, 0, 0, "FF"), // 无前导音符的标记
        ];
        let table = make_table();
        let (paired, consumed) = pair_lnobj(&notes, ln_obj, &table);

        assert!(paired.is_empty());
        assert!(consumed.is_empty());
    }

    // LNTYPE 1："00" 条目必须被过滤掉

    #[test]
    fn lntype1_skips_zero_entries() {
        // 通道数据：AA（有声）00（静音）BB（有声）→ 单个配对 (AA, BB)。
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "BB"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
    }

    #[test]
    fn lntype1_skips_multiple_zero_entries() {
        // AA 00 00 BB 00 CC DD → 配对：(AA, BB)、(CC, DD)
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "00"),
            ln_event(1, 1, 0, 3, "BB"),
            ln_event(1, 1, 0, 4, "00"),
            ln_event(1, 1, 0, 5, "CC"),
            ln_event(1, 1, 0, 6, "DD"),
        ];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
        assert_eq!(result[1].wav_id, "CC".parse().unwrap());
    }

    #[test]
    fn lntype1_all_zero_entries_produces_nothing() {
        let events = vec![ln_event(1, 1, 0, 0, "00"), ln_event(1, 1, 0, 1, "00")];
        let table = make_table();
        let result = pair_lntype1(&events, &table);

        assert!(result.is_empty());
    }

    // LNTYPE 2 (MGQ)

    #[test]
    fn lntype2_pairs_run_with_trailing_zero() {
        // AA BB CC 00 → 从 AA 到 00 的长音。
        // denom=8（通过 ln_event 辅助函数），因此每个 numer 步长 = 960/8 = 120 脉冲。
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "BB"),
            ln_event(1, 1, 0, 2, "CC"),
            ln_event(1, 1, 0, 3, "00"),
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tick, 0);
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
        // 00 在 numer=3（共 8）→ 位置 = 3/8 小节 → 脉冲 = 960 * 3/8 = 360
        assert_eq!(result[0].duration, 360);
    }

    #[test]
    fn lntype2_multiple_runs() {
        // AA 00 BB CC 00 DD 00 → LN1：(AA, 第一个 00)，LN2：(BB, 第二个 00)，
        // LN3：(DD, 第三个 00)。denom=8 → 每个 numer 步长 = 120 脉冲。
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"), // 脉冲 0
            ln_event(1, 1, 0, 1, "00"), // 脉冲 120
            ln_event(1, 1, 0, 2, "BB"), // 脉冲 240
            ln_event(1, 1, 0, 3, "CC"), // 脉冲 360 —— 继续 LN2
            ln_event(1, 1, 1, 0, "00"), // 脉冲 960
            ln_event(1, 1, 1, 1, "DD"), // 脉冲 1080
            ln_event(1, 1, 1, 2, "00"), // 脉冲 1200
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.len(), 3);
        // LN1：AA(0) → 00(120)
        assert_eq!(result[0].wav_id, "AA".parse().unwrap());
        assert_eq!(result[0].tick, 0);
        assert_eq!(result[0].duration, 120);
        // LN2：BB(240) → 00(960) —— CC(360) 继续该长音
        assert_eq!(result[1].wav_id, "BB".parse().unwrap());
        assert_eq!(result[1].tick, 240);
        assert_eq!(result[1].duration, 720);
        // LN3：DD(1080) → 00(1200)
        assert_eq!(result[2].wav_id, "DD".parse().unwrap());
        assert_eq!(result[2].tick, 1080);
        assert_eq!(result[2].duration, 120);
    }

    #[test]
    fn lntype2_unterminated_run_dropped() {
        // AA BB（无尾部 00）→ 无配对长音。
        let events = vec![ln_event(1, 1, 0, 0, "AA"), ln_event(1, 1, 0, 1, "BB")];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert!(result.is_empty());
    }

    #[test]
    fn lntype2_immediate_release() {
        // AA 00 00 BB 00 → LN1：(AA, 第一个 00)，间隔，LN2：(BB, 第二个 00)
        let events = vec![
            ln_event(1, 1, 0, 0, "AA"),
            ln_event(1, 1, 0, 1, "00"),
            ln_event(1, 1, 0, 2, "00"),
            ln_event(1, 1, 0, 3, "BB"),
            ln_event(1, 1, 1, 0, "00"),
        ];
        let table = make_table();
        let result = pair_lntype2(&events, &table);

        assert_eq!(result.len(), 2);
    }
}
