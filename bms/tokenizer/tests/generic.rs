//! Polymorphism tests for the generic string container `C`.
//!
//! Verifies that tokenization with different `C` types produces semantically
//! equivalent results, and that `C = String` produces owned tokens.

use std::num::NonZeroUsize;

use bms_tokenizer::{
    BmsHeader, BmsHeaderMetadata, BmsHeaderResDefAudio, BmsToken, BmsTokenizeError, BmsTokenizer,
};

/// `Vec<(line, result)>` with explicit C type.
type TokenVec<C> = Vec<(NonZeroUsize, Result<BmsToken<C>, BmsTokenizeError<C>>)>;

/// Tokenize with `&str` and `String` — verify same line numbers and Ok status.
#[test]
fn different_c_containers_same_content() {
    let input = "#TITLE My Song\n#ARTIST Composer\n#WAV01 kick.wav\n#00101:1122";

    let tokens_ref: TokenVec<&str> = BmsTokenizer::new().tokenize(input);
    let tokens_string: TokenVec<String> = BmsTokenizer::new().tokenize(input);

    assert_eq!(tokens_ref.len(), tokens_string.len());

    for i in 0..tokens_ref.len() {
        assert_eq!(tokens_ref[i].0, tokens_string[i].0);
        assert!(tokens_ref[i].1.is_ok());
        assert!(tokens_string[i].1.is_ok());
    }
}

/// `C = String` tokens own their string data (no borrow from input).
#[test]
fn string_container_owned_values() {
    let input = "#TITLE My Song";
    let tokens: TokenVec<String> = BmsTokenizer::new().tokenize(input);
    if let BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title(s))) =
        &tokens[0].1.as_ref().unwrap()
    {
        assert_eq!(s.as_str(), "My Song");
    } else {
        panic!("expected Title header");
    }
}

/// Derive macro correctly identifies C fields vs numeric fields.
#[test]
fn c_fields_parsed_correctly() {
    let input = "#WAV01 kick.wav";
    let tokens: TokenVec<&str> = BmsTokenizer::new().tokenize(input);
    let token = &tokens[0];
    let unwrapped = token.1.as_ref().unwrap();
    assert!(matches!(
        unwrapped,
        BmsToken::Header(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav { .. }))
    ));
}
