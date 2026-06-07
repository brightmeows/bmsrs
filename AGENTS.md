# bmsrs

## Commands

```bash
cargo clippy --workspace --quiet
cargo test --workspace --quiet
cargo doc --workspace --quiet --no-deps
cargo fmt
cargo deny check
```

## Testing

- Test naming: `<scenario>_<expectation>`.
- One assertion per test. Prefer testing edge cases through public types
  over internal `Wrap` structs.
