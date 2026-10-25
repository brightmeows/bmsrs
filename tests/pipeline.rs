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
fn from_text_collect_all_reports_tokenize_warnings() {
    // CollectAll 跳过错误行，但逐条错误必须以警告形式可见。
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let (_, warnings) = bmsrs::bms::from_text(text, &mut FixedRng, ErrorStrategy::CollectAll)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(
        warnings.as_slice(),
        [bmsrs::bms::FromTextWarning::Tokenize { line, .. }] if line.get() == 2
    ));
}

#[test]
fn from_text_fail_fast_returns_tokenize_error() {
    // FailFast 的语义是“在错误处失败”：首个分词错误作为 Err 返回，
    // 携带行号，而非静默截断后返回 Ok。
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let result = bmsrs::bms::from_text(text, &mut FixedRng, ErrorStrategy::FailFast);
    assert!(matches!(
        result,
        Err(bmsrs::bms::FromTextError::Tokenize { line, .. }) if line.get() == 2
    ));
}
