# bmsrs-bms-values-derive

## Role

`#[derive(BmsTokenAttr)]` proc-macro crate. Macro behavior (three modes,
attributes, `String` field detection, error mapping) is documented in the
crate-level docs of `src/lib.rs`.

Pure compile-time logic, no runtime dependencies.

## Tests

Tested indirectly through `bmsrs-bms-values` roundtrips; proc-macro unit tests
live in `parse.rs` using `proc_macro2` types.
