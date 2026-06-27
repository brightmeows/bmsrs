//! 泛型字符串容器 `C` 的多态性测试。
//!
//! 验证用不同 `C` 类型分词产生语义
//! 等价的结果，且 `C = String` 产生 owned token。

use std::num::NonZeroUsize;

use proptest::prelude::*;

use bms_tokenizer::{
    BmsHeader, BmsHeaderMetadata, BmsHeaderResDefAudio, BmsToken, BmsTokenizeError, BmsTokenizer,
};

/// 带显式 C 类型的 `Vec<(line, result)>`。
type TokenVec<C> = Vec<(NonZeroUsize, Result<BmsToken<C>, BmsTokenizeError<C>>)>;

/// 用 `&str` 与 `String` 分词——验证行号与 Ok 状态相同。
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

/// `C = String` 的 token 拥有自身的字符串数据（不借用输入）。
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

/// derive 宏正确区分 C 字段与数值字段。
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

proptest! {
    #[test]
    fn tokenize_never_panics_on_valid_input(title in "TITLE [A-Za-z0-9 ]+") {
        let input = format!("#{title}");
        let _tokens: Vec<_> = BmsTokenizer::new()
            .tokenize::<Vec<_>, &str>(&input);
        // 应始终产生某种结果，绝不 panic
    }
}
