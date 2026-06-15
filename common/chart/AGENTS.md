# bmsrs-chart

Format-agnostic chart data model — the central IR between format
processors and the player.

## Time model

Two domains, converted only at API boundaries:

- **Tick domain** (`u64`): all event positions, stop durations, LN durations.
- **Time domain** (`Duration`): `AudioAsset.start`, `Chart::duration()`.

[`TimingTrack`] bridges the two via `tick_to_duration` / `duration_to_tick`.
Internal computation uses `f64`; conversion happens via
`Duration::from_secs_f64` / `as_secs_f64`.

`resolution` defines ticks per quarter note (default 240).

## Generic note data

`Chart<T>` is parameterised by a `NoteData` type `T`. The default is
`DefaultNoteData` (lane + kind only). Custom types carry format-specific
extensions (volume, pan, LN mode, etc.).

## Zero external dependencies

This crate is pure data model — no `serde`, no `thiserror`. Only `std`.

## Tests

```bash
cargo test -p bmsrs-chart
```
