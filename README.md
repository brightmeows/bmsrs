# bmsrs

English | [简体中文](README.zh-CN.md)

A Rust toolkit for parsing, converting, and simulating playback of BMS and
BMSON chart formats.

> Pre-release: `0.1.0` has not been released; public interfaces may change.

## Overview

bmsrs handles two chart formats:

- BMS: a text format with a complete pipeline from raw bytes to a structured
  model.
- BMSON: a JSON format with v0, v1, and v2 type definitions and upgrades.

Both pipelines output the unified `Chart<T>` central IR. Renderers, players, and
tools only need to consume this layer.

The project uses Rust 2024 edition with MSRV 1.95.

## Pipeline

| Input | Processing | Output |
|-------|------------|--------|
| BMS bytes | `bmsrs-bms-source`, `bmsrs-bms-syntax`, `bmsrs-bms-parser` | `Bms` |
| `Bms` | `bmsrs-bms-processor` | `Chart` |
| BMSON JSON | `bmsrs-bmson-def`, `bmsrs-bmson-processor` | `Chart` |
| `Chart` | `bmsrs-player` | timeline queries |

`bmsrs-bms-analysis` consumes the CST directly, providing diagnostics, a symbol index,
and editor data. `bmsrs-bms-writer` rewrites `Bms` canonically as BMS text.

## Crates

| Crate | Responsibility |
|-------|----------------|
| `bmsrs-bms-source` | BMS source-text layer: bytes, encoding, line index, byte-faithful editing |
| `bmsrs-bms-syntax` | BMS syntax layer: rowan CST, lexing, control-flow queries |
| `bmsrs-bms-values` | BMS single-line syntax and typed values |
| `bmsrs-bms-parser` | BMS semantic layer: CST to `Bms` |
| `bmsrs-bms-processor` | `Bms` to `Chart` |
| `bmsrs-bms-writer` | canonical rewrite of `Bms` as BMS text |
| `bmsrs-bms-analysis` | diagnostics, symbol index, and editor data |
| `bmsrs-bmson-def` | BMSON v0, v1, and v2 type definitions |
| `bmsrs-bmson-processor` | `Bmson` to `Chart` |
| `bmsrs-chart` | format-agnostic `Chart<T>` central IR |
| `bmsrs-player` | pure simulation layer for `Chart` |
| `bmsrs` | public re-export facade |

## Quick start

`0.1.0` has not been released. Use a git dependency for now:

```toml
[dependencies]
bmsrs = { git = "https://github.com/brightmeows/bmsrs" }
```

### BMS to Player

```rust
use std::time::Duration;

use bmsrs::bms::processor::BmsProcessor;
use bmsrs::bms::{ErrorStrategy, from_bytes};
use bmsrs::player::Player;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("chart.bms")?;
    let (bms, _warnings) = from_bytes(bytes, |_| 1, ErrorStrategy::CollectAll)?;
    let processed = BmsProcessor::process_default(&bms)?;

    let mut player = Player::new(processed.chart)?;
    player.advance(Duration::from_secs(1));

    println!("current tick: {}", player.current_tick());
    Ok(())
}
```

The second argument of `from_bytes` is the branch-selection function for
`#RANDOM` and `#SWITCH`. The example always picks the first branch; use
`from_text` when the input is already UTF-8.

### BMSON to Chart

```rust
use bmsrs::bmson::def::Bmson;
use bmsrs::bmson::processor::BmsonProcessor;

fn load(json: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bmson: Bmson<'_> = serde_json::from_str(json)?;
    let processed = BmsonProcessor::process_default(&bmson)?;

    println!("events: {}", processed.chart.data.events.len());
    Ok(())
}
```

Add `serde_json` to your project. v0 and v1 files are upgraded to v2 through
`Bmson::from` first.

## Development

Common checks:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --quiet
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Run every hook before committing:

```bash
pre-commit run --all-files
```

Player hot-path benchmarks:

```bash
cargo bench -p bmsrs-player
```

CI also runs `cargo deny check` and an MSRV 1.95 check.

Crate responsibilities and development conventions are documented in
`AGENTS.md`. Subdirectory rules live in `bms/AGENTS.md`, `bmson/AGENTS.md`, and
`common/AGENTS.md`.

## License

Apache-2.0, see [`LICENSE`](LICENSE).
