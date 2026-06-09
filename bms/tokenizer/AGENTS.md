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
`try_match_header` and `format_header` automatically.

For commands with non-trivial value types, add `#[bms_fallback]` to the
variant — parse failures return `Ok(None)` (falling through to
`BmsHeaderFallback`) instead of hard errors.

Only hand-interpret commands whose value format is fundamentally
non-templateable (e.g., `#STP`'s `xxx[.yyy] zzzz`).

## Value types

Implement `BmsValue<'a>` (or `FromStr + Display` — blanket impl covers it)
for custom value types used in header variants.  Simple domain enums can use
`#[derive(BmsTokenAttr)]` with `#[bms_token("literal")]` on each variant.

## Dispatch

No build script.  `BmsHeader` uses `#[derive(BmsTokenAttr)]` in dispatch
mode (auto-detected: no `#[bms_token]`, all single-tuple variants).  The
derive generates `BmsHeader::try_match_header` which calls each sub-enum's
`try_match_header` in declaration order.  Variants with `#[bms_fallback]`
(e.g., the `Fallback` catch-all) are excluded from dispatch.
