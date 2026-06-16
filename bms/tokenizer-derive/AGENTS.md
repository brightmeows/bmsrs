# bms-tokenizer-derive

Proc-macro crate for `#[derive(BmsTokenAttr)]` — handles all
`#[bms_token("...")]` patterns via three auto-detected modes.

## Modes

| Mode | Detection | Generated items |
|---|---|---|
| **Command** | `#[bms_token("#BPM{id} {value}")]` or `#[bms_token("#TITLE {}")]` | `try_match_header`, `format_header` |
| **Literal** | `#[bms_token("0")]` (no `#`/`%` prefix) | `FromStr`, `Display` |
| **Dispatch** | no `#[bms_token]`, all single-tuple | `BmsHeader::try_match_header` |

## Module layout

- `lib.rs` — derive macro entry point, mode detection
- `parse.rs` — template string parser (`"#BPM{id} {value}"` or `"#TITLE {}"` → `BmsTokenTemplate`)
- `codegen.rs` — generates `try_match_header` / `format_header` (command mode)
  and `BmsHeader::try_match_header` (dispatch mode)
- `value_codegen.rs` — generates `FromStr` / `Display` (literal mode)

## Generated functions

- `try_match_header` — per-sub-enum parsing (command mode)
- `format_header` — formats an enum variant back to `(command, value)`
- `BmsHeader::try_match_header` — top-level dispatch (dispatch mode)
- `FromStr` / `Display` — value-to-string mapping (literal mode)

## Attributes

- `#[bms_token("...")]` — command pattern or literal value
- `#[bms_fallback]` — in command mode: parse failure → `Ok(None)`;
  in dispatch mode: skip this variant
- `#[doc(hidden)]` — skip variant in codegen (used for phantom variants)

## `C` field detection

First type param on enum = string container.  Fields matching that ident are
parsed via `<C as From<&str>>::from(value)` instead of `FromStr`/`BmsValue`.
`#[doc(hidden)]` variants are always skipped.

## Error mapping

The macro converts every `FromStr::Err` into a [`BmsTokenizeError`] via
[`IntoTokensError`].  Built-in impls exist for `ParseIntError`/`ParseFloatError`
and common BMS errors (`ParseBmsValueError`, `BmsChannelIdError`,
`ParseDifficultyError`).  Custom `FromStr` types implement `IntoTokensError`
to control which error variant is produced — no attributes needed on the
header variant.

## Testing

Tested indirectly via `bms-tokenizer`'s roundtrip tests (`bms_token.rs`).
Polymorphism tests live in `bms/tokenizer/tests/generic.rs`.
Proc-macro unit tests live in `parse.rs` (use `proc_macro2` types).
