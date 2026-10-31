# bmsrs-bms-values

## Role

BMS single-line syntax and typed values: headers/messages, index types,
single-line parsing and encoding. It does not own whole-file loops and does not
build a CST; `bmsrs-bms-syntax` owns whole-file structure, `bmsrs-bms-parser`
owns CST to `Bms`, and `bmsrs-bms-analysis` reuses this crate's value types
directly.

## Boundaries

| Does | Does not |
|------|----------|
| Single-line command matching and value parsing | Line splitting, comment state machine, encoding detection |
| Typed `BmsHeader` / `BmsMessage` | Control-flow expansion and branch selection |
| Base36 / Base62 indices and normalization | The `Bms` aggregate model |
| Single-line roundtrip (parse and format) | Whole-file write-back |

## Three-layer parsing

| Layer | Mechanism | Failure path |
|-------|-----------|--------------|
| Command match | derive matches the command name and extracts `{id}` | No match falls through to `BmsHeaderFallback` |
| Value parse | `String` / `FromStr` / `BmsValue::parse` | Failure gives `BmsTokenizeError`; `#[bms_fallback]` returns `None` |
| Fallback | `#[bms_fallback]` captures unmatched input | Falls through to `BmsHeaderFallback` |

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| `BmsIndex` comparison is case-sensitive | Standard BMS normalizes to uppercase in the parser; Base62 preserves the original case |
| A leading `##` is not a header | `parse_header_line` returns `Ok(None)`; the syntax layer classifies it as a comment |
| The `#` and `%` prefixes are supplied by the caller | The caller passes the default `['#', '%']` |
| Value types are owned | Single-line entry points do not borrow input, which simplifies cross-layer passing |

## Tests

```bash
cargo test -p bmsrs-bms-values
```

Coverage: command matching, value parsing, indices, channel classification,
single-line roundtrip, and derive.

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
