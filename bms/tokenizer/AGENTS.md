# bms-tokenizer

First stage of `tokenizer → parser → processor`.

## Zero-copy

All `&'a str` fields borrow from input. Don't `.to_owned()` unnecessarily.

## Public API

All types re-exported from `lib.rs`. Import from crate root, not submodules.

## New headers

Add variant to matching domain enum (`BmsHeaderMetadata`, `BmsHeaderGameplay`,
…). Only use `BmsHeaderExt` for genuinely unrecognised commands.
