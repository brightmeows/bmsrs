//! Chumsky-based JSON parser that produces [`serde_json::Value`].
//!
//! Parser supports error recovery: missing commas, trailing commas, and
//! unmatched brackets produce warnings but continue parsing where possible.
//! If no output value is produced the errors are fatal.

use chumsky::error::RichReason;
use chumsky::prelude::*;
use serde_json::Value;

/// Parser error type.
pub type ParseError<'a> = Rich<'a, char>;

/// Parser result: optional output value with a list of errors.
pub type ParseResult<'a, T> = (Option<T>, Vec<ParseError<'a>>);

/// Build a chumsky-based JSON parser.
///
/// The parser recovers from common errors (missing/trailing commas,
/// unmatched brackets) and produces [`serde_json::Value`] on success.
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
                // JSON forbids leading zeros on numbers (except the literal "0").
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

        // Support normal commas, missing commas (emit error but continue),
        // and trailing commas.
        let subsequent_member = choice((
            // Normal: comma then member.
            just(',').padded().ignore_then(member.clone()).map(Some),
            // Missing comma: directly another member. Emit an error.
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
            // Trailing comma: consume it and yield no item.
            just(',').padded().to::<Option<(String, Value)>>(None),
        ));

        // Members: optional first member followed by more members.
        // If the first member is absent (leading comma), emit a diagnostic.
        let members = member
            .clone()
            .or_not()
            .then(subsequent_member.repeated().collect::<Vec<_>>())
            .validate(|(first_opt, rest), e, emitter| {
                if first_opt.is_none() && rest.iter().flatten().next().is_some() {
                    // First member missing but subsequent members exist —
                    // this means the input has a leading comma.
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

/// Parse a JSON string into a [`serde_json::Value`].
///
/// Returns the parsed value (if any) and a list of errors.
/// Non-fatal errors (recovered) are included in the list; the value is
/// still returned.  Fatal errors (no output) result in `None`.
#[must_use]
pub fn parse_json(input: &str) -> ParseResult<'_, Value> {
    parser().parse(input.trim()).into_output_errors()
}

/// Classify chumsky `Rich` errors into warning / recovered / fatal categories.
///
/// | Category | Condition |
/// |---|---|
/// | `Warning` | `RichReason::Custom` (parser diagnostics emitted via `Rich::custom`) |
/// | `Recovered` | Other errors when an output value was produced |
/// | `Fatal` | Other errors when no output value was produced |
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
