# bmsrs

## Commands

### Pre-commit (auto on commit)

```bash
pre-commit run --all-files --quiet    # manually trigger all hooks at once
```

Hooks configured: `cargo fmt --check`, `cargo clippy --workspace --quiet`, `cargo doc --workspace --no-deps --quiet`.

### CI / manual only

```bash
cargo test --workspace --quiet
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

## MSRV

- Minimum Rust version: **1.85**.

## Testing

- Test naming: `<scenario>_<expectation>`.
- One assertion per test. Prefer testing edge cases through public types
  over internal `Wrap` structs.
