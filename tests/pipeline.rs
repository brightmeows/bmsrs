//! `bmsrs` 整管线入口的集成测试。
//!
//! `bms::from_text` 是 CST → 控制流选择 → 单行解析的组装入口；
//! 这里验证错误策略的两种行为与外部消费者的使用方式一致。

use bmsrs::bms::ErrorStrategy;

#[test]
fn from_text_collect_all_keeps_tokens_after_error() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let (bms, _) = bmsrs::bms::from_text(text, |_| 1, ErrorStrategy::CollectAll)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(bms.metadata.artist.as_deref(), Some("B"));
}

#[test]
fn from_text_collect_all_reports_tokenize_warnings() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let (_, warnings) = bmsrs::bms::from_text(text, |_| 1, ErrorStrategy::CollectAll)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(
        warnings.as_slice(),
        [bmsrs::bms::FromTextWarning::Tokenize { range, .. }] if !range.is_empty()
    ));
}

#[test]
fn from_text_fail_fast_returns_tokenize_error() {
    let text = "#TITLE A\n#BPM abc\n#ARTIST B\n";
    let result = bmsrs::bms::from_text(text, |_| 1, ErrorStrategy::FailFast);
    assert!(matches!(
        result,
        Err(bmsrs::bms::FromTextError::Tokenize { range, .. }) if !range.is_empty()
    ));
}
