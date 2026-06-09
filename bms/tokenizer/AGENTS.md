# bms-tokenizer

First stage of `tokenizer → parser → processor`.

## Zero-copy

All `&'a str` fields borrow from input. Don't `.to_owned()` unnecessarily.

## Entry point

`BmsTokenizer` (builder pattern).  `tokenize<C: FromIterator<...>>` returns
`C<(NonZeroUsize, Result<BmsToken, BmsTokenizeError>)>` — one entry per
meaningful line, line numbers skip empty/comment lines.

## Public API

All types re-exported from `lib.rs`. Import from crate root, not submodules.

## New headers

Add a variant annotated with `#[bms_token("...")]` to the appropriate domain
enum (e.g., `BmsHeaderMetadata`).  The derive macro generates
`try_match_header` and `format_header` automatically.  All headers must use
this flow — no command-specific parse logic in `parse_header_line`.

Parsing proceeds in three layers:

1. **Command match** (derive): match command name, extract `{id}` index.
2. **Value parse** (derive calls per-field): `FromStr` for simple types,
   `BmsValue::parse` for complex structs.  Literal enums use
   `#[derive(BmsTokenAttr)]`; constrained types with fallback (e.g.
   `Rank`) hand-write `FromStr` + `Display`; multi-field values (e.g.
   `StpParams`, `BgaParams`) implement `BmsValue<'a>`.
3. **Fallback** (opt-in via `#[bms_fallback]`): parse failure yields
   `Ok(None)`, eventually reaching `BmsHeaderFallback`.  Without it,
   failures are hard errors.

`parse_header_line` does only prefix detection, command/value splitting,
and `%`-command filtering — no command-specific logic.

For commands with non-trivial value types, add `#[bms_fallback]` to the
variant — parse failures return `Ok(None)` (falling through to
`BmsHeaderFallback`) instead of hard errors.

## Value types

Implement `BmsValue<'a>` (or `FromStr + Display` — blanket impl covers it)
for custom value types used in header variants.  Literal enums use
`#[derive(BmsTokenAttr)]`; constrained numeric types with fallback
(e.g. `Rank`) hand-write `FromStr` + `Display`.

## Dispatch

No build script.  `BmsHeader` uses `#[derive(BmsTokenAttr)]` in dispatch
mode (auto-detected: no `#[bms_token]`, all single-tuple variants).  The
derive generates `BmsHeader::try_match_header` which calls each sub-enum's
`try_match_header` in declaration order.  Variants with `#[bms_fallback]`
(e.g., the `Fallback` catch-all) are excluded from dispatch.
