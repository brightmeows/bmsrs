# bmsrs-bms-values-derive

## Role

`#[derive(BmsTokenAttr)]` proc-macro crate. It handles every
`#[bms_token("...")]` pattern automatically.

Pure compile-time logic, no runtime dependencies.

## Three modes

| Mode | Detection | Generated items |
|------|-----------|-----------------|
| **Command** | `#[bms_token("#BPM{id} {value}")]` or `#[bms_token("#TITLE {}")]` | `try_match_header` + `format_header` |
| **Literal** | `#[bms_token("0")]` (no `#` or `%` prefix) | `FromStr` + `Display` |
| **Dispatch** | No `#[bms_token]`; all variants are single tuple variants | `BmsHeader::try_match_header` |

## Module structure

| File | Responsibility |
|------|----------------|
| `lib.rs` | derive macro entry point, mode detection |
| `parse.rs` | template string parsing (`"#BPM{id} {value}"` to `BmsTokenTemplate`) |
| `codegen.rs` | generates `try_match_header` / `format_header` (command and dispatch modes) |
| `value_codegen.rs` | generates `FromStr` / `Display` (literal mode) |

## Generated functions

| Function | Mode | Purpose |
|----------|------|---------|
| `try_match_header` | Command | sub-enum parse entry point |
| `format_header` | Command | enum variant to `(command, value)` |
| `BmsHeader::try_match_header` | Dispatch | top-level dispatch |
| `FromStr` / `Display` | Literal | value-to-string mapping |

## Attributes

| Attribute | Effect |
|-----------|--------|
| `#[bms_token("...")]` | command pattern or literal value |
| `#[bms_fallback]` | Command mode: failure returns `Ok(None)`; Dispatch mode: skip this variant |
| `#[doc(hidden)]` | skip codegen (used for phantom variants) |

## `String` field detection

When the field type is `String`, it is parsed through `String::from(value)` (an
owned conversion) rather than `FromStr` / `BmsValue`.

## Error mapping

The macro converts every `FromStr::Err` into a `BmsTokenizeError` through
`IntoTokensError`.

| Built-in implementation | Meaning |
|-------------------------|---------|
| `ParseIntError` / `ParseFloatError` | standard-library integer and float parse errors |
| `ParseBmsValueError` | generic BMS value parse error |
| `BmsChannelIdError` | channel number parse error |
| `ParseDifficultyError` | difficulty level parse error |

Custom `FromStr` types implement `IntoTokensError` to control the error variant.

## Tests

```bash
# Tested indirectly through bmsrs-bms-values roundtrip tests
cargo test -p bmsrs-bms-values
# Polymorphic tests
```

Proc-macro unit tests live in `parse.rs` (using `proc_macro2` types).
