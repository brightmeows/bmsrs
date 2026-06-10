# bms-tokenizer

First stage of `tokenizer → parser → processor`.

## Scope

The tokenizer is a syntactic pass. Every token exists solely for:
1. **Roundtrip fidelity** — `format_header` reproduces the original line.
2. **Syntax validation** — verifies well-formedness.

All semantic interpretation is the responsibility of downstream stages
(parser/processor).

## Zero-copy

All `&'a str` fields borrow from input. Don't `.to_owned()` unnecessarily.

## Usage

`BmsTokenizer` (builder). `tokenize<C>` returns per-line results.
Import from crate root, not submodules.

## New headers

Add `#[bms_token("...")]` variant to the appropriate domain enum.
Derive generates everything — no custom logic in `parse_header_line`.

Three-layer parse:

1. **Command match** (derive) — match command, extract `{id}`.
2. **Value parse** (derive per-field) — `FromStr`, `BmsValue::parse`, or
   `#[derive(BmsTokenAttr)]` for literal enums.
3. **Fallback** (`#[bms_fallback]`) — failure → `Ok(None)` → `BmsHeaderFallback`.
   Without it, failures are hard errors.

### Value types

Implement `BmsValue<'a>` (or `FromStr + Display` — blanket impl).
`ParseBmsValueError`, `BmsChannelIdError`, `ParseDifficultyError` have
built-in `IntoTokensError` impls. Custom `FromStr` types can implement
`IntoTokensError` to choose the error variant.

## Dispatch

No build script. `BmsHeader` uses `#[derive(BmsTokenAttr)]` in dispatch
mode (no `#[bms_token]`, single-tuple variants). Variants with
`#[bms_fallback]` are excluded from dispatch.

## Conversions (`From` / `TryFrom`)

All `BmsHeaderXXX` and `BmsMessage` get `From<T>` → parent, `TryFrom<Parent> → T`.
Chain via `?.try_into()?`. Error type: [`crate::BmsTryFromError`].

Add a new sub-enum: `From<NewEnum<'_>> for BmsHeader<'_>` + `TryFrom<BmsHeader<'_>> for NewEnum<'_>` in its file.

## `#BASE`

Tokenized as a plain header (`BmsHeaderGameplay::Base(BmsBaseMode)`) — no
charset switching or case-folding.
