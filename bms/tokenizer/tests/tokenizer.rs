//! Integration tests for the public API of `bms-tokenizer`.

use std::collections::HashMap;

use bms_tokenizer::{
    BmsHeader, BmsHeaderMetadata, BmsToken, BmsTokenizeError, BmsTokenizer, ErrorStrategy,
};

#[test]
fn tokenize_empty_input() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize("");
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_whitespace_only() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize("  \n  \n  ");
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_comment_only() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize("// just a comment\n// another one");
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_single_header() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize("#TITLE My Song");
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
    let tokens: Vec<_> = BmsTokenizer::new().tokenize("#00111:11223344");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0.get(), 1);
    match &tokens[0].1 {
        Ok(BmsToken::Message(msg)) => {
            assert_eq!(msg.measure, 1);
            assert_eq!(msg.channel.as_str(), "11");
            assert_eq!(msg.values, "11223344");
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
    let tokens: Vec<_> = BmsTokenizer::new().tokenize(bms);
    assert_eq!(tokens.len(), 6);

    assert!(matches!(
        tokens[0].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(_)))
    ));
    assert!(matches!(
        tokens[1].1,
        Ok(BmsToken::Header(BmsHeader::Metadata(_)))
    ));
    assert!(matches!(
        tokens[2].1,
        Ok(BmsToken::Header(BmsHeader::Timing(_)))
    ));
    assert!(matches!(
        tokens[3].1,
        Ok(BmsToken::Header(BmsHeader::ResDefAudio(_)))
    ));
    assert!(matches!(tokens[4].1, Ok(BmsToken::Message(_))));
    assert!(matches!(tokens[5].1, Ok(BmsToken::Message(_))));
}

#[test]
fn tokenize_crlf_line_endings() {
    let bms = "#TITLE Test\r\n#ARTIST Artist\r\n";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize(bms);
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 2);
}

#[test]
fn tokenize_cr_line_endings() {
    let bms = "#TITLE Test\r#ARTIST Artist\r";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize(bms);
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 2);
}

#[test]
fn tokenize_mixed_line_endings() {
    let bms = "#TITLE Test\n#ARTIST Artist\r\n#GENRE Piano\r#BPM 180\n";
    let tokens: Vec<_> = BmsTokenizer::new().tokenize(bms);
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 2);
    assert_eq!(tokens[2].0.get(), 3);
    assert_eq!(tokens[3].0.get(), 4);
}

#[test]
fn tokenize_with_unknown_header() {
    let tokens: Vec<_> = BmsTokenizer::new().tokenize("#UNKNOWN value");
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
    let tokens: Vec<_> = BmsTokenizer::new().tokenize(bms);
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
        .tokenize(bms);
    // comment line is skipped; 2 tokens expected
    assert_eq!(tokens.len(), 2);
    // Line numbers reflect original input (comment skipped)
    assert_eq!(tokens[0].0.get(), 1);
    assert_eq!(tokens[1].0.get(), 3);
}

#[test]
fn collect_all_continues_past_errors() {
    // #001GZ: has an invalid channel number (non-hex character G)
    let bms = "\
#TITLE Song
#001GZ:1122
#00201:AABB
";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::CollectAll)
        .tokenize(bms);
    assert_eq!(tokens.len(), 3);
    // First line: OK
    assert!(tokens[0].1.is_ok());
    // Second line: the invalid channel error
    assert!(tokens[1].1.is_err());
    // Third line: parsed successfully (continued past error)
    assert!(tokens[2].1.is_ok());
}

#[test]
fn fail_fast_stops_at_first_error() {
    let bms = "\
#TITLE Song
#001GZ:1122
#00201:AABB
";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::FailFast)
        .tokenize(bms);
    // FailFast stops at the error line (line 2), including it
    assert_eq!(tokens.len(), 2);
    assert!(tokens[0].1.is_ok());
    assert!(tokens[1].1.is_err());
    // Line numbers correct
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
        .tokenize(bms);
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
    let tokens: Vec<_> = BmsTokenizer::new().tokenize(bms);
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].0.get(), 1); // #TITLE
    assert_eq!(tokens[1].0.get(), 3); // #BPM (line 2 is blank)
    assert_eq!(tokens[2].0.get(), 5); // #00101 (line 4 is blank)
}

#[test]
fn default_strategy_is_collect_all() {
    let tokenizer = BmsTokenizer::new();
    // Tokenize with default should be CollectAll
    let bms = "#00101:11\n#001GZ:FF\n#00201:22";
    let tokens: Vec<_> = tokenizer.tokenize(bms);
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
    let err = BmsTokenizeError::InvalidMeasure("abc");
    assert_eq!(err.to_string(), "invalid measure number: \"abc\"");
}

#[test]
fn tokenizer_error_display_invalid_channel() {
    let err = BmsTokenizeError::InvalidChannel("xyz");
    assert_eq!(err.to_string(), "invalid channel number: \"xyz\"");
}

#[test]
fn tokenizer_error_trait_is_implemented() {
    fn assert_error<T: std::error::Error>() {}
    assert_error::<BmsTokenizeError>();
}

#[test]
fn tokenizer_error_debug_and_clone() {
    let err = BmsTokenizeError::InvalidMeasure("000");
    let cloned = err.clone();
    assert_eq!(format!("{err:?}"), format!("{cloned:?}"));
}

#[test]
fn tokenizer_error_partial_eq() {
    let a = BmsTokenizeError::InvalidMeasure("000");
    let b = BmsTokenizeError::InvalidMeasure("000");
    assert_eq!(a, b);

    let c = BmsTokenizeError::InvalidChannel("00");
    assert_ne!(a, c);
}

#[test]
fn tokenize_into_hashmap() {
    let bms = "\
#TITLE Song
#BPM 180
#00101:1122
";
    let map: HashMap<_, _> = BmsTokenizer::new().tokenize(bms);
    assert_eq!(map.len(), 3);
    assert!(map.contains_key(&std::num::NonZeroUsize::MIN));
    assert!(map.contains_key(&std::num::NonZeroUsize::new(2).unwrap()));
    assert!(map.contains_key(&std::num::NonZeroUsize::new(3).unwrap()));
}

#[test]
fn fail_fast_error_line_included_in_results() {
    let bms = "#00101:11\n#001GZ:FF";
    let tokens: Vec<_> = BmsTokenizer::new()
        .error_strategy(ErrorStrategy::FailFast)
        .tokenize(bms);
    assert_eq!(tokens.len(), 2);
    // The error line itself is included (not dropped)
    assert!(tokens[1].1.is_err());
}

#[test]
fn bms_tokenizer_debug_and_clone() {
    let t1 = BmsTokenizer::new().error_strategy(ErrorStrategy::FailFast);
    let t2 = t1.clone();
    let r1: Vec<_> = t1.tokenize("#TITLE A");
    let r2: Vec<_> = t2.tokenize("#TITLE A");
    assert_eq!(r1.len(), r2.len());
}

#[test]
fn bms_tokenizer_default() {
    let t: BmsTokenizer = Default::default();
    assert!(t.tokenize::<Vec<_>>("").is_empty());
}
