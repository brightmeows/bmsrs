# bms-tokenizer

First stage of `tokenizer → parser → processor`.

## Zero-copy

All `&'a str` fields borrow from input. Don't `.to_owned()` unnecessarily.

## Public API

All types re-exported from `lib.rs`. Import from crate root, not submodules.

## New headers

Add a variant annotated with `#[bms_token("...")]` to the appropriate domain
enum (e.g., `BmsHeaderMetadata`).  The derive macro generates
`try_match_header` and `__bms_dispatch` automatically.  Only use
`BmsHeaderExt` for genuinely unrecognised commands.

## Dispatch

No build script.  Dispatch in `header.rs` calls each sub-enum's
`__bms_dispatch` via `macro_rules!` — 7 calls, effectively O(1).
