//! BMSON 谱面解析示例 —— 基于 chumsky 的 JSON 解析器与 serde 反序列化。
//!
//! 本示例演示如何使用 chumsky 构建支持错误恢复的 JSON 解析器，
//! 并将 BMSON JSON 文本解析为 `bmson_def::Bmson`（自动检测 v0/v1/v2）。
//!
//! # 运行
//!
//! ```bash
//! cargo run --example bmson_parser -p bmson-def
//! ```

#![expect(
    clippy::missing_docs_in_private_items,
    reason = "example binary; all items are internal"
)]
#![expect(
    clippy::missing_assert_message,
    reason = "example binary test assertions; intent is clear from context"
)]
#![expect(clippy::print_stdout, reason = "example binary prints test results")]
#![expect(clippy::print_stderr, reason = "example binary prints error messages")]
#![expect(
    clippy::unwrap_used,
    reason = "example binary test assertions; intentional panic on failure"
)]

use std::fmt;

use chumsky::error::Rich;

fn main() {
    let tests: &[(&str, fn())] = &[
        ("v2_roundtrip", test_v2_roundtrip),
        ("v1_conversion", test_v1_conversion),
        ("v0_conversion", test_v0_conversion),
        ("unknown_version", test_unknown_version),
        ("malformed_json", test_malformed_json),
        ("empty_input", test_empty_input),
        ("v0_negative_bpm", test_v0_negative_bpm),
        ("v1_missing_field", test_v1_missing_field),
        (
            "trailing_comma_diagnostics",
            test_trailing_comma_diagnostics,
        ),
        ("v2_sound_channels", test_v2_sound_channels),
        ("v1_stop_events", test_v1_stop_events),
        ("v0_sound_channel", test_v0_sound_channel),
        ("v2_scroll_events", test_v2_scroll_events),
        ("json_null", test_json_null),
        ("json_true", test_json_true),
        ("json_false", test_json_false),
        ("json_integer", test_json_integer),
        ("json_float", test_json_float),
        ("json_string", test_json_string),
        ("json_empty_array", test_json_empty_array),
        ("json_empty_object", test_json_empty_object),
        ("json_nested_object", test_json_nested_object),
        ("json_escape_sequences", test_json_escape_sequences),
        ("json_unicode_escape", test_json_unicode_escape),
        ("json_trailing_comma", test_json_trailing_comma),
        ("json_missing_comma", test_json_missing_comma),
        ("json_unterminated_array", test_json_unterminated_array),
        ("json_exponential", test_json_exponential),
        ("json_error_classification", test_json_error_classification),
    ];

    let mut passed: u32 = 0;
    let mut failed: u32 = 0;
    for (name, test) in tests {
        print!("test {name} ... ");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
        if let Err(e) = result {
            println!("FAILED");
            if let Some(msg) = e.downcast_ref::<&str>() {
                eprintln!("  {msg}");
            } else if let Some(msg) = e.downcast_ref::<String>() {
                eprintln!("  {msg}");
            }
            failed += 1;
        } else {
            println!("ok");
            passed += 1;
        }
    }

    let total = tests.len();
    println!("\ntest result: {passed} passed, {failed} failed ({total} total)");
    if failed > 0 {
        std::process::exit(1);
    }
}

//
// 辅助函数：最小 BMSON JSON 字符串
//

/// 最小合法的 v2.0.0 BMSON JSON 字符串。
const fn minimal_v2_json() -> &'static str {
    r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]}}"#
}

/// 最小合法的 v1.0.0 BMSON JSON 字符串。
const fn minimal_v1_json() -> &'static str {
    r#"{"version":"1.0.0","info":{"title":"T","artist":"A","genre":"G","init_bpm":140.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#
}

/// 最小合法的 v0.2.1 BMSON JSON 字符串。
const fn minimal_v0_json() -> &'static str {
    r#"{"info":{"title":"T","artist":"A","genre":"G","initBPM":140.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#
}

//
// 错误类型
//

/// BMSON 解析与反序列化过程中可能出现的错误。
#[derive(Debug, thiserror::Error)]
enum BmsonDeError {
    /// 致命的 JSON 解析错误（chumsky 未产生任何输出）。
    #[error("JSON parse error(s):\n{0}")]
    JsonParse(String),

    /// bmson 版本字符串缺失或无法识别。
    #[error("{0}")]
    UnknownVersion(String),

    /// 从 `serde_json::Value` 反序列化版本特定类型失败。
    #[error("Failed to deserialize {version} bmson: {message}")]
    Deserialize {
        /// 可读的版本标识符。
        version: &'static str,
        /// 底层错误描述。
        message: String,
    },

    /// 从旧版 v0.2.1 格式转换为统一 v2 格式失败。
    #[error("V0 conversion error: {0}")]
    V0Conversion(String),
}

impl From<bmson_def::BmsonError> for BmsonDeError {
    fn from(e: bmson_def::BmsonError) -> Self {
        match e {
            bmson_def::BmsonError::UnknownVersion(v) => Self::UnknownVersion(v),
            bmson_def::BmsonError::V0Conversion(v) => Self::V0Conversion(v),
            _ => Self::UnknownVersion(e.to_string()),
        }
    }
}

impl From<bmson_def::v0::TryFromV0Error> for BmsonDeError {
    fn from(e: bmson_def::v0::TryFromV0Error) -> Self {
        Self::V0Conversion(e.message)
    }
}

//
// 版本检测（轻量级字符串扫描）
//

/// 通过扫描 JSON 文本的 `"version"` 字段检测 bmson 格式版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectedVersion {
    /// v0.2.1（遗留，无 `version` 字段）。
    V0,
    /// v1.0.0（扁平 schema）。
    V1,
    /// v2.0.0-rc1（拆分 schema）。
    V2,
}

impl DetectedVersion {
    /// 扫描 JSON 字符串并检测 bmson 版本。
    #[expect(
        clippy::string_slice,
        reason = "JSON bytes for \"version\" key and ASCII version strings; byte indexing is safe"
    )]
    fn detect(json: &str) -> Result<Self, BmsonDeError> {
        let mut depth: u32 = 0;
        let mut in_string = false;
        let mut string_start: usize = 0;
        let mut key_pos: Option<usize> = None;

        let mut chars = json.char_indices();
        while let Some((i, ch)) = chars.next() {
            if in_string {
                if ch == '\\' {
                    chars.next();
                } else if ch == '"' {
                    in_string = false;
                    if depth == 1 && &json[string_start..i] == "version" {
                        key_pos = Some(string_start - 1);
                        break;
                    }
                }
            } else {
                match ch {
                    '{' => depth += 1,
                    '}' => depth = depth.saturating_sub(1),
                    '"' => {
                        in_string = true;
                        string_start = i + 1;
                    }
                    _ => {}
                }
            }
        }

        let Some(pos) = key_pos else {
            return Ok(Self::V0);
        };

        let mut rest = &json[pos + 9..];
        rest = rest.trim_start();
        rest = rest.strip_prefix(':').ok_or_else(|| {
            BmsonDeError::UnknownVersion("malformed version field: expected ':'".into())
        })?;
        rest = rest.trim_start();
        rest = rest.strip_prefix('"').ok_or_else(|| {
            BmsonDeError::UnknownVersion("malformed version field: expected string".into())
        })?;
        let end = rest
            .find('"')
            .ok_or_else(|| BmsonDeError::UnknownVersion("unterminated version string".into()))?;
        let version = &rest[..end];

        if version.starts_with("2.") {
            Ok(Self::V2)
        } else if version.starts_with("1.") {
            Ok(Self::V1)
        } else if version.starts_with("0.") {
            Ok(Self::V0)
        } else {
            Err(BmsonDeError::UnknownVersion(version.to_owned()))
        }
    }
}

//
// BMSON 解析入口
//

/// 将 BMSON JSON 字符串解析为 `bmson_def::Bmson`（自动检测 v0/v1/v2）。
fn parse_bmson(json: &str) -> Result<bmson_def::Bmson<'_>, BmsonDeError> {
    // 1. 运行 chumsky 解析器进行校验。
    let (value, errors) = parse_json(json);
    let had_output = value.is_some();

    // 2. 若解析器完全未产生输出，返回解析错误。
    if !had_output {
        let (_warnings, _recovered, fatal) = classify_errors(errors, false);
        let msg = fatal
            .iter()
            .map(|e| fmt::format(format_args!("{e}")))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(BmsonDeError::JsonParse(msg));
    }

    // 3. 从原始文本中检测 bmson 版本。
    let version = DetectedVersion::detect(json)?;

    // 4. 反序列化；失败时附带 chumsky 诊断信息。
    deserialize_by_version(json, version).map_err(|err| {
        if errors.is_empty() {
            return err;
        }
        match err {
            BmsonDeError::Deserialize {
                version: deser_version,
                message,
            } => {
                let diag = errors
                    .iter()
                    .map(|e| fmt::format(format_args!("{e}")))
                    .collect::<Vec<_>>()
                    .join("; ");
                BmsonDeError::Deserialize {
                    version: deser_version,
                    message: format!("{message}\n  (parser diagnostics: {diag})"),
                }
            }
            other => other,
        }
    })
}

/// 内部：分派到版本特定的反序列化。
fn deserialize_by_version<'a>(
    json: &'a str,
    version: DetectedVersion,
) -> Result<bmson_def::Bmson<'a>, BmsonDeError> {
    match version {
        DetectedVersion::V2 => serde_json::from_str::<bmson_def::Bmson<'a>>(json).map_err(|e| {
            BmsonDeError::Deserialize {
                version: "v2.0.0",
                message: e.to_string(),
            }
        }),
        DetectedVersion::V1 => {
            let v1: bmson_def::v1::Bmson<'a> =
                serde_json::from_str(json).map_err(|e| BmsonDeError::Deserialize {
                    version: "v1.0.0",
                    message: e.to_string(),
                })?;
            Ok(bmson_def::Bmson::from(v1))
        }
        DetectedVersion::V0 => {
            let v0: bmson_def::v0::Bmson<'a> =
                serde_json::from_str(json).map_err(|e| BmsonDeError::Deserialize {
                    version: "v0.2.1",
                    message: e.to_string(),
                })?;
            Ok(bmson_def::Bmson::try_from(v0)?)
        }
    }
}

//
// 基于 chumsky 的 JSON 解析器
//

use chumsky::error::RichReason;
use chumsky::prelude::*;
use serde_json::Value;

/// 解析器错误类型。
type ParseError<'a> = Rich<'a, char>;

/// 解析结果：可选的输出值与一组错误列表。
type ParseResult<'a, T> = (Option<T>, Vec<ParseError<'a>>);

/// 构建基于 chumsky 的 JSON 解析器。
///
/// 解析器能从常见错误（缺失/尾随逗号、括号不匹配）中恢复，
/// 并在成功时产出 `serde_json::Value`。
#[expect(
    clippy::too_many_lines,
    reason = "parser combinator API composes many sub-parsers inline"
)]
fn parser<'a>() -> impl Parser<'a, &'a str, Value, extra::Err<Rich<'a, char>>> {
    recursive(|value| {
        let digits = text::digits(10).to_slice();

        let frac = just('.').then(digits);

        let exp = just('e')
            .or(just('E'))
            .then(one_of("+-").or_not())
            .then(digits);

        let number = just('-')
            .or_not()
            .then(text::int(10))
            .then(frac.or_not())
            .then(exp.or_not())
            .to_slice()
            .validate(|s: &str, e, emitter| {
                if s.len() > 1 && s.as_bytes().first() == Some(&b'0') {
                    emitter.emit(Rich::custom(
                        e.span(),
                        "leading zeros are not allowed in JSON numbers",
                    ));
                }
                s.parse::<i64>()
                    .map(|i| Value::Number(serde_json::Number::from(i)))
                    .or_else(|_| {
                        s.parse::<f64>().map(|f| {
                            Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| {
                                emitter.emit(Rich::custom(e.span(), "number overflow or NaN"));
                                serde_json::Number::from(0)
                            }))
                        })
                    })
                    .unwrap_or_else(|_| {
                        emitter.emit(Rich::custom(e.span(), "invalid numeric literal"));
                        Value::Number(serde_json::Number::from(0))
                    })
            })
            .boxed();

        let escape = just('\\').ignore_then(choice((
            just('\\'),
            just('/'),
            just('"'),
            just('b').to('\x08'),
            just('f').to('\x0C'),
            just('n').to('\n'),
            just('r').to('\r'),
            just('t').to('\t'),
            just('u').ignore_then(text::digits(16).exactly(4).to_slice().validate(
                |hex_digits, e, emitter| {
                    let Ok(codepoint) = u32::from_str_radix(hex_digits, 16) else {
                        emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                        return '\u{FFFD}';
                    };
                    char::from_u32(codepoint).unwrap_or_else(|| {
                        emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                        '\u{FFFD}'
                    })
                },
            )),
        )));

        let string = none_of("\\\"")
            .validate(|c: char, e, emitter| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    emitter.emit(Rich::custom(
                        e.span(),
                        "unescaped control character in string",
                    ));
                }
                c
            })
            .or(escape)
            .repeated()
            .collect::<Vec<char>>()
            .map(|cs| cs.into_iter().collect::<String>())
            .delimited_by(just('"'), just('"'))
            .boxed();

        let array = value
            .clone()
            .separated_by(just(',').padded().recover_with(skip_then_retry_until(
                any().ignored(),
                one_of(",]").ignored(),
            )))
            .allow_trailing()
            .collect()
            .padded()
            .delimited_by(
                just('['),
                just(']')
                    .ignored()
                    .recover_with(via_parser(end()))
                    .recover_with(skip_then_retry_until(any().ignored(), end())),
            )
            .boxed();

        let member = string
            .clone()
            .then_ignore(just(':').padded())
            .then(value.clone());

        let subsequent_member = choice((
            just(',').padded().ignore_then(member.clone()).map(Some),
            member
                .clone()
                .validate(|m, e, emitter| {
                    emitter.emit(Rich::custom(
                        e.span(),
                        "expected ',' between object members",
                    ));
                    m
                })
                .map(Some),
            just(',').padded().to::<Option<(String, Value)>>(None),
        ));

        let members = member
            .clone()
            .or_not()
            .then(subsequent_member.repeated().collect::<Vec<_>>())
            .validate(|(first_opt, rest), e, emitter| {
                if first_opt.is_none() && rest.iter().flatten().next().is_some() {
                    emitter.emit(Rich::custom(e.span(), "leading comma in object"));
                }
                (first_opt, rest)
            })
            .map(|(first, rest)| {
                first
                    .into_iter()
                    .chain(rest.into_iter().flatten())
                    .collect::<Vec<_>>()
            });

        let object = members
            .map(|pairs| {
                let mut map = serde_json::Map::new();
                for (key, val) in pairs {
                    map.insert(key, val);
                }
                Value::Object(map)
            })
            .padded()
            .delimited_by(
                just('{'),
                just('}')
                    .ignored()
                    .recover_with(via_parser(end()))
                    .recover_with(skip_then_retry_until(any().ignored(), end())),
            )
            .boxed();

        choice((
            just("null").to(Value::Null),
            just("true").to(Value::Bool(true)),
            just("false").to(Value::Bool(false)),
            number,
            string.map(Value::String),
            array.map(Value::Array),
            object,
        ))
        .recover_with(via_parser(nested_delimiters(
            '{',
            '}',
            [('[', ']')],
            |_| Value::Null,
        )))
        .recover_with(via_parser(nested_delimiters(
            '[',
            ']',
            [('{', '}')],
            |_| Value::Null,
        )))
        .recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(",]}").ignored(),
        ))
        .padded()
    })
}

/// 将 JSON 字符串解析为 `serde_json::Value`。
fn parse_json(input: &str) -> ParseResult<'_, Value> {
    parser().parse(input.trim()).into_output_errors()
}

/// 将 chumsky 的 `Rich` 错误划分为 warning / recovered / fatal 三类。
fn classify_errors<'a>(
    errors: impl IntoIterator<Item = ParseError<'a>>,
    had_output: bool,
) -> (
    Vec<ParseError<'a>>,
    Vec<ParseError<'a>>,
    Vec<ParseError<'a>>,
) {
    let mut warnings = Vec::new();
    let mut recovered = Vec::new();
    let mut fatal = Vec::new();
    for err in errors {
        match err.reason() {
            RichReason::Custom(_) => warnings.push(err),
            RichReason::ExpectedFound { .. } if had_output => recovered.push(err),
            RichReason::ExpectedFound { .. } => fatal.push(err),
        }
    }
    (warnings, recovered, fatal)
}

//
// 测试用例
//

fn test_v2_roundtrip() {
    let bmson = parse_bmson(minimal_v2_json()).unwrap();
    assert_eq!(bmson.song_info.title, "T");
    assert_eq!(bmson.song_info.artist, "A");
    assert_eq!(bmson.song_info.genre, "G");
    assert_eq!(bmson.chart_info.level, 1);
}

fn test_v1_conversion() {
    let bmson = parse_bmson(minimal_v1_json()).unwrap();
    assert_eq!(bmson.song_info.title, "T");
    assert_eq!(bmson.song_info.artist, "A");
    assert_eq!(bmson.chart_info.level, 1);
}

fn test_v0_conversion() {
    let bmson = parse_bmson(minimal_v0_json()).unwrap();
    assert_eq!(bmson.song_info.title, "T");
    assert_eq!(bmson.song_info.artist, "A");
    assert_eq!(bmson.chart_info.level, 1);
}

fn test_unknown_version() {
    let json = r#"{"version":"3.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]}}"#;
    let result = parse_bmson(json);
    assert!(result.is_err());
    assert!(matches!(result, Err(BmsonDeError::UnknownVersion(_))));
}

fn test_malformed_json() {
    let result = parse_bmson("not json at all");
    assert!(result.is_err());
}

fn test_empty_input() {
    let result = parse_bmson("");
    assert!(result.is_err());
}

fn test_v0_negative_bpm() {
    let json = r#"{"info":{"title":"T","artist":"A","genre":"G","initBPM":-1.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let result = parse_bmson(json);
    assert!(result.is_err());
    assert!(matches!(result, Err(BmsonDeError::V0Conversion(_))));
}

fn test_v1_missing_field() {
    let json = r#"{"version":"1.0.0","info":{"title":"T","genre":"G","init_bpm":140.0,"level":1},"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let result = parse_bmson(json);
    assert!(result.is_err());
}

fn test_trailing_comma_diagnostics() {
    let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},}"#;
    let result = parse_bmson(json);
    assert!(result.is_err());
    match &result {
        Err(BmsonDeError::Deserialize { message, .. }) => {
            assert!(
                message.contains("trailing") || message.contains("diagnostics"),
                "expected chumsky diagnostic in error message, got: {message}"
            );
        }
        other => panic!("expected Deserialize, got: {other:?}"),
    }
}

fn test_v2_sound_channels() {
    let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":5,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":180.0,"lines":null,"bpm_events":[{"y":0,"bpm":180.0}],"stop_events":[],"sound_channels":[{"name":"kick.wav","note_events":[{"x":1,"y":0,"l":0,"c":false}]},{"name":"snare.wav","note_events":[{"x":3,"y":240,"l":0,"c":false}]}]}}"#;
    let bmson = parse_bmson(json).unwrap();
    assert_eq!(bmson.chart_info.level, 5);
    assert_eq!(bmson.chart_data.bpm_events.len(), 1);
    assert_eq!(bmson.chart_data.sound_channels.len(), 2);
}

fn test_v1_stop_events() {
    let json = r#"{"version":"1.0.0","info":{"title":"T","artist":"A","genre":"G","init_bpm":140.0,"level":1},"bpm_events":[{"y":0,"bpm":140.0}],"stop_events":[{"y":480,"duration":240}],"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let bmson = parse_bmson(json).unwrap();
    assert_eq!(bmson.chart_data.bpm_events.len(), 1);
    assert_eq!(bmson.chart_data.stop_events.len(), 1);
}

fn test_v0_sound_channel() {
    let json = r#"{"info":{"title":"T","artist":"A","genre":"G","initBPM":140.0,"level":1},"soundChannel":[{"name":"hat.wav","notes":[{"x":2,"y":120,"l":0,"c":false}]}],"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}}"#;
    let bmson = parse_bmson(json).unwrap();
    assert_eq!(bmson.chart_data.sound_channels.len(), 1);
}

fn test_v2_scroll_events() {
    let json = r#"{"version":"2.0.0","song_info":{"title":"T","artist":"A","genre":"G"},"chart_info":{"subtitle":"","subartists":[],"chart_name":"","level":1,"bga":{"bga_header":[],"bga_events":[],"layer_events":[],"poor_events":[]}},"chart_data":{"init_bpm":140.0,"lines":null,"bpm_events":[],"stop_events":[],"sound_channels":[]},"scroll_events":[{"y":0,"rate":1.0},{"y":960,"rate":2.0}]}"#;
    let bmson = parse_bmson(json).unwrap();
    assert_eq!(bmson.scroll_events.len(), 2);
}

fn test_json_null() {
    let (val, errors) = parse_json("null");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Null));
}

fn test_json_true() {
    let (val, errors) = parse_json("true");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Bool(true)));
}

fn test_json_false() {
    let (val, errors) = parse_json("false");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Bool(false)));
}

fn test_json_integer() {
    let (val, errors) = parse_json("42");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Number(serde_json::Number::from(42))));
}

fn test_json_float() {
    let (val, errors) = parse_json("2.5");
    assert!(errors.is_empty());
    let n = val.as_ref().and_then(Value::as_f64).unwrap();
    assert!((n - 2.5).abs() < 1e-10);
}

fn test_json_string() {
    let (val, errors) = parse_json(r#""hello world""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("hello world".to_owned())));
}

fn test_json_empty_array() {
    let (val, errors) = parse_json("[]");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Array(vec![])));
}

fn test_json_empty_object() {
    let (val, errors) = parse_json("{}");
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::Object(serde_json::Map::new())));
}

fn test_json_nested_object() {
    let json = r#"{"outer": {"inner": 42}}"#;
    let (val, errors) = parse_json(json);
    assert!(errors.is_empty());
    let inner = val
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|m| m.get("outer"))
        .and_then(Value::as_object)
        .and_then(|m| m.get("inner"))
        .and_then(Value::as_u64);
    assert_eq!(inner, Some(42));
}

fn test_json_escape_sequences() {
    let (val, errors) = parse_json(r#""\n\t\r\\\"""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("\n\t\r\\\"".to_owned())));
}

fn test_json_unicode_escape() {
    let (val, errors) = parse_json(r#""\u0041""#);
    assert!(errors.is_empty());
    assert_eq!(val, Some(Value::String("A".to_owned())));
}

fn test_json_trailing_comma() {
    let (val, errors) = parse_json(r#"{"a": 1, "b": 2,}"#);
    assert!(val.is_some());
    assert!(
        errors.is_empty(),
        "trailing commas should parse without errors"
    );
}

fn test_json_missing_comma() {
    let (val, errors) = parse_json(r#"{"a": 1 "b": 2}"#);
    assert!(val.is_some());
    assert!(!errors.is_empty());
}

fn test_json_unterminated_array() {
    let (val, errors) = parse_json("[1, 2, 3");
    assert!(val.is_some());
    assert!(!errors.is_empty());
}

fn test_json_exponential() {
    let (val, errors) = parse_json("1.5e2");
    assert!(errors.is_empty());
    let n = val.as_ref().and_then(Value::as_f64).unwrap();
    assert!((n - 150.0).abs() < 1e-10);
}

fn test_json_error_classification() {
    let (warnings, recovered, fatal) = classify_errors(vec![], true);
    assert!(warnings.is_empty());
    assert!(recovered.is_empty());
    assert!(fatal.is_empty());

    let err = Rich::<'_, char>::custom((0..1).into(), "test diagnostic");
    let (w, _, f) = classify_errors(vec![err], false);
    assert_eq!(w.len(), 1);
    assert!(f.is_empty());
}
