//! Integration tests for the public API of `bms-tokenizer`.

use bms_tokenizer::{
    tokenize, tokenize_line, BmsHeader, BmsHeaderMetadata, BmsToken, TokenizerError,
};

// ---------------------------------------------------------------------------
// tokenize
// ---------------------------------------------------------------------------

#[test]
fn tokenize_empty_input() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_whitespace_only() {
    let tokens = tokenize("  \n  \n  ").unwrap();
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_comment_only() {
    let tokens = tokenize("// just a comment\n// another one").unwrap();
    assert!(tokens.is_empty());
}

#[test]
fn tokenize_single_header() {
    let tokens = tokenize("#TITLE My Song").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(
        tokens[0],
        BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title("My Song")))
    );
}

#[test]
fn tokenize_single_message() {
    let tokens = tokenize("#00111:11223344").unwrap();
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        BmsToken::Message(msg) => {
            assert_eq!(msg.measure, 1);
            assert_eq!(msg.channel, 11);
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
    let tokens = tokenize(bms).unwrap();
    assert_eq!(tokens.len(), 6);

    assert!(matches!(tokens[0], BmsToken::Header(BmsHeader::Metadata(_))));
    assert!(matches!(tokens[1], BmsToken::Header(BmsHeader::Metadata(_))));
    assert!(matches!(tokens[2], BmsToken::Header(BmsHeader::Timing(_))));
    assert!(matches!(
        tokens[3],
        BmsToken::Header(BmsHeader::ResDefAudio(_))
    ));
    assert!(matches!(tokens[4], BmsToken::Message(_)));
    assert!(matches!(tokens[5], BmsToken::Message(_)));
}

#[test]
fn tokenize_crlf_line_endings() {
    let bms = "#TITLE Test\r\n#ARTIST Artist\r\n";
    let tokens = tokenize(bms).unwrap();
    assert_eq!(tokens.len(), 2);
}

#[test]
fn tokenize_cr_line_endings() {
    let bms = "#TITLE Test\r#ARTIST Artist\r";
    let tokens = tokenize(bms).unwrap();
    assert_eq!(tokens.len(), 2);
}

#[test]
fn tokenize_mixed_line_endings() {
    let bms = "#TITLE Test\n#ARTIST Artist\r\n#GENRE Piano\r#BPM 180\n";
    let tokens = tokenize(bms).unwrap();
    assert_eq!(tokens.len(), 4);
}

#[test]
fn tokenize_with_unknown_header() {
    let tokens = tokenize("#UNKNOWN value").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0], BmsToken::Header(BmsHeader::Ext(_))));
}

#[test]
fn tokenize_interleaved_headers_and_messages() {
    let bms = "\
#TITLE Song
#00101:11
#ARTIST Me
#00201:22
";
    let tokens = tokenize(bms).unwrap();
    assert_eq!(tokens.len(), 4);
    assert!(matches!(tokens[0], BmsToken::Header(_)));
    assert!(matches!(tokens[1], BmsToken::Message(_)));
    assert!(matches!(tokens[2], BmsToken::Header(_)));
    assert!(matches!(tokens[3], BmsToken::Message(_)));
}

// ---------------------------------------------------------------------------
// tokenize_line
// ---------------------------------------------------------------------------

#[test]
fn tokenize_line_empty() {
    assert_eq!(tokenize_line("").unwrap(), None);
}

#[test]
fn tokenize_line_comment_slash() {
    assert_eq!(tokenize_line("// comment").unwrap(), None);
}

#[test]
fn tokenize_line_header() {
    let result = tokenize_line("#TITLE test").unwrap().unwrap();
    assert!(matches!(result, BmsToken::Header(_)));
}

#[test]
fn tokenize_line_message() {
    let result = tokenize_line("#00101:1122").unwrap().unwrap();
    assert!(matches!(result, BmsToken::Message(_)));
}

// ---------------------------------------------------------------------------
// BmsToken
// ---------------------------------------------------------------------------

#[test]
fn debug_and_clone_bms_token() {
    let token = BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title("t")));
    let cloned = token.clone();
    assert_eq!(format!("{token:?}"), format!("{cloned:?}"));
}

// ---------------------------------------------------------------------------
// TokenizerError
// ---------------------------------------------------------------------------

#[test]
fn tokenizer_error_display_invalid_measure() {
    let err = TokenizerError::InvalidMeasure("abc".to_string());
    assert_eq!(err.to_string(), "invalid measure number: \"abc\"");
}

#[test]
fn tokenizer_error_display_invalid_channel() {
    let err = TokenizerError::InvalidChannel("xyz".to_string());
    assert_eq!(err.to_string(), "invalid channel number: \"xyz\"");
}

#[test]
fn tokenizer_error_trait_is_implemented() {
    fn assert_error<T: std::error::Error>() {}
    assert_error::<TokenizerError>();
}

#[test]
fn tokenizer_error_debug_and_clone() {
    let err = TokenizerError::InvalidMeasure("000".to_string());
    let cloned = err.clone();
    assert_eq!(format!("{err:?}"), format!("{cloned:?}"));
}

#[test]
fn tokenizer_error_partial_eq() {
    let a = TokenizerError::InvalidMeasure("000".to_string());
    let b = TokenizerError::InvalidMeasure("000".to_string());
    assert_eq!(a, b);

    let c = TokenizerError::InvalidChannel("00".to_string());
    assert_ne!(a, c);
}
