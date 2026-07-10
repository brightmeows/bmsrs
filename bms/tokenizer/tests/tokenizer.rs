//! `bms-tokenizer` 公共 API 的集成测试。

use std::collections::HashMap;

use bms_tokenizer::{
    BmsHeader, BmsHeaderMetadata, BmsToken, BmsTokenizeError, BmsTokenizer, ErrorStrategy,
};

#[test]
fn tokenize_empty_input() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>("");
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_whitespace_only() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>("  \n  \n  ");
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_comment_only() {
    let tokens: Vec<_> =
        BmsTokenizer::new().tokenize::<_, &str>("// just a comment\n// another one");
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_semicolon_comment() {
    let tokens: Vec<_> =
        BmsTokenizer::new().tokenize::<_, &str>("; debug line\n#TITLE real\n  ; indented comment");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Title("real")
        )))
    ));
}

#[test]
fn tokenize_single_header() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>("#TITLE My Song");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0.get(), 1);
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Title("My Song")
        )))
    ));
}

#[test]
fn tokenize_single_message() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>("#00111:11223344");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0.get(), 1);
    match &tokens[0].1 {
        Ok(BmsToken::Message(msg)) => {
            assert_eq!(msg.track(), 1);
            assert_eq!(msg.channel().to_string(), "11");
            assert_eq!(msg.body, "11223344");
        }
        _ => panic!("expected Message token"),
    }
}

#[test]
fn tokenize_mixed_content() {
    let bms = "\
#TITLE Test Song
#ARTIST Test Artist
#BPM 180
#WAV01 kick.wav
#00111:11223344
#00201:AABB
";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>(bms);
    insta::assert_debug_snapshot!(tokens);
}

#[test]
fn tokenize_crlf_line_endings() {
    let bms = "#TITLE Test\r\n#ARTIST Artist\r\n";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 2);
}

#[test]
fn tokenize_cr_line_endings() {
    let bms = "#TITLE Test\r#ARTIST Artist\r";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 2);
}

#[test]
fn tokenize_mixed_line_endings() {
    let bms = "#TITLE Test\n#ARTIST Artist\r\n#GENRE Piano\r#BPM 180\n";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 2);
    assert_eq!(tokens[2].0.get(), 3);
    assert_eq!(tokens[3].0.get(), 4);
}

#[test]
fn tokenize_with_unknown_header() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>("#UNKNOWN value");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Fallback(_)))
    ));
}

#[test]
fn tokenize_interleaved_headers_and_messages() {
    let bms = "\
#TITLE Song
#00101:11
#ARTIST Me
#00201:22
";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 4);
    assert!(matches!(tokens[0].1, Ok(BmsToken::Header(_))));
    assert!(matches!(tokens[1].1, Ok(BmsToken::Message(_))));
    assert!(matches!(tokens[2].1, Ok(BmsToken::Header(_))));
    assert!(matches!(tokens[3].1, Ok(BmsToken::Message(_))));
}

#[test]
fn collect_all_returns_all_results() {
    let bms = "\
#TITLE Song
// comment
#00111:1122
";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::CollectAll)
        .tokenize::<_, &str>(bms);
    // 注释行被跳过；预期 2 个 token
    assert_eq!(tokens.len(), 2);
    // 行号反映原始输入（注释被跳过）
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 3);
}

#[test]
fn collect_all_continues_past_errors() {
    // #001..: 含有无效通道（点号不是有效的 Base62）
    let bms = "\
#TITLE Song
#001..:1122
#00201:AABB
";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::CollectAll)
        .tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 3);
    // 第一行：OK
    assert!(tokens[0].1.is_ok());
    // 第二行：无效通道错误
    assert!(tokens[1].1.is_err());
    // 第三行：解析成功（越过错误继续）
    assert!(tokens[2].1.is_ok());
}

#[test]
fn fail_fast_stops_at_first_error() {
    let bms = "\
#TITLE Song
#001..:1122
#00201:AABB
";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::FailFast)
        .tokenize::<_, &str>(bms);
    // FailFast 在错误行（第 2 行）停止，包含该行
    assert_eq!(tokens.len(), 2);
    assert!(tokens[0].1.is_ok());
    assert!(tokens[1].1.is_err());
    // 行号正确
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 2);
}

#[test]
fn fail_fast_no_error_returns_all() {
    let bms = "\
#TITLE Song
#00101:1122
";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::FailFast)
        .tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 2);
    assert!(tokens[0].1.is_ok());
    assert!(tokens[1].1.is_ok());
}

#[test]
fn line_number_gaps_with_skipped_lines() {
    let bms = "\
#TITLE A

#BPM 180

#00101:11
";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].0.get(), 1); // #TITLE
    assert_eq!(tokens[1].0.get(), 3); // #BPM（第 2 行为空）
    assert_eq!(tokens[2].0.get(), 5); // #00101（第 4 行为空）
}

#[test]
fn default_strategy_is_collect_all() {
    let tokenizer = BmsTokenizer::new();
    // 默认分词应为 CollectAll
    let bms = "#00101:11\n#001..:FF\n#00201:22";
    let tokens: Vec<_> = tokenizer.tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 3);
    assert!(tokens[0].1.is_ok());
    assert!(tokens[1].1.is_err());
    assert!(tokens[2].1.is_ok());
}

#[test]
fn debug_and_clone_bms_token() {
    let token = BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title("t")));
    let cloned = token.clone();
    assert_eq!(format!("{token:?}"), format!("{cloned:?}"));
}

#[test]
fn tokenizer_error_display_invalid_measure() {
    let err = BmsTokenizeError::InvalidMeasure { value: "abc" };
    assert_eq!(err.to_string(), "invalid measure number: \"abc\"");
}

#[test]
fn tokenizer_error_display_invalid_channel() {
    let err = BmsTokenizeError::InvalidChannel { value: "xyz" };
    assert_eq!(err.to_string(), "invalid channel number: \"xyz\"");
}

#[test]
fn tokenizer_error_trait_is_implemented() {
    fn assert_error<T: std::error::Error>() {}
    assert_error::<BmsTokenizeError<&'static str>>();
}

#[test]
fn tokenizer_error_debug_and_clone() {
    let err = BmsTokenizeError::InvalidMeasure { value: "000" };
    let cloned = err.clone();
    assert_eq!(format!("{err:?}"), format!("{cloned:?}"));
}

#[test]
fn tokenizer_error_partial_eq() {
    let a = BmsTokenizeError::InvalidMeasure { value: "000" };
    let b = BmsTokenizeError::InvalidMeasure { value: "000" };
    assert_eq!(a, b);

    let c = BmsTokenizeError::InvalidChannel { value: "00" };
    assert_ne!(a, c);
}

#[test]
fn tokenize_into_btreemap() {
    let bms = "\
#TITLE Song
#BPM 180
#00101:1122
";
    let map: HashMap<_, _> = BmsTokenizer::new().tokenize::<_, &str>(bms);
    assert_eq!(map.len(), 3);
    assert!(map.contains_key(&std::num::NonZeroUsize::MIN));
    assert!(map.contains_key(&std::num::NonZeroUsize::new(2).unwrap()));
    assert!(map.contains_key(&std::num::NonZeroUsize::new(3).unwrap()));
}

#[test]
fn fail_fast_error_line_included_in_results() {
    let bms = "#00101:11\n#001..:FF";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::FailFast)
        .tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 2);
    // 错误行本身被包含（未被丢弃）
    assert!(tokens[1].1.is_err());
}

#[test]
fn bms_tokenizer_debug_and_clone() {
    let t1 = BmsTokenizer::new().error_strategy(ErrorStrategy::FailFast);
    let t2 = t1.clone();
    let r1: Vec<_> = t1.tokenize::<_, &str>("#TITLE A");
    let r2: Vec<_> = t2.tokenize::<_, &str>("#TITLE A");
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn bms_tokenizer_default() {
    let t = BmsTokenizer::default();
    assert!(t.tokenize::<Vec<_>, &str>("").is_empty());
}

#[test]
fn custom_prefix_filters_percent() {
    // 仅 `#` 前缀时，`%URL` 行应被跳过。
    let bms = "#TITLE Song\n%URL https://example.com";
    let tokens: Vec<_> = BmsTokenizer::new()
        .header_prefixes(&['#'])
        .tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 1);
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Title("Song")
        )))
    ));
}

#[test]
fn custom_prefix_accepts_percent_only() {
    // 仅 `%` 前缀时，`#TITLE` 被跳过，但 `%URL` 被解析。
    let bms = "#TITLE Song\n%URL example.com";
    let tokens: Vec<_> = BmsTokenizer::new()
        .header_prefixes(&['%'])
        .tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 1);
    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(
            BmsHeaderMetadata::Url("example.com")
        )))
    ));
}

#[test]
fn empty_prefixes_skips_all() {
    let bms = "#TITLE Song\n#BPM 180\n#00101:11";
    let tokens: Vec<_> = BmsTokenizer::new()
        .header_prefixes(&[])
        .tokenize::<_, &str>(bms);
    // 仅剩通道消息行（无 `#` 行作为头部）
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].1, Ok(BmsToken::Message(_))));
}

#[test]
fn custom_prefix_single_char() {
    let bms = "#TITLE A\n@CUSTOM value";
    let tokens: Vec<_> = BmsTokenizer::new()
        .header_prefixes(&['#', '@'])
        .tokenize::<_, &str>(bms);
    assert_eq!(tokens.len(), 2);
    // `@CUSTOM` 不是已知头部，因此回退到 Fallback
    assert!(matches!(
        tokens[1].1,
        Ok(BmsToken::Header(BmsHeader::Fallback(_)))
    ));
}

#[test]
fn header_prefixes_default() {
    let default = BmsTokenizer::new();
    let explicit = BmsTokenizer::new().header_prefixes(&['#', '%']);
    let bms = "#TITLE A\n%URL b\n";
    let r1: Vec<_> = default.tokenize::<_, &str>(bms);
    let r2: Vec<_> = explicit.tokenize::<_, &str>(bms);
    assert_eq!(r1.len(), r2.len());
    assert_eq!(r1, r2);
}
