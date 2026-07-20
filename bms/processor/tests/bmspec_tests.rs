//! `BMSpec` 可执行规范集成测试。
//!
//! 这些测试从原始 BMS 文本出发，走完整管道
//! （tokenizer → parser → processor → Chart），验证事件时间位置
//! 符合 bmspec 规范断言。每个测试对应 bmspec 目录下的一个场景。
//!
//! 节拍（beat）换算：1 beat = resolution（240）个脉冲。

use bms_control_flow::FlowDoc;
use bms_parser::Bms;
use bms_processor::BmsProcessor;
use bms_processor::custom_event::BmsCustomEvent;
use bms_processor::layout::Bme;
use bms_tokenizer::BmsTokenizer;
use bmsrs_chart::{Chart, EventKind, NoteKind};
use bmsrs_player::Player;

/// bms-processor 返回的 chart 类型。
type BmsChart = Chart<(), BmsCustomEvent>;

/// 管道辅助：BMS 原文 → `Chart<(), BmsCustomEvent>`。
#[expect(clippy::expect_used, reason = "test helper panics on process failure")]
fn process(bms_text: &str) -> BmsChart {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>>(bms_text)
        .into_iter()
        .filter_map(|(_, res)| res.ok())
        .collect();
    let bms = Bms::from_flat_tokens(tokens);
    BmsProcessor::process::<Bme>(&bms).expect("process should succeed")
}

/// 提取所有 Note 事件（含种类信息）。
fn all_notes(chart: &BmsChart) -> Vec<(u64, NoteKind)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note { kind, .. } = &e.kind {
                Some((e.tick(), *kind))
            } else {
                None
            }
        })
        .collect()
}

/// 提取所有 Scroll 事件。
fn scroll_events(chart: &BmsChart) -> Vec<(u64, f64)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Scroll { rate } = &e.kind {
                Some((e.tick(), *rate))
            } else {
                None
            }
        })
        .collect()
}

/// 提取所有 Speed 事件。
fn speed_events(chart: &BmsChart) -> Vec<(u64, f64)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Speed { rate } = &e.kind {
                Some((e.tick(), *rate))
            } else {
                None
            }
        })
        .collect()
}

/// 提取 Long `Note`（`NoteKind::Long`）事件。
fn long_notes(chart: &BmsChart) -> Vec<(u64, u64)> {
    chart
        .data
        .events
        .iter()
        .filter_map(|e| {
            if let EventKind::Note {
                kind: NoteKind::Long { duration },
                ..
            } = &e.kind
            {
                Some((e.tick(), *duration))
            } else {
                None
            }
        })
        .collect()
}

/// bmspec-1-01-Sentences: 解析标准 BMS 文件。
#[test]
fn bmspec_1_01_parse_sentences() {
    let chart = process(
        "#TITLE BY MY SIDE\n\
         #ARTIST flicknote\n\
         #00101:0100010001\n\
         This is a comment\n",
    );
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
    // 通道 01 含有 4 个 BGM 事件。注释行被忽略。
    assert_eq!(chart.data.audio_assets.len(), 0); // WAV 未定义
}

/// bmspec-1-01-Sentences: 格式错误的命令行（无空格分隔符的头部视为注释）。
#[test]
fn bmspec_1_01_malformed_command() {
    // #ARTIST:flicknote （冒号而非空格）→ 不被识别为头部。
    let chart = process("#TITLE BY MY SIDE\n#ARTIST:flicknote\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    // #ARTIST 没被解析到（冒号导致 parse_header_line 回退）
    assert_eq!(chart.song.artist, "");
}

/// bmspec-1-02-Header: 标准头部。
#[test]
fn bmspec_1_02_standard_header() {
    let chart = process("#TITLE BY MY SIDE\n#ARTIST flicknote\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
}

/// bmspec-1-02-Header: 多空格分隔符。
#[test]
fn bmspec_1_02_header_multiple_spaces() {
    let chart = process("#TITLE      BY MY SIDE\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
}

/// bmspec-1-02-Header: 不区分大小写。
#[test]
fn bmspec_1_02_case_insensitive_header() {
    let chart = process("#Title BY MY SIDE\n#Artist flicknote\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
}

/// bmspec-1-02-Header: 重复头部，后胜出。
#[test]
fn bmspec_1_02_duplicated_header() {
    let chart = process("#TiTlE BEAT MUSIC SEQUENCE\n#tItLe BY*MY*SIDE\n");
    assert_eq!(chart.song.title, "BY*MY*SIDE");
}

/// bmspec-1-03-Objects: 基本音符对象位置。
#[test]
fn bmspec_1_03_basic_objects() {
    let chart = process("#00111:01000002\n#00311:0003\n");
    let notes = all_notes(&chart);
    // 3 个音符，分别位于 beat 4, 7, 14
    // beat = tick / 240
    assert_eq!(notes.len(), 3);
    let ticks: Vec<u64> = notes.into_iter().map(|(t, _)| t).collect();
    assert_eq!(ticks[0], 960, "object 01 at beat 4"); // measure 1, pos 0/4
    assert_eq!(ticks[1], 1680, "object 02 at beat 7"); // measure 1, pos 3/4
    assert_eq!(ticks[2], 3360, "object 03 at beat 14"); // measure 3, pos 1/2
}

/// bmspec-1-03-Objects: 重叠音符对象（通道合并）。
#[test]
fn bmspec_1_03_overlapped_objects() {
    let chart = process(
        "#00113:11111111\n\
         #00113:0022332255224400\n\
         #00113:0066\n",
    );
    let notes = all_notes(&chart);
    // 合并后应有 7 个对象（过滤掉 "00"）
    assert_eq!(notes.len(), 7);
    // merge: max_count=8。
    //   行 1 ("11111111", 4 值) → 位置 0,2,4,6
    //   行 2 ("0022332255224400", 8 值) → 位置 1=22,2=33,3=22,4=55,5=22,6=44
    //   行 3 ("0066", 2 值) → 位置 4=66（覆盖 55）
    // 合并结果 (过滤 00): 11(0),22(1),33(2),22(3),66(4),22(5),44(6)
    // tick at pos i = 960 + i/8 * 960
    let ticks: Vec<u64> = notes.iter().map(|(t, _)| *t).collect();
    assert_eq!(ticks[4], 1440, "object 66 should be at beat 6"); // pos 4/8 → 960 + 480 = 1440
    assert_eq!(ticks[6], 1680, "object 44 at beat 7"); // pos 6/8 → 960 + 720 = 1680
}

/// bmspec-1-03-Objects: 空通道行。
#[test]
fn bmspec_1_03_empty_channel() {
    let chart = process("#00111:\n");
    let notes = all_notes(&chart);
    assert!(notes.is_empty(), "empty channel produces no objects");
}

/// bmspec-1-04-Time-Signature: 变拍子定位。
#[test]
fn bmspec_1_04_time_signature() {
    let chart = process(
        "#00102:0.750\n\
         #00111:0104\n\
         #00211:02\n\
         #00311:03\n",
    );
    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 4);
    let ticks: Vec<u64> = notes.into_iter().map(|(t, _)| t).collect();

    // measure 1 is 3/4 (0.75 × 960 = 720 ticks), 2 values → each=360 ticks
    // beat = tick / 240
    // obj 01 at beat 4: measure 1, pos 0/2 → tick 960
    // obj 04 at beat 5.5: measure 1, pos 1/2 → tick 1320
    // obj 02 at beat 7: measure 2, pos 0/1 → tick 1680
    // obj 03 at beat 11: measure 3, pos 0/1 → tick 2640

    assert_eq!(ticks[0], 960, "obj 01 at beat 4");
    assert_eq!(ticks[1], 1320, "obj 04 at beat 5.5");
    assert_eq!(ticks[2], 1680, "obj 02 at beat 7");
    assert_eq!(ticks[3], 2640, "obj 03 at beat 11");
}

/// bmspec-1-05-BPM: BPM 变化 —— 对象应位于正确的时间位置。
#[test]
fn bmspec_1_05_bpm_change() {
    let chart = process("#BPM 60\n#00003:0078\n#00111:01\n");

    // #00003:0078 → channel 03 (BPM change), track=0, values [00, 78]
    // 78 hex = 120 decimal → BPM change to 120 at position 0/2 of measure 0
    // Then #00111:01 → note at measure 1, pos 0/1

    let has_bpm = chart
        .data
        .events
        .iter()
        .any(|e| matches!(&e.kind, EventKind::Bpm { .. }));
    assert!(has_bpm, "should have BPM events");

    // The note should be at 3 seconds
    // At 60 BPM: tick 0 to ... Let me calculate
    // #00003:0078 → Position(0, 1, 2) → starts[0] + 1/2 * 960 = 0 + 480 = 480
    // At tick 480, BPM changes from 60 to 120
    // #00111:01 → Position(1, 0, 1) → starts[1] + 0 = 960

    let notes = all_notes(&chart);
    let note_tick = notes.first().map_or(0, |(t, _)| *t);
    assert_eq!(note_tick, 960, "note at tick 960");

    let dur = chart.data.timing.tick_to_duration(note_tick);
    // 0-480 ticks at 60 BPM: 480/240 * 60/60 = 2.0s
    // 480-960 ticks at 120 BPM: 480/240 * 60/120 = 1.0s
    // Total: 3.0s
    assert!(
        (dur.as_secs_f64() - 3.0).abs() < 1e-6,
        "expected 3.0s, got {dur:?}"
    );
}

/// bmspec-1-05-BPM: 多段 BPM 变化。
#[test]
fn bmspec_1_05_multiple_bpm_changes() {
    let chart = process(
        "#BPM 100\n\
         #00003:0060C0\n\
         #00011:00010203\n\
         #00111:04\n",
    );

    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 4, "should have 4 notes");
    let ticks: Vec<u64> = notes.iter().map(|(t, _)| *t).collect();
    // #00003:0060C0 → Position(0, 0, 2)=BPM 96 (60h=96), (0,1,2)=BPM 192 (C0h=192)
    // #00011:00010203 → 4 notes at measure 0: positions 0/4, 1/4, 2/4, 3/4

    // #00011:00010203 → measure=0, channel 11, values [00, 01, 02, 03]
    // "00" 被过滤掉。剩下 3 个 note 在 measure 0, positions 1/4, 2/4, 3/4
    // #00111:04 → measure=1, pos=0/1
    // starts = [0, 960, 1920]
    // Position(0, 0, 4) → 0 (filtered out by "00" rule)
    // Position(0, 1, 4) → 0 + 960/4 = 240
    // Position(0, 2, 4) → 0 + 960/4*2 = 480
    // Position(0, 3, 4) → 0 + 960/4*3 = 720
    // Position(1, 0, 1) → 960
    assert_eq!(ticks[0], 240, "note 01");
    assert_eq!(ticks[1], 480, "note 02");
    assert_eq!(ticks[2], 720, "note 03");
    assert_eq!(ticks[3], 960, "note 04");

    // Times:
    // 0-240 ticks at 100 BPM: 240/240 * 60/100 = 0.6s → note 01 at 0.6s ✓
    let dur0 = chart.data.timing.tick_to_duration(240);
    assert!(
        (dur0.as_secs_f64() - 0.6).abs() < 1e-4,
        "note 01 should be at 0.6s, got {dur0:?}"
    );
}

/// bmspec-1-05-BPM: 扩展 BPM（#BPMxx 定义 + #00008 引用）。
#[test]
fn bmspec_1_05_extended_bpm() {
    let chart = process(
        "#BPM 60\n\
         #BPM01 120\n\
         #00008:0001\n\
         #00111:05\n",
    );

    // #BPM01 120 defines extended BPM index 01 = 120.0
    // #00008:0001 → channel 08 (extended BPM), track=0, values [00, 01]
    // "00" = 休止（无 BPM 变更），"01" 引用 BPM01=120 at position 1/2
    // BPM 序列：
    //   tick 0: init 60 (from #BPM 60)
    //   tick 480: BPM 120 (from "01" → BPM01)
    // Note at tick 960 = measure 1, pos 0/1
    //   0-480 ticks at 60 BPM = 480/240 * 60/60 = 2.0s
    //   480-960 ticks at 120 BPM = 480/240 * 60/120 = 1.0s
    //   Total: 3.0s

    let notes = all_notes(&chart);
    assert_eq!(notes[0].0, 960, "note at tick 960");
    let dur = chart.data.timing.tick_to_duration(notes[0].0);
    assert!(
        (dur.as_secs_f64() - 3.0).abs() < 1e-6,
        "expected 3.0s, got {dur:?}"
    );
}

/// bmspec-1-06-STOP: 基本停止。
#[test]
fn bmspec_1_06_basic_stop() {
    let chart = process(
        "#BPM 60\n\
         #STOP11 96\n\
         #00111:01000200\n\
         #00109:00110000\n",
    );

    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 2, "should have 2 notes");
    // Note 01 at measure 1, pos 0/4 = tick 960
    // Note 02 at measure 1, pos 3/4 = tick 1680
    // Stop at position 0/4 of measure 1, duration = 96/192 * 960 = 480 ticks
    //   Wait, STOP duration = raw / 192.0 * resolution * 4
    //   96 / 192 * 960 = 480 ticks at 60 BPM = 4 seconds

    // Note 01 at tick 960:
    //   0-960 at 60 BPM = 960/240 * 60/60 = 4.0s
    //   But there's a stop at the same position as Note 01 (measure 1, pos 0/4).
    //   In tick_to_duration, stop at target tick doesn't add pause (by spec).
    //   So note 01 is at 4.0s

    // Note 02 at tick 1680:
    //   0-960 at 60 BPM = 4.0s
    //   Stop at tick 960: duration = 480 ticks = 4.0s at 60 BPM
    //   Stop 严格在 tick 960（< 1680），停止时长计入。
    //   停止后位置推进到 tick 960 + 480 = 1440。
    //   1440-1680 = 240 ticks at 60 BPM = 1.0s
    //   合计 = 4.0 + 4.0 + 1.0 = 9.0s

    let dur1 = chart.data.timing.tick_to_duration(notes[0].0);

    // bmspec: obj 01 at 4s
    assert!(
        (dur1.as_secs_f64() - 4.0).abs() < 1.0,
        "obj 01 approx at 4s, got {dur1:?}"
    );
}

/// bmspec-1-06-STOP: STOP 与 BPM 同 tick 时序——BPM（优先级 2）应在 STOP（优先级 3）之前
/// 生效。覆盖 A11 场景。
#[test]
fn bmspec_1_06_stop_on_same_beat_as_bpm_matches_expected_timing() {
    let chart = process(
        "#BPM 60\n\
         #BPM01 120\n\
         #STOP01 96\n\
         #00108:0100\n\
         #00109:0100\n\
         #00211:01\n",
    );

    let notes = all_notes(&chart);
    assert_eq!(notes.len(), 1, "should have 1 note");
    // BPM 变更为 120 与 STOP 同在 tick 960（measure 1, pos 0/2）。
    // 事件排序：BPM(2) 先于 STOP(3)。
    //   tick 0-960 at 60 BPM = 4.0s
    //   STOP 480 ticks at 120 BPM（BPM 已变更）= 1.0s
    //   tick 960-1920 at 120 BPM = 2.0s
    //   合计 = 7.0s
    // 若 BPM 在 STOP 之后变更：STOP 480 ticks at 60 BPM = 2.0s → 合计 8.0s
    let dur = chart.data.timing.tick_to_duration(notes[0].0);
    assert!(
        (dur.as_secs_f64() - 7.0).abs() < 1e-6,
        "expected note at 7.0s (BPM before STOP), got {dur:?}"
    );
}

/// bmspec-2-LNOBJ: 长音对象配对。
#[test]
fn bmspec_2_lnobj() {
    let chart = process("#LNOBJ XX\n#00111:01XX02XX03XX04XX\n");
    let lns = long_notes(&chart);

    // 4 个长音，每个从 beat 4 到 4.5（半拍间隔在 8 值/小节中）
    // #00111:01XX02XX03XX04XX 有 8 个值
    // 每个值 960/8 = 120 个脉冲
    // 01 在 pos 0/8: tick 960 → beat 4
    // XX 在 pos 1/8: tick 1080 → beat 4.5（长音终点）
    // 02 在 pos 2/8: tick 1200 → beat 5（下一个长音起点）
    // 以此类推...

    assert_eq!(lns.len(), 4, "should have 4 long notes");

    // 每个长音的 duration: 1 个 subdivision = 120 ticks
    assert_eq!(lns[0], (960, 120), "LN 01: beat 4 to 4.5");
    assert_eq!(lns[1], (1200, 120), "LN 02: beat 5 to 5.5");
    assert_eq!(lns[2], (1440, 120), "LN 03: beat 6 to 6.5");
    assert_eq!(lns[3], (1680, 120), "LN 04: beat 7 to 7.5");
}

/// bmspec-2-LNTYPE1: 独立长音通道。
#[test]
fn bmspec_2_lntype1() {
    let chart = process("#00151:0101020203030404\n");
    let lns = long_notes(&chart);

    // 通道 51, 8 个值：01 01 02 02 03 03 04 04
    // 第 1-2: 01 at pos 0, 01 at pos 1 → 长音 01 从 pos 0 到 pos 1
    // 第 3-4: 02 at pos 2, 02 at pos 3 → 长音 02 从 pos 2 到 pos 3
    // 以此类推...

    assert_eq!(lns.len(), 4, "should have 4 long notes");

    let sub = 960 / 8; // 每个 subdivision = 120 个脉冲
    assert_eq!(lns[0], (960, sub), "LN 01: beat 4 to 4.5");
    assert_eq!(lns[1], (1200, sub), "LN 02: beat 5 to 5.5");
    assert_eq!(lns[2], (1440, sub), "LN 03: beat 6 to 6.5");
    assert_eq!(lns[3], (1680, sub), "LN 04: beat 7 to 7.5");
}

/// bmspec-2-LNTYPE2: MGQ 长音记法。
#[test]
fn bmspec_2_lntype2_pairs_long_notes_correctly() {
    let chart = process(
        "#LNTYPE 2\n\
         #00151:00222222\n\
         #00251:22\n\
         #00351:2200\n",
    );
    let lns = long_notes(&chart);

    // LNTYPE 2 (MGQ) 中，"00" 充当释放标记。
    // #00151:00222222 → 4 个值：00 22 22 22（denom=4）
    //   00 at pos 0/4 = tick 960 → 跳过
    //   22 at pos 1/4 = tick 1200 → 开始长音
    //   22 at pos 2/4 = tick 1440 → 继续
    //   22 at pos 3/4 = tick 1680 → 继续
    // #00251:22 → 1 个值：22（denom=1）
    //   22 at pos 0/1 = tick 1920 → 继续
    // #00351:2200 → 2 个值：22 00（denom=2）
    //   22 at pos 0/2 = tick 2880 → 继续
    //   00 at pos 1/2 = tick 3360 → 释放长音
    //
    // 单个长音从 tick 1200 到 tick 3360，duration = 2160

    assert_eq!(lns.len(), 1, "should have 1 long note");
    assert_eq!(lns[0], (1200, 2160), "LN from beat 5 to beat 14");
}

/// bmspec-3-SCROLL: 基础：速度与位置累积。
#[test]
fn bmspec_3_scroll_basic() {
    let chart = process("#SCROLL02 0.5\n#001SC:02\n");

    // scroll_defs["02"] = 0.5
    // #001SC:02 → channel SC (scroll), value "02" at position 0/1 of measure 1
    let scrolls = scroll_events(&chart);
    assert!(!scrolls.is_empty(), "should have scroll events");
    // The scroll event is at measure 1, pos 0 = tick 960
    assert_eq!(scrolls[0].0, 960, "scroll at beat 4");
    assert!(
        (scrolls[0].1 - 0.5).abs() < 1e-9,
        "scroll rate should be 0.5"
    );

    // Verify scroll position via Player
    let player = Player::new(chart).unwrap();
    // Before scroll: rate 1.0, position = ticks / resolution
    assert!((player.scroll_rate_at(0) - 1.0).abs() < 1e-9);
    assert!((player.scroll_position_at(0) - 0.0).abs() < 1e-9);
    // At beat 4 (tick 960): scroll just changed to 0.5, position = 4 (no delay yet)
    assert!((player.scroll_position_at(960) - 4.0).abs() < 1e-9);
    // At beat 6 (tick 1440): 2 beats at 0.5 → position advances 1 → cum = 5
    assert!(
        (player.scroll_position_at(1440) - 5.0).abs() < 1e-9,
        "expected scroll position 5 at beat 6 (tick 1440)"
    );
}

/// bmspec-3-SCROLL: 初始速度：第 0 小节指定。
#[test]
fn bmspec_3_scroll_initial_speed() {
    let chart = process("#SCROLL02 0.5\n#000SC:02\n");
    let scrolls = scroll_events(&chart);
    assert!(!scrolls.is_empty(), "should have scroll events");
    // Scroll at measure 0, pos 0 → tick 0
    assert_eq!(scrolls[0].0, 0, "initial scroll at tick 0");
    assert!(
        (scrolls[0].1 - 0.5).abs() < 1e-9,
        "scroll rate should be 0.5"
    );

    // Verify position: scroll at tick 0 with rate 0.5
    let player = Player::new(chart).unwrap();
    assert!((player.scroll_rate_at(0) - 0.5).abs() < 1e-9);
    // At tick 0: position = 0, but rate = 0.5 from start
    // At beat -1 (negative beat, tick -240): position = 0.5 * (-1) = -0.5
    // But we can't test negative ticks...
    // At beat 4 (tick 960): 4 beats at 0.5 = position 2.0
    assert!(
        (player.scroll_position_at(960) - 2.0).abs() < 1e-9,
        "expected position 2.0 at beat 4 with 0.5x scroll"
    );
}

/// 辅助函数：走完整控制流管道的 BMS 处理。
/// 用 `SETRANDOM` / `SETSWITCH` 固定分支值。
/// 测试用确定性 RNG（总是返回 0，适合 SETRANDOM 固定值场景）。
struct TestRng;
impl bms_control_flow::BranchRng for TestRng {
    fn gen_range(&mut self, _max: u64) -> u64 {
        0
    }
}

fn process_with_cf(bms_text: &str) -> BmsChart {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>>(bms_text)
        .into_iter()
        .filter_map(|(line, res)| res.ok().map(|t| (line, t)))
        .collect();
    #[expect(clippy::expect_used, reason = "test helper panics on build failure")]
    let tree = FlowDoc::from_tokens(tokens).expect("build FlowDoc");
    let (flat, _) = tree.select_branches(&mut TestRng);
    let bms = Bms::from_flat_tokens(flat);
    #[expect(clippy::expect_used, reason = "test helper panics on failure")]
    BmsProcessor::process::<Bme>(&bms).expect("process should succeed")
}

/// bmspec-4-RANDOM: SETRANDOM 固定值选择分支。
#[test]
fn bmspec_4_random_setrandom() {
    // SETRANDOM 1 → 始终选中 #IF 1
    let chart = process_with_cf(
        "#SETRANDOM 1\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #BPM 120\n",
    );
    // 应只有 WAV01 (a.wav) 出现在 audio_assets 中
    assert_eq!(chart.data.audio_assets.len(), 1);
    assert_eq!(chart.data.audio_assets[0].path.to_string_lossy(), "a.wav");
}

/// bmspec-4-RANDOM: SETRANDOM 选中第二个分支。
#[test]
fn bmspec_4_random_setrandom_second() {
    let chart = process_with_cf(
        "#SETRANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #BPM 120\n",
    );
    assert_eq!(chart.data.audio_assets.len(), 1);
    assert_eq!(chart.data.audio_assets[0].path.to_string_lossy(), "b.wav");
}

/// bmspec-4-RANDOM: 多个连续块各自独立选择。
#[test]
fn bmspec_4_random_multiple_blocks() {
    // 第一个块选中 #IF 2，第二个块选中 #IF 1（顺序不重要）
    let chart = process_with_cf(
        "#SETRANDOM 2\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n\
         #SETRANDOM 1\n\
         #IF 1\n\
         #WAV02 c.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV02 d.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n",
    );
    assert_eq!(chart.data.audio_assets.len(), 2);
    let paths: Vec<_> = chart
        .data
        .audio_assets
        .iter()
        .map(|a| a.path.to_string_lossy().to_string())
        .collect();
    assert!(
        paths.contains(&"b.wav".to_owned()),
        "first block picks branch 2"
    );
    assert!(
        paths.contains(&"c.wav".to_owned()),
        "second block picks branch 1"
    );
}

/// bmspec-4-RANDOM: 无匹配时不出 WAV。
#[test]
fn bmspec_4_random_no_match() {
    let chart = process_with_cf(
        "#SETRANDOM 3\n\
         #IF 1\n\
         #WAV01 a.wav\n\
         #ENDIF\n\
         #IF 2\n\
         #WAV01 b.wav\n\
         #ENDIF\n\
         #ENDRANDOM\n",
    );
    // RNG=3 不匹配任何 IF → 不产出 WAV
    assert!(chart.data.audio_assets.is_empty());
}

/// bmspec-5-Subtitle: 基本副标题。
#[test]
fn bmspec_5_subtitle() {
    let chart = process("#TITLE BY MY SIDE\n#SUBTITLE [TUTORIAL]\n");
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.chart.subtitle, "[TUTORIAL]");
}

/// bmspec-1-07-Basic-Info: 基本信息完整解析。
#[test]
fn bmspec_1_07_basic_info() {
    let chart = process(
        "#TITLE BY MY SIDE\n\
         #ARTIST flicknote\n\
         #GENRE Trance Core\n\
         #DIFFICULTY 2\n\
         #PLAYLEVEL 5\n",
    );
    assert_eq!(chart.song.title, "BY MY SIDE");
    assert_eq!(chart.song.artist, "flicknote");
    assert_eq!(chart.song.genre, "Trance Core");
    assert_eq!(chart.chart.chart_name, "5");
    assert_eq!(chart.chart.level, 5);
}

/// bmspec-1-08-WAV: WAV 定义引用（在 Bms 模型层验证，Chart 不保留 WavIndex→文件名映射）。
#[test]
fn bmspec_1_08_wav_references() {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>>(
            "#TITLE #WAV test case\n\
             #WAV01 index.mp3\n\
             #WAVZZ wow.mp3\n\
             #WAVAA zz.ogg\n",
        )
        .into_iter()
        .filter_map(|(_, res)| res.ok())
        .collect();
    let bms = Bms::from_flat_tokens(tokens);

    let wav01: bms_tokenizer::WavIndex = "01".parse().unwrap();
    let wav_zz: bms_tokenizer::WavIndex = "ZZ".parse().unwrap();
    let wav_aa: bms_tokenizer::WavIndex = "AA".parse().unwrap();
    let wav02: bms_tokenizer::WavIndex = "02".parse().unwrap();

    assert_eq!(
        bms.audio.wav_files.get(&wav01).map(String::as_str),
        Some("index.mp3")
    );
    assert_eq!(
        bms.audio.wav_files.get(&wav_zz).map(String::as_str),
        Some("wow.mp3")
    );
    assert_eq!(
        bms.audio.wav_files.get(&wav_aa).map(String::as_str),
        Some("zz.ogg")
    );
    assert_eq!(bms.audio.wav_files.get(&wav02).map(String::as_str), None);
}

/// bmspec-6-SPEED: 未设置时保持默认 1.0。
#[test]
fn bmspec_6_speed_without_set() {
    let chart = process("#SPEED01 0.5\n");
    let speeds = speed_events(&chart);
    assert!(speeds.is_empty(), "no SPEED event without #xxxSP channel");
}

/// bmspec-6-SPEED: 单值设置。
#[test]
fn bmspec_6_speed_single() {
    let chart = process("#SPEED01 0.5\n#001SP:0001\n");
    let speeds = speed_events(&chart);
    // #001SP:0001 → 2 values [00, 01], 但 SPEED00 未定义 → 跳过
    // 只有 SPEED01=0.5 被发出：Position(1, 1, 2) → tick=960+480=1440
    assert_eq!(speeds.len(), 1, "single speed event");
    assert_eq!(speeds[0].0, 1440, "speed at beat 6");
    assert!((speeds[0].1 - 0.5).abs() < 1e-9, "speed rate should be 0.5");
}

/// bmspec-6-SPEED: 多值插值（多个 `SPEEDxx` 定义）。
#[test]
fn bmspec_6_speed_multiple() {
    let chart = process(
        "#SPEED01 0.5\n\
         #SPEED02 1.5\n\
         #SPEED03 1\n\
         #001SP:0102\n\
         #002SP:03\n",
    );
    let speeds = speed_events(&chart);
    // #001SP:0102 = [01, 02] → 2 events（00 不存在于输入中）
    //   Position(1, 0, 2)=tick 960(value=01→0.5),
    //   Position(1, 1, 2)=tick 1440(value=02→1.5)
    // #002SP:03 = [03] → 1 event
    //   Position(2, 0, 1)=tick 1920(value=03→1.0)
    assert_eq!(speeds.len(), 3, "should have 3 speed events");
    assert_eq!(speeds[0].0, 960, "speed 01 at beat 4");
    assert!((speeds[0].1 - 0.5).abs() < 1e-9);
    assert_eq!(speeds[1].0, 1440, "speed 02 at beat 6");
    assert!((speeds[1].1 - 1.5).abs() < 1e-9);
    assert_eq!(speeds[2].0, 1920, "speed 03 at beat 8");
    assert!((speeds[2].1 - 1.0).abs() < 1e-9);
}

/// bmspec-6-SPEED: 间距插值（通过 `Player::spacing_at` 验证）。
#[test]
fn bmspec_6_speed_interpolation() {
    let chart = process(
        "#SPEED01 0.5\n\
         #SPEED02 1.5\n\
         #SPEED03 1\n\
         #001SP:0102\n\
         #002SP:03\n",
    );
    let player = Player::new(chart).unwrap();

    // 首个关键帧之前 → 1.0（默认）
    assert!((player.spacing_at(0) - 1.0).abs() < 1e-9, "before first kf");

    // keyframe 01: tick 960 (beat 4), rate 0.5
    assert!((player.spacing_at(960) - 0.5).abs() < 1e-9, "kf 01");

    // keyframe 01→02 插值: tick 1200 位于 960 与 1440 的中点
    // 0.5 + (1.5 - 0.5) * 0.5 = 1.0
    assert!(
        (player.spacing_at(1200) - 1.0).abs() < 1e-9,
        "linear interpolation midpoint"
    );

    // keyframe 02: tick 1440 (beat 6), rate 1.5
    assert!((player.spacing_at(1440) - 1.5).abs() < 1e-9, "kf 02");

    // keyframe 02→03 插值: tick 1680 (beat 7), 75% between 1440 and 1920
    // 1.5 + (1.0 - 1.5) * (1680 - 1440) / (1920 - 1440)
    // = 1.5 + (-0.5) * 0.5 = 1.25
    assert!(
        (player.spacing_at(1680) - 1.25).abs() < 1e-9,
        "interpolation at beat 7"
    );

    // keyframe 03: tick 1920 (beat 8), rate 1.0
    assert!((player.spacing_at(1920) - 1.0).abs() < 1e-9, "kf 03");

    // 最后一个关键帧之后 → 保持 1.0
    assert!(
        (player.spacing_at(2400) - 1.0).abs() < 1e-9,
        "after last kf"
    );
}

// BmsCustomEvent 集成测试

/// BGA Base Opacity（通道 0B）产生 `BmsCustomEvent::BgaOpacity`。
#[test]
fn bga_opacity_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#0010B:FF00000000000000\n");
    let customs: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgaOpacity { layer, opacity }) => {
                Some((*layer, *opacity))
            }
            _ => None,
        })
        .collect();
    assert_eq!(customs.len(), 1, "should produce one BgaOpacity event");
    assert_eq!(customs[0], (bmsrs_chart::BgaLayer::Base, 0xFF));
}

/// TEXT 通道（99）产生 `BmsCustomEvent::TextDisplay`。
#[test]
fn text_display_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00199:0100000000000000\n");
    let texts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::TextDisplay { text_index }) => Some(*text_index),
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], 1);
}

/// BGM Volume（通道 97）产生 `BmsCustomEvent::BgmVolume`。
#[test]
fn bgm_volume_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00197:8000000000000000\n");
    let vols: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgmVolume { volume }) => Some(*volume),
            _ => None,
        })
        .collect();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0], 0x80);
}

/// Option 通道（A6）产生 `BmsCustomEvent::OptionChange`。
#[test]
fn option_change_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A6:0500000000000000\n");
    let opts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::OptionChange { option_id, .. }) => Some(*option_id),
            _ => None,
        })
        .collect();
    assert_eq!(opts.len(), 1);
    assert_eq!(opts[0], 5, "option_id should be decoded as base36");
}

/// ARGB 通道（A1-A4）需要 `#ARGBxx` 定义来解析颜色，未定义时跳过。
#[test]
fn argb_custom_event_skips_when_undefined() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A1:0100000000000000\n");
    // ARGB index 01 未定义，无 `#ARGB01` 头部，因此不应产生事件。
    let argb_count = chart
        .data
        .events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Custom(BmsCustomEvent::BgaArgb { .. })))
        .count();
    assert_eq!(argb_count, 0, "undefined ARGB should be skipped");
}

/// SWBGA 通道（A5）产生 `BmsCustomEvent::BgaKeyBound`。
#[test]
fn swbga_custom_event() {
    let chart = process(
        "#BPM 120\n#BMP01 bg.png\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A5:0100000000000000\n",
    );
    let keybounds: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgaKeyBound { resource_id }) => Some(*resource_id),
            _ => None,
        })
        .collect();
    assert_eq!(keybounds.len(), 1);
    assert_eq!(
        keybounds[0], 0,
        "BMP index 01 经 bmp_map 查表映射到 0 基 resource_id"
    );
}

/// Judge 通道（A0）产生 `BmsCustomEvent::JudgeOverride`。
#[test]
fn judge_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#001A0:5000000000000000\n");
    let judges: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::JudgeOverride { rank }) => Some(*rank),
            _ => None,
        })
        .collect();
    assert_eq!(judges.len(), 1);
    // "50" in base36 = 5*36 + 0 = 180
    assert_eq!(judges[0], 180, "base36 '50' should decode to 180");
}

/// Key Volume 通道（98）产生 `BmsCustomEvent::KeyVolume`。
#[test]
fn key_volume_custom_event() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00198:4000000000000000\n");
    let vols: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::KeyVolume { volume }) => Some(*volume),
            _ => None,
        })
        .collect();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0], 0x40, "hex '40' should decode to 64");
}

/// Video Seek 通道（05）查 `#SEEK` 定义表产生 `BmsCustomEvent::VideoSeek`，
/// position 为定义表中的毫秒值（C4：不再使用 base36 原值）。
#[test]
fn video_seek_custom_event() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#SEEK03 500\n#00101:1100000000000000\n#00105:0300000000000000\n",
    );
    let seeks: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::VideoSeek { position }) => Some(*position),
            _ => None,
        })
        .collect();
    assert_eq!(seeks.len(), 1);
    assert_eq!(seeks[0], 500, "#SEEK03 500 应映射为 500 毫秒");
}

/// Video Seek 通道引用未定义的 `#SEEK` id 时，事件被跳过（C4）。
#[test]
fn video_seek_undefined_skipped() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00101:1100000000000000\n#00105:0300000000000000\n");
    let seek_count = chart
        .data
        .events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Custom(BmsCustomEvent::VideoSeek { .. })))
        .count();
    assert_eq!(seek_count, 0, "未定义 #SEEK 的 id 应被跳过");
}

/// 自定义事件插入后不影响原生事件的排序与查询。
#[test]
fn custom_events_dont_affect_note_queries() {
    let chart =
        process("#BPM 120\n#WAV01 kick.wav\n#00111:0100000000000000\n#0010B:FF00000000000000\n");
    let player = Player::new(chart).unwrap();
    let note_count = player
        .events_in_range(0..2000)
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Note { .. }))
        .count();
    assert_eq!(note_count, 1, "note should still be present");
}

// F1：non_event_data 查表归一化（Base36 小写索引）

/// ARGB 通道（A1）使用小写索引引用小写定义的 `#ARGB`，归一化后应命中（F1）。
#[test]
fn argb_lowercase_index_resolves_after_normalize() {
    // #ARGBaa 定义小写 id；通道 A1 用小写 "aa" 引用。
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#ARGBaa 10,20,30,40\n#00101:0100000000000000\n#001A1:aa00000000000000\n",
    );
    let argbs: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::BgaArgb { a, r, g, b, .. }) => Some((*a, *r, *g, *b)),
            _ => None,
        })
        .collect();
    assert_eq!(argbs.len(), 1, "小写 ARGB 索引经归一化应命中定义表");
    assert_eq!(argbs[0], (10, 20, 30, 40));
}

/// BGA `KeyBound` 通道（A5）使用小写索引引用小写定义的 `#BMP`（F1）。
#[test]
fn keybound_lowercase_index_resolves_after_normalize() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#BMPaa bg.png\n#00101:0100000000000000\n#001A5:aa00000000000000\n",
    );
    let keybound_count = chart
        .data
        .events
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                EventKind::Custom(BmsCustomEvent::BgaKeyBound { .. })
            )
        })
        .count();
    assert_eq!(
        keybound_count, 1,
        "小写 KeyBound 索引经归一化应命中 bmp_map"
    );
}

/// Option 通道（A6）+ `#CHANGEOPTION` 定义均用小写，归一化后双向自洽（F1+F2）。
#[test]
fn option_lowercase_def_and_ref_resolve() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#CHANGEOPTIONaa opt_value\n#00101:0100000000000000\n#001A6:aa00000000000000\n",
    );
    let opts: Vec<_> = chart
        .data
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Custom(BmsCustomEvent::OptionChange { value, .. }) => Some(value.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(opts.len(), 1, "小写 Option 定义 + 引用经归一化应匹配");
    assert_eq!(opts[0], "opt_value");
}

// F3a：#LNMODE → ln_type_hint

/// `#LNMODE` 各值映射到 `LnTypeHint`（F3a）。
#[test]
fn ln_mode_maps_to_ln_type_hint() {
    let base = "#BPM 120\n#WAV01 kick.wav\n#00101:0100000000000000\n";
    assert_eq!(
        process(&format!("#LNMODE 1\n{base}")).data.ln_type_hint,
        bmsrs_chart::LnTypeHint::Ln,
    );
    assert_eq!(
        process(&format!("#LNMODE 2\n{base}")).data.ln_type_hint,
        bmsrs_chart::LnTypeHint::Cn,
    );
    assert_eq!(
        process(&format!("#LNMODE 3\n{base}")).data.ln_type_hint,
        bmsrs_chart::LnTypeHint::Hcn,
    );
    // 未声明 #LNMODE 时默认为 Ln。
    assert_eq!(process(base).data.ln_type_hint, bmsrs_chart::LnTypeHint::Ln,);
}

// C1：#BGA / #@BGA → BgaResource.crop

/// `#BGA` 定义产生带裁剪的 BGA 资源（C1）。
#[test]
fn bga_crop_def_attaches_crop_rect() {
    // #BMP01 为源；#BGA01 裁剪自 BMP 1（十进制），矩形 (0,0)-(100,100)，偏移 (10,20)。
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#BMP01 src.png\n#BGA01 1 0 0 100 100 10 20\n#00104:0100000000000000\n",
    );
    let cropped: Vec<_> = chart
        .chart
        .bga_resources
        .iter()
        .filter_map(|r| r.crop.map(|c| (r.path.to_string_lossy().into_owned(), c)))
        .collect();
    assert_eq!(cropped.len(), 1, "应有一个带裁剪的 BGA 资源");
    let (path, crop) = &cropped[0];
    assert_eq!(path, "src.png", "路径取自源 BMP");
    assert_eq!((crop.x1, crop.y1, crop.x2, crop.y2), (0, 0, 100, 100));
    assert_eq!((crop.dx, crop.dy), (10, 20));
}

/// `#@BGA`（宽/高形式）归一为右下角形式（C1）。
#[test]
fn at_bga_wh_form_normalizes_to_xy2() {
    // #@BGA：sx=0 sy=0 w=50 h=40 → x2=50 y2=40。
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#BMP01 src.png\n#@BGA01 1 0 0 50 40 0 0\n#00104:0100000000000000\n",
    );
    let cropped: Vec<_> = chart
        .chart
        .bga_resources
        .iter()
        .filter_map(|r| r.crop)
        .collect();
    assert_eq!(cropped.len(), 1);
    let c = cropped[0];
    assert_eq!((c.x1, c.y1, c.x2, c.y2), (0, 0, 50, 40), "w/h 应转为 x2/y2");
}

// C2：#VIDEOFILE / #MOVIE → VideoAsset

/// `#VIDEOFILE` 产生循环视频资源（C2）。
#[test]
fn videofile_produces_looping_video_asset() {
    let chart = process(
        "#BPM 120\n#WAV01 kick.wav\n#VIDEOFILE bg.mp4\n#VIDEOf/s 30\n#VIDEOCOLORS 16\n#VIDEODLY 5\n#00101:0100000000000000\n",
    );
    let video = chart.chart.video.as_ref().expect("应有视频资源");
    assert_eq!(video.path.to_string_lossy(), "bg.mp4");
    assert!(video.loop_playback, "#VIDEOFILE 应循环");
    assert_eq!(video.fps, Some(30.0));
    assert_eq!(video.colors, Some(16));
    assert_eq!(video.delay_frames, Some(5));
}

/// `#MOVIE` 产生单次播放视频资源（C2）。
#[test]
fn movie_produces_non_looping_video_asset() {
    let chart = process("#BPM 120\n#WAV01 kick.wav\n#MOVIE intro.mp4\n#00101:0100000000000000\n");
    let video = chart.chart.video.as_ref().expect("应有视频资源");
    assert_eq!(video.path.to_string_lossy(), "intro.mp4");
    assert!(!video.loop_playback, "#MOVIE 应单次播放");
    assert!(video.fps.is_none(), "未声明 #VIDEOf/s 时 fps 为 None");
}
