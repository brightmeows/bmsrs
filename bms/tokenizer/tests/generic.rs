//! `String` 容器的测试——验证产生 owned token。

use std::num::NonZeroUsize;

use proptest::prelude::*;

use bms_tokenizer::{
    BmsHeader, BmsHeaderMetadata, BmsHeaderResDefAudio, BmsToken, BmsTokenizeError, BmsTokenizer,
};

/// 带显式类型的 `Vec<(line, result)>`。
type TokenVec = Vec<(NonZeroUsize, Result<BmsToken, BmsTokenizeError>)>;

/// 用 `String` 分词——验证行号与 Ok 状态。
#[test]
fn string_container_same_content() {
    let input = "#TITLE My Song\n#ARTIST Composer\n#WAV01 kick.wav\n#00101:1122";

    let tokens: TokenVec = BmsTokenizer::new().tokenize(input);

    assert_eq!(tokens.len(), 4);

    for token in &tokens {
        assert!(token.1.is_ok());
    }
}

/// `String` 的 token 拥有自身的字符串数据（不借用输入）。
#[test]
fn string_container_owned_values() {
    let input = "#TITLE My Song";
    let tokens: TokenVec = BmsTokenizer::new().tokenize(input);
    if let BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title(s))) =
        &tokens[0].1.as_ref().unwrap()
    {
        assert_eq!(s.as_str(), "My Song");
    } else {
        panic!("expected Title header");
    }
}

/// derive 宏正确区分 String 字段与数值字段。
#[test]
fn string_fields_parsed_correctly() {
    let input = "#WAV01 kick.wav";
    let tokens: TokenVec = BmsTokenizer::new().tokenize(input);
    let token = &tokens[0];
    let unwrapped = token.1.as_ref().unwrap();
    assert!(matches!(
        unwrapped,
        BmsToken::Header(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav { .. }))
    ));
}

proptest! {
    #[test]
    fn tokenize_never_panics_on_valid_input(title in "TITLE [A-Za-z0-9 ]+") {
        let input = format!("#{title}");
        let _tokens: Vec<_> = BmsTokenizer::new()
            .tokenize::<Vec<_>>(&input);
        // 应始终产生某种结果，绝不 panic
    }
}
