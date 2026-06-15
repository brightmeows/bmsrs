# bmsrs-player

Pure simulation layer for `Chart<T>`. Provides time-based queries for
gameplay rendering and scheduling. No I/O, rendering, judgement, or scoring.

## Time API

All public time parameters use `Duration`:

- `advance(Duration)` — advance playback cursor.
- `seek(Duration)` — jump to arbitrary position.
- `current_time() -> Duration` — playback position.

## TimingCache

Pre-computes BPM segments and stop-duration cumulative sums for O(log n)
binary-search lookup. `tick_to_duration` / `duration_to_tick` on `Player`
delegate to the cache.

`TimingCache` and `TimingTrack` (from `bmsrs-chart`) produce identical
results — verified by tests. The cache is an O(log n) optimisation over
`TimingTrack`'s O(n) linear scan. They are intentionally separate: do not
merge them.

## Queries

- `active_notes` / `upcoming_notes` — note visibility by tick window.
- `active_bga` / `bga_events_in` — BGA layer state.
- `current_bpm` / `scroll_rate_at` — timing/scroll at current position.

## Tests

```bash
cargo test -p bmsrs-player
```
