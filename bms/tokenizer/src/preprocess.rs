//! BMS 去注释预处理器。
//!
//! 本模块提供一个词法级 pass，在分词前移除 BMS 注释语法。
//! 它处理 BMS 控制流文档中规定的三种注释形式：
//!
//! - `//` —— 单行注释（从标记处剥离到行尾）。
//! - `;` —— 单行注释，**仅限行首**（在可选空白之后）。
//! - `/* ... */` —— 块注释（不可嵌套，可跨多行）。
//!
//! 双引号字符串（`"..."`）内的注释标记会被保留，
//! 与 `IIDXv` / HDX 的字符串转义行为一致。

/// 逐字符注释扫描器的状态。
enum State {
    /// 不在任何注释或字符串中。
    Normal,
    /// 在 `//` 单行注释中。
    LineComment,
    /// 在 `/*` 块注释中。
    BlockComment,
    /// 在块注释中遇到 `*` 之后（可能是 `*/`）。
    BlockMaybeEnd,
    /// 在 `"..."` 引号字符串中——注释标记为字面字符。
    InString(u8),
}

/// 从源文本中剥离 BMS 注释，返回 owned 的 [`String`]。
///
/// # 移除内容
///
/// | 语法 | 范围 | 示例 |
/// |--------|-------|---------|
/// | `//` 到行尾 | 行中任意位置 | `#TITLE foo // comment` => `#TITLE foo ` |
/// | `;` 到行尾 | 仅限行首（trim 后）| `; debug` => _（整行移除）_ |
/// | `/* ... */` | 多行，不可嵌套 | `/* block */#TITLE x` => `#TITLE x` |
///
/// # 字符串转义
///
/// 双引号字符串（`"..."`）内的注释标记被视为普通字符，
/// 与 `IIDXv` / HDX 的字符串转义约定一致。
///
/// 字符串内反斜杠前缀的字符被视为转义
/// （例如 `\"` 产生字面引号，`\\` 产生字面反斜杠）。
///
/// # 用法
///
/// 在分词**之前**调用此函数，然后以 `C = String` 分词：
///
/// ```ignore
/// let cleaned = bms_tokenizer::preprocess(raw_bms);
/// let tokens: Vec<(_, _)> = BmsTokenizer::new()
///     .tokenize::<Vec<_>, String>(&cleaned);
/// ```
#[must_use]
#[expect(clippy::indexing_slicing, reason = "guard checks ensure bounds")]
pub fn preprocess(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut state = State::Normal;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        match &mut state {
            State::Normal => match b {
                b'"' => {
                    out.push('"');
                    state = State::InString(0);
                    i += 1;
                }
                b';' if at_line_start(bytes, i) => {
                    state = State::LineComment;
                    i += 1;
                }
                b'/' if i + 1 < bytes.len() => match bytes[i + 1] {
                    b'/' => {
                        state = State::LineComment;
                        i += 2;
                    }
                    b'*' => {
                        state = State::BlockComment;
                        i += 2;
                    }
                    _ => {
                        out.push('/');
                        i += 1;
                    }
                },
                _ => {
                    out.push(b as char);
                    i += 1;
                }
            },

            State::LineComment => {
                if b == b'\n' || b == b'\r' {
                    out.push(b as char);
                    state = State::Normal;
                }
                i += 1;
            }

            State::BlockComment => {
                if b == b'*' {
                    state = State::BlockMaybeEnd;
                }
                i += 1;
            }

            State::BlockMaybeEnd => match b {
                b'/' => {
                    state = State::Normal;
                    i += 1;
                }
                b'*' => {
                    i += 1;
                }
                _ => {
                    state = State::BlockComment;
                    i += 1;
                }
            },

            State::InString(escape) => {
                if *escape == 0 && b == b'\\' {
                    out.push(b as char);
                    i += 1;
                    if let Some(&next) = bytes.get(i) {
                        out.push(next as char);
                        i += 1;
                    }
                } else if *escape == 0 && b == b'"' {
                    out.push('"');
                    state = State::Normal;
                    i += 1;
                } else {
                    out.push(b as char);
                    i += 1;
                }
            }
        }
    }

    out
}

/// 检查 `bytes` 中位置 `i` 是否位于行首（或仅在引导空白之后），
/// 用于 `;` 注释检测。
fn at_line_start(bytes: &[u8], mut i: usize) -> bool {
    loop {
        i = match i.checked_sub(1) {
            None => return true,
            Some(prev) => match bytes.get(prev) {
                Some(b' ' | b'\t') => prev,
                Some(b'\n' | b'\r') => return true,
                _ => return false,
            },
        };
    }
}
