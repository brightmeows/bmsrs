# bmsrs

## Commands

```bash
cargo clippy --workspace --quiet
cargo test --workspace --quiet
cargo doc --workspace --quiet --no-deps
cargo fmt
cargo deny check
```

## Project Structure

```text
bmson/def/          # bmson format type definitions (v0/v1/v2) — see bmson/def/AGENTS.md
bmson/de-chumsky/   # bmson deserializer (chumsky parser)
bmson/processor/    # bmson processing pipeline
bms/tokenizer/      # BMS tokenizer
bms/parser/         # BMS parser
bms/processor/      # BMS processing pipeline
core/chart/         # Shared chart model
player/             # Music game player
```

## Testing

- **Integration tests** in `tests/` (public API).
  **Unit tests** in `src/` (private API).
- Test naming: `<scenario>_<expectation>`.
- One assertion per test. Prefer testing edge cases through public types
  over internal `Wrap` structs.

## Code Style

- Edition 2024.
- `missing_docs = "deny"` — all public items must have doc comments.
- `clippy::pedantic` with `allow_attributes_without_reason = "deny"`.
- `unwrap_used = "deny"`, `indexing_slicing = "deny"`.
