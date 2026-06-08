# bms-tokenizer-derive

Proc-macro crate for `#[derive(BmsTokenAttr)]` — generates `try_match_header`,
`format_header`, and `__bms_dispatch` on header sub-enums.

## Module layout

- `lib.rs` — derive macro entry point, collects `#[bms_token]` per variant
- `parse.rs` — template string parser (`"#BPM{id} {value}"` → `BmsTokenTemplate`)
- `codegen.rs` — generates `try_match_header` / `format_header` / `__bms_dispatch`

## Generated functions

- `try_match_header` — per-sub-enum parsing (used in roundtrip tests)
- `__bms_dispatch` — thin wrapper returning `Option<BmsHeader<'_>>`,
  called by the 7-line dispatch in `header.rs` (no build script needed)
- `format_header` — formats an enum variant back to `(command, value)`

## Testing

Tested indirectly via `bms-tokenizer`'s roundtrip tests (`bms_token.rs`).
Proc-macro unit tests live in `parse.rs` (use `proc_macro2` types).
