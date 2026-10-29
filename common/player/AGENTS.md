# bmsrs-player

## Role

Pure simulation layer for `Chart<T>`. Provides time-based query APIs for game
rendering and scheduling.

**No I/O, rendering, judging, or scoring.**

## Time API

Every public time parameter uses `Duration`:

| Method | Purpose |
|--------|---------|
| `Player::new(chart)` | sorts events, then validates `ChartData`; returns `Result` |
| `advance(Duration)` | pure playback-time accumulation (O(1)); ticks derived on demand |
| `seek(Duration)` | jump to a given position |
| `current_time() -> Duration` | current playback position (the state itself) |
| `current_tick() -> u64` | current tick position (derived from time, O(log n)) |

## Queries

| Query type | Purpose |
|------------|---------|
| Notes | `events_in_range` plus caller-side kind filtering; `notes_for_judgement` applies the judgment-relevant filter (Normal/Long) |
| BGM/audio | audio triggers on the timeline |
| BGA (three layers) | Base / Layer / Layer2 animation events |
| Visual effects | bar lines, scroll/speed changes |

See the `Player` impl block.

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| Duration is the only time interface | ticks are not exposed externally; conversion happens inside |
| The player does not judge | judging logic belongs to the upper layer (renderer/UI) |
| Time-driven state machine | `current_time` is the only state field; `current_tick()` is derived every time. Many instants inside a stop map to the same tick, so `tick` to `time` is not injective; using ticks as state would make small per-frame `advance` steps get stuck inside a stop forever |
| Semantics during STOP | `current_tick()` freezes; `current_time()` keeps flowing. For "before the pause" semantics, convert `current_tick` through `tick_to_duration` |

## Performance benchmarks

The hot-path benchmarks (`benches/player.rs`) quantify the per-frame cost of the
playback loop, providing data for optimizing timing queries and query paths.

```bash
cargo bench -p bmsrs-player --bench player -- --quick  # quick estimate
```

Coverage: `duration_to_tick`, `tick_to_duration`, `advance`, and
`events_in_range`.

> **Performance-related changes must be verified with benchmarks**; do not
> conclude from big-O reasoning. A large constant factor in O(log^2 n) can be
> slower than O(n) on typical charts, and only measurement can tell the
> difference.
