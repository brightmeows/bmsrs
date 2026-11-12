# common/: format-agnostic shared layer

## Role

Format-agnostic chart data model and simulation layer. The BMS and BMSON
pipelines converge here; both processors output `Chart<T>`.

**Dependency direction.** The graph flows from the format domains into
common: regular dependencies of a common crate may only be `std`, sibling
common crates (`bmsrs-player` depends on `bmsrs-chart`), and external crates
judged by normal review. Domain crates (`bmsrs-bms-*`, `bmsrs-bmson-*`) must
not appear in regular dependencies. Dev-dependencies are exempt: they never
enter downstream dependency trees.

## Architecture

```mermaid
flowchart LR
    BProc[bmsrs-bms-processor] --> Chart["bmsrs-chart (Chart&lt;T&gt;)"]
    BMProc[bmsrs-bmson-processor] --> Chart
    Chart --> Player[bmsrs-player]
```

| Crate | Responsibility |
|-------|----------------|
| `bmsrs-chart` | `Chart<T>` central IR: format-agnostic model of events, timing, and notes |
| `bmsrs-player` | pure simulation layer for `Chart<T>`: time-based query interface |
| `bmsrs-pipeline` | shared pipeline result and diagnostic types: `Outcome<T, W>`, `SourceSpan`, `Diagnostic` |

## Time model

| Domain | Type | Use |
|--------|------|-----|
| Tick | `u64` | all event positions, LN durations |
| Time | `Duration` | `AudioAsset.start`, `Player::duration()`, player public API |

`ChartData` stores the scalar timing configuration (`init_bpm`, `resolution`)
and the event stream. `bmsrs-player` derives `TimingTrack` from
`EventKind::Bpm`/`EventKind::Stop`; the event stream is the only timing source.
`BpmLookup` serves lightweight BPM queries before constructing a `TimingTrack`.
Both live in `bmsrs-chart`.

## Design principles

| Principle | Meaning |
|-----------|---------|
| Dependency direction | the graph flows from format domains into common; domain crates never appear in common regular dependencies, external dependencies follow normal review |
| No side effects | no I/O, rendering, judging, or scoring; pure data and queries |
| Do not store mode information | `Chart` does not store a mode family; notes already carry `(NoteSide, Lane)` |

## Cross-format exemptions

These are deliberate non-unifications. The reassessment point is the arrival of
the corresponding use case from a third format; do not merge them casually
before then.

| Decision | Reason |
|----------|--------|
| `BmsLayout` and `BmsonLayout` are not unified into one trait | their input dimensions differ fundamentally: BMS takes a validated `(player, lane)` pair, BMSON takes one-dimensional `x`; the formats evolve independently, and unifying only adds a generic indirection layer |
| The two `ProcessError` types are defined separately | they share only `TimingTrackError` (forwarded through `#[from]`); BMSON also has `InvalidKeyCount`, and merging would drag format-specific errors into a shared type |
| `TimingTrack` forward and inverse segments accumulate time separately | two implementations of the same mathematical transform (`new` and `merge_inv`); if a third kind of timing distortion appears (such as gradual BPM), reassess before writing a third copy |

## Tests

```bash
cargo test -p bmsrs-chart
cargo test -p bmsrs-player
cargo bench -p bmsrs-player          # hot-path performance benchmarks
```
