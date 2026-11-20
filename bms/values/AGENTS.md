# bmsrs-bms-values

## Role

BMS single-line syntax and typed values: headers/messages, index types,
single-line parsing and encoding. It does not own whole-file loops and does not
build a CST; `bmsrs-bms-syntax` owns whole-file structure, `bmsrs-bms-parser`
owns CST to `Bms`, and `bmsrs-bms-analysis` reuses this crate's value types
directly.

## Three-layer parsing

Header parsing runs command match, then value parse, then fallback. A failed
command match falls through to `BmsHeaderFallback`; a failed value parse gives
`BmsTokenizeError`, unless `#[bms_fallback]` captures it as `Ok(None)`.

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| `BmsIndex` comparison is case-sensitive | Standard BMS normalizes to uppercase in the parser; Base62 preserves the original case |
| A leading `##` is not a header | `BmsHeader::parse` returns `Ok(None)`; the syntax layer classifies it as a comment |
| Unified line entry | `BmsToken::parse_line` tries `BmsMessage::parse` first, then `BmsHeader::parse`, so non-canonical `#1:0101` lines are messages |
| jbms-style colon separator | For `#` headers, a colon directly after the command word (no whitespace before it) splits name and value (`#STOP01:48`, `#TITLE:xxx`); the first colon splits and later colons stay in the value; `%` commands and unknown commands keep the old unrecognized path |
| The `#` and `%` prefixes are supplied by the caller | The caller passes the default `['#', '%']` |
| Value types are owned | Single-line entry points do not borrow input, which simplifies cross-layer passing |
| Line spans | `BmsMessage.span` is set by the parser; single-line parsing leaves it `None` |

## Tests

Completion: `cargo test -p bmsrs-bms-values` green.

## Always / Ask / Never

### Always

- When adding a header, add a `#[bms_token("...")]` variant to the corresponding
  domain enum
- For every new sub-enum variant, check whether `From<T>` / `TryFrom<T>` to the
  parent is needed
- When adding a value type, implement `BmsValue` (or `FromStr + Display`)

### Ask

- Adding a `BmsChannel` variant (involves the channel classification map)
- Changing `BmsIndex` comparison semantics

### Never

- Perform whole-file tokenization, encoding detection, or control-flow
  expansion here
- Resolve cross-command references (WAV/BMP reference semantics belong to the
  processor)
- Add `bmsrs-bms-syntax` / `bmsrs-bms-parser` dependencies; this crate must
  remain a leaf that others depend on
