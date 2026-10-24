//! 测试用逐行分词辅助。
//!
//! `bms-tokenizer` 只负责单行语法，整文件循环在 `bms-document`。控制流
//! 测试的夹具不含注释与编码差异，因此用这个最小辅助把文本转换为 token。

use std::num::NonZeroUsize;

use bms_tokenizer::{BmsToken, parse_header_line, parse_message_line};

/// 把测试文本逐行解析为 `(line, token)`。
pub fn tokenize(input: &str) -> Vec<(NonZeroUsize, BmsToken)> {
    input
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let text = line.trim();
            if text.is_empty() {
                return None;
            }
            let number = NonZeroUsize::new(index.saturating_add(1))?;
            match parse_message_line(text) {
                Ok(Some(message)) => Some((number, BmsToken::Message(message))),
                Ok(None) => match parse_header_line(text, &['#', '%']) {
                    Ok(Some(header)) => Some((number, BmsToken::Header(header))),
                    _ => None,
                },
                Err(_) => None,
            }
        })
        .collect()
}
