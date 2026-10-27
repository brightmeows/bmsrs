//! 集成测试共用的辅助函数。

#![allow(dead_code, reason = "not every integration test uses every helper")]

use bmsrs_chart::{Chart, Event, EventKind};
use osu_processor::{OsuChartMeta, OsuCustomEvent, OsuNoteExt, OsuProcessor};

/// 4K mania 合成谱面。
pub const MANIA_4K: &str = include_str!("../fixtures/mania_4k.osu");

/// 解析并转换 4K 合成谱面。
pub fn chart() -> Chart<OsuNoteExt, OsuCustomEvent> {
    OsuProcessor::from_str(MANIA_4K).unwrap()
}

/// 返回谱面级扩展元数据。
pub fn chart_meta(chart: &Chart<OsuNoteExt, OsuCustomEvent>) -> &OsuChartMeta {
    chart
        .data
        .events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::Custom(OsuCustomEvent::ChartMeta(meta)) => Some(meta),
            _ => None,
        })
        .unwrap()
}

/// 返回全部音符事件。
pub fn note_events(
    chart: &Chart<OsuNoteExt, OsuCustomEvent>,
) -> Vec<&Event<OsuNoteExt, OsuCustomEvent>> {
    chart
        .data
        .events
        .iter()
        .filter(|event| matches!(&event.kind, EventKind::Note { .. }))
        .collect()
}
