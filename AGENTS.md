# bmsrs

## Commands

```bash
cargo clippy --workspace --quiet
cargo test --workspace --quiet
cargo doc --workspace --quiet --no-deps
cargo fmt
cargo deny check
```

## Crates

| Crate | Directory | Summary |
|---|---|---|
| `bmsrs` | `./src` | Re-export facade |
| `bms-tokenizer` | `bms/tokenizer` | First stage of `tokenizer → parser → processor` |
| `bms-tokenizer-derive` | `bms/tokenizer-derive` | Proc-macro for `#[derive(BmsTokenAttr)]` |
| `bms-parser` | `bms/parser` | |
| `bms-processor` | `bms/processor` | |
| `bmson-def` | `bmson/def` | bmson format type definitions (v0/v1/v2) |
| `bmson-de-chumsky` | `bmson/de-chumsky` | |
| `bmson-processor` | `bmson/processor` | |
| `bmsrs-chart` | `core/chart` | |
| `bmsrs-player` | `player` | |

## Commit format

Conventional Commits matching `release-plz.toml` changelog groups:
`feat:` / `fix:` / `refactor:` / `perf:` / `test:` / `docs:` / `ci:` / `security:` / `deprecated:` / `revert:`

- Title/body in English.
- Use `()` for scope, e.g. `feat(bms-parser):`.
- Use `!` for BREAKING CHANGE, e.g. `feat!:` or `feat(scope)!:`.

## Comment style

- Use doc comments (`///` for items, `//!` for modules) for all API
  documentation — clippy enforces docs on all items.
- **No** section header comments (`// Foo`, `// -- Foo --`, `// Foo ----`).
  Doc comments on the item itself are sufficient.
- **No** decorative divider lines (`// ----`, `// ====`, `// //`, etc.).
  Use a blank line instead.

## MSRV

- Minimum Rust version: **1.85**.

## Clippy

- `unwrap_used`, `expect_used`, `indexing_slicing` are **deny**.
  Use `?` operator or `.get()` instead. Use `#[expect(…)]` with a `reason` only
  when no alternative is practical (e.g., pre-validated input).
- `pedantic` group is **deny**.

## Testing

- Test naming: `<scenario>_<expectation>`.
- One assertion per test. Prefer testing edge cases through public types
  over internal `Wrap` structs.
