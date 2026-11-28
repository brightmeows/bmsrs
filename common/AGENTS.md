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

Three crates: `bmsrs-chart` (the `Chart<T>` central IR), `bmsrs-player`
(pure simulation queries), and `bmsrs-pipeline` (`Outcome<T, W>`,
`SourceSpan`, `Diagnostic`). The time model is documented in
`bmsrs-chart`'s crate-level docs.

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
| The two `Layout` traits (`raw::bms::processor::layout::Layout` and `raw::bmson::processor::layout::Layout`) are not unified into one | their input dimensions differ fundamentally: BMS takes a validated `(player, lane)` pair, BMSON takes one-dimensional `x`; the formats evolve independently, and unifying only adds a generic indirection layer |
| The two `ProcessError` types are defined separately | they share only `TimingTrackError` (forwarded through `#[from]`); BMSON also has `InvalidKeyCount`, and merging would drag format-specific errors into a shared type |
| `TimingTrack` forward and inverse segments accumulate time separately | two implementations of the same mathematical transform (`new` and `merge_inv`); if a third kind of timing distortion appears (such as gradual BPM), reassess before writing a third copy |

