//! scroll 事件编辑的集成测试（BMS 与 bmson 双侧）。

use std::io::Write as _;

use bmsedit::output::Output;

/// 构造静默输出缓冲。
const fn silent() -> Output {
    Output::new(true)
}

/// 写临时谱面文件。
#[expect(clippy::expect_used, reason = "test helper panics on invalid input")]
fn temp_chart(name: &str, content: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("bmsedit-scroll-{name}-{}", std::process::id()));
    let mut file = std::fs::File::create(&path).expect("create temp file");
    file.write_all(content.as_bytes()).expect("write");
    path
}

const BMS_CHART: &str = "#TITLE t\n#BPM 160\n#SCROLL01 1.5\n#001SC:0101\n";

#[test]
fn bms_scroll_remove_deletes_only_target_slot() {
    // `#001SC:0101` 携带两个事件（1/0 与 1/2）；删除 1/2 后 1/0 保留，
    // 行分辨率按剩余事件重取整（单事件 → 4 槽）。
    let path = temp_chart("bms", BMS_CHART);
    bmsedit::exec_bms::timing_scroll_remove(&path, 1, "1:1/2", &mut silent()).expect("remove");
    let text = std::fs::read_to_string(&path).expect("read back");
    assert_eq!(text, "#TITLE t\n#BPM 160\n#SCROLL01 1.5\n#001SC:01000000\n");
    std::fs::remove_file(&path).ok();
}

#[test]
fn bms_scroll_remove_without_event_errors() {
    let path = temp_chart("bms-miss", BMS_CHART);
    let err = bmsedit::exec_bms::timing_scroll_remove(&path, 1, "1:3/4", &mut silent())
        .expect_err("should fail without event");
    assert_eq!(err.code(), "usage");
    std::fs::remove_file(&path).ok();
}

const BMSON_CHART: &str = r#"{"version":"2.0.0",
  "song_info":{"title":"t","artist":"a","genre":"g"},
  "chart_info":{"subtitle":"","subartists":[],"chart_name":"N","level":1,
    "bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},
  "scroll_events":[{"y":960,"rate":2.0}],
  "chart_data":{"mode_hint":"beat-7k","ln_type_hint":"ln","ln_judge_hint":"normal",
    "ln_life_hint":"normal","init_bpm":160.0,"resolution":240,"lines":null,
    "bpm_events":[],"stop_events":[],
    "sound_channels":[{"name":"k.wav","note_events":[]}]}}"#;

#[test]
fn bmson_scroll_add_inserts_sorted_event() {
    let path = temp_chart("bmson", BMSON_CHART);
    bmsedit::exec_bmson::timing_scroll_add(&path, "2:1/2", -0.5, &mut silent()).expect("add");
    let text = std::fs::read_to_string(&path).expect("read back");
    let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");
    let events = value["scroll_events"].as_array().expect("scroll array");
    let ys: Vec<u64> = events.iter().filter_map(|e| e["y"].as_u64()).collect();
    assert_eq!(
        ys,
        vec![960, 2400],
        "sorted by pulse; measure 2 starts at 1920, +1/2 = 2400"
    );
    assert_eq!(events[1]["rate"].as_f64(), Some(-0.5));
    std::fs::remove_file(&path).ok();
}

#[test]
fn bmson_scroll_remove_deletes_event_at_pulse() {
    let path = temp_chart("bmson-rm", BMSON_CHART);
    bmsedit::exec_bmson::timing_scroll_remove(&path, "1:0", &mut silent()).expect("remove");
    let text = std::fs::read_to_string(&path).expect("read back");
    let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");
    assert_eq!(
        value["scroll_events"].as_array().expect("array").len(),
        0,
        "event should be gone"
    );
    std::fs::remove_file(&path).ok();
}
