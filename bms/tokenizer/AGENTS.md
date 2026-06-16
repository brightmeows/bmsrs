# bms-tokenizer

First stage of `tokenizer → parser → processor`.

## Scope

The tokenizer is a syntactic pass, with one semantic exception:
**channel classification** (e.g. `Bgm` vs `Note` vs `Unknown`) is resolved
at tokenizer level. The raw index is preserved in `Note`/`Unknown` for
downstream interpretation — value-level semantics (WAV/BMP references,
player/key extraction) remain downstream.

Every token exists solely for:
1. **Roundtrip fidelity** — `format_header` reproduces the original line.
2. **Syntax validation** — verifies well-formedness.

## Generic string container `C`

`BmsToken<'a, C>` / `BmsHeader<'a, C>` / `BmsMessage<'a, C>` — `C` defaults
to `&'a str` (zero-copy).  Downstream reads via `.as_ref()`.  Chosen at
`tokenize` call site, propagates through the pipeline.

```rust
// default C = &str
let tokens: Vec<(_, _)> = BmsTokenizer::new().tokenize(input);
// explicit C = String
let owned: Vec<(_, _)> = BmsTokenizer::new().tokenize::<_, String>(input);
```

## New headers

Add `#[bms_token("...")]` variant to the appropriate domain enum.
Derive generates everything — no custom logic in `parse_header_line`.

Three-layer parse:

1. **Command match** (derive) — match command, extract `{id}`.
2. **Value parse** (derive per-field) — `C`, `FromStr`, `BmsValue::parse`,
   or `#[derive(BmsTokenAttr)]` for literal enums.
3. **Fallback** (`#[bms_fallback]`) — failure → `Ok(None)` → `BmsHeaderFallback`.

Derive auto-detects C fields (matching enum's first type param) vs
`FromStr`/`BmsValue` fields.

### Value types

Implement `BmsValue<'a, C>` (or `FromStr + Display` — blanket impl works for
any `C`).  `ParseBmsValueError`, `BmsChannelIdError`, `ParseDifficultyError`
have built-in `IntoTokensError` impls.  Custom `FromStr` types can implement
`IntoTokensError` to choose the error variant.

## Dispatch

No build script. `BmsHeader` uses `#[derive(BmsTokenAttr)]` in dispatch
mode (no `#[bms_token]`, single-tuple variants). Variants with
`#[bms_fallback]` or `#[doc(hidden)]` are excluded from dispatch.

## Conversions (`From` / `TryFrom`)

All `BmsHeaderXXX` and `BmsMessage` get `From<T>` → parent, `TryFrom<Parent> → T`.
Chain via `?.try_into()?`. Error type: [`crate::BmsTryFromError`].

New sub-enum: add `From<NewEnum<'a, C>> for BmsHeader<'a, C>` +
`TryFrom<BmsHeader<'a, C>> for NewEnum<'a, C>` in its file.

## `#BASE`

Tokenized as a plain header (`BmsHeaderGameplay::Base(BmsBaseMode)`) — no
charset switching or case-folding.
