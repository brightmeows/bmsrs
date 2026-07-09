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
    InString,
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
                    state = State::InString;
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
                        i += push_raw(input, &mut out, i);
                    }
                },
                _ => {
                    i += push_raw(input, &mut out, i);
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

            State::InString => {
                if b == b'\\' {
                    out.push('\\');
                    i += 1;
                    if i < bytes.len() {
                        i += push_raw(input, &mut out, i);
                    }
                } else if b == b'"' {
                    out.push('"');
                    state = State::Normal;
                    i += 1;
                } else {
                    i += push_raw(input, &mut out, i);
                }
            }
        }
    }

    out
}

/// 将 `input` 中从位置 `start` 开始的一个完整 UTF-8 字符原样追加到 `out`，
/// 返回该字符占用的字节数。
///
/// 注释标记均为 ASCII 字符，因此非 ASCII 内容（如中日韩标题）可整段保留，
/// 无需逐字节扫描。`start` 由调用方保证位于字符边界且小于 `input` 长度，
/// 故切片不会 panic 且 `chars().next()` 必返回 `Some`。
#[expect(clippy::string_slice, reason = "start 由调用方保证位于 UTF-8 字符边界")]
#[expect(
    clippy::expect_used,
    reason = "start < input.len() 由 while 循环边界保证，chars 必非空"
)]
fn push_raw(input: &str, out: &mut String, start: usize) -> usize {
    let ch = input[start..].chars().next().expect("start 在边界内");
    out.push(ch);
    ch.len_utf8()
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
