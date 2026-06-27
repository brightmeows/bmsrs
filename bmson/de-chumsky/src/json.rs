//! 基于 chumsky 的 JSON 解析器，产出 [`serde_json::Value`]。
//!
//! 解析器支持错误恢复：缺失逗号、尾随逗号、括号不匹配会产生警告，
//! 但仍尽可能继续解析。若未产出任何值，则这些错误为致命错误。

use chumsky::error::RichReason;
use chumsky::prelude::*;
use serde_json::Value;

/// 解析器错误类型。
pub type ParseError<'a> = Rich<'a, char>;

/// 解析结果：可选的输出值与一组错误列表。
pub type ParseResult<'a, T> = (Option<T>, Vec<ParseError<'a>>);

/// 构建基于 chumsky 的 JSON 解析器。
///
/// 解析器能从常见错误（缺失/尾随逗号、括号不匹配）中恢复，并在成功时
/// 产出 [`serde_json::Value`]。
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "parser combinator API composes many sub-parsers inline"
)]
pub fn parser<'a>() -> impl Parser<'a, &'a str, Value, extra::Err<Rich<'a, char>>> {
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
                // JSON 禁止数字前导零（字面量 "0" 除外）。
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

        // 支持普通逗号、缺失逗号（发出错误但继续）以及尾随逗号。
        let subsequent_member = choice((
            // 普通：逗号后跟成员。
            just(',').padded().ignore_then(member.clone()).map(Some),
            // 缺失逗号：直接跟另一个成员。发出一个错误。
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
            // 尾随逗号：消费它但不产出项。
            just(',').padded().to::<Option<(String, Value)>>(None),
        ));

        // 成员：可选的首个成员，其后跟随更多成员。
        // 若首个成员缺失（前导逗号），发出一条诊断。
        let members = member
            .clone()
            .or_not()
            .then(subsequent_member.repeated().collect::<Vec<_>>())
            .validate(|(first_opt, rest), e, emitter| {
                if first_opt.is_none() && rest.iter().flatten().next().is_some() {
                    // 首个成员缺失但后续成员存在——
                    // 这意味着输入存在前导逗号。
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

/// 将 JSON 字符串解析为 [`serde_json::Value`]。
///
/// 返回解析得到的值（若有）以及一组错误列表。非致命错误（已恢复）会
/// 包含在列表中，且仍会返回该值；致命错误（无输出）会导致返回 `None`。
#[must_use]
pub fn parse_json(input: &str) -> ParseResult<'_, Value> {
    parser().parse(input.trim()).into_output_errors()
}

/// 将 chumsky 的 `Rich` 错误划分为 warning / recovered / fatal 三类。
///
/// | 类别 | 条件 |
/// |---|---|
/// | `Warning` | `RichReason::Custom`（通过 `Rich::custom` 发出的解析器诊断）|
/// | `Recovered` | 产出值时的其他错误 |
/// | `Fatal` | 未产出值时的其他错误 |
#[must_use]
pub fn classify_errors<'a>(
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
