//! `bmsrs` 整管线入口的集成测试。
//!
//! `bms::from_text` 是分词 → 控制流展开 → 解析的组装入口；
//! 这里验证错误策略的两种行为与外部消费者的使用方式一致。

use bmsrs::bms::document::ErrorStrategy;

/// 固定返回 1 的 `BranchRng`，不依赖外部 rng 实现。
struct FixedRng;

impl bmsrs::bms::control_flow::BranchRng for FixedRng {
    fn gen_range(&mut self, _max: u64) -> u64 {
        1
    }
}

#[test]
fn from_text_collect_all_keeps_tokens_after_error() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let (bms, _) = bmsrs::bms::from_text(text, &mut FixedRng, ErrorStrategy::CollectAll)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(bms.metadata.artist.as_deref(), Some("B"));
}

#[test]
fn from_text_fail_fast_stops_at_error() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let (bms, _) = bmsrs::bms::from_text(text, &mut FixedRng, ErrorStrategy::FailFast)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(bms.metadata.artist.is_none());
}
