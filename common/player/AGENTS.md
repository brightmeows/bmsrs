# bmsrs-player

## Role

Pure simulation layer for `Chart<T>`. Derives timing, scroll caches, and a
stable note index from the chart event stream, then exposes time, event, note,
and viewport queries.

**No I/O, rendering, judging, or scoring.** Judgment policy belongs to the
caller.

## Time and data model

`ChartData` exposes `events`, `init_bpm`, and `resolution` through getters.
`Player::new` derives `TimingTrack` from `EventKind::Bpm` and `EventKind::Stop`;
the event stream is the only timing source.

| Method | Purpose |
|--------|---------|
| `Player::new(chart)` | derives timing/caches/NoteTable from an already validated `Chart`; infallible |
| `advance(Duration)` | pure playback-time accumulation (O(1)); ticks derived on demand |
| `seek(Duration)` | jump to an absolute position |
| `current_time() -> Duration` | current playback position (the only mutable state) |
| `current_tick() -> u64` | rounded tick position (O(log n)) |
| `current_tick_f64() -> f64` | continuous tick position for smooth scroll |
| `duration_to_tick_f64(Duration)` | continuous time-to-tick conversion |

During a STOP, `current_time()` keeps flowing while ticks freeze. Use
`current_tick_f64()` for rendering positions; convert a tick through
`tick_to_duration` when "before the pause" semantics are needed.

## Queries

| Query type | Purpose |
|------------|---------|
| Events | `events_in_range` (Bound-correct), `events_in_time_range([start, end))`, `next_event_after` |
| Notes | `notes_in_range` returns `NoteRef`/`NotePoint`; `long_notes()`; `note(id)` |
| Audio | `audio_assets()` plus time-range event queries |
| BGA | `bga_resources()` plus time-range event queries |
| Scroll/SPEED | `scroll_rate_at`, `scroll_position_at`, `scroll_position_at_f64`, `scroll_delta`, `spacing_at_time` |
| Visibility | `Viewport::visible_notes_at`, `visible_long_notes_at`, `visible_lines_at`; delta-range variants `visible_notes_in` / `visible_long_notes_in` / `visible_lines_in` plus `max_scroll_delta_at` |

`NoteTable` assigns stable `NoteId`s in event order and groups note ids by
`(NoteSide, Lane)`. It does not store a `judgeable` flag.

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| events are the only timing source | `TimingTrack::from_events` derives BPM changes and stops; do not add a second timing field |
| `NotePoint` is descriptive | `Normal + Release` is a release keysound/BSS marker; `Long + Release` is an MSS endpoint. The caller decides with its Ln/Cn/Hcn policy. There is no `notes_for_judgement` |
| Viewport is scroll-domain | Never convert a time window to a tick range for visibility. During a STOP, future notes stay frozen on screen; Viewport uses `scroll_position_at_f64` and `spacing_at_time` |
| Viewport delta ranges | The `visible_*_in` queries take a scroll-delta range (`impl RangeBounds<f64>`) relative to the judgment line; `..` yields everything. During reverse-scroll sections future notes sit below the line (negative delta), so the default `visible_*_at` window misses them by design — pass a range with a negative lower bound. `progress` stays `delta / max_scroll_delta_at` (signed; judgment line 0.0, lane top 1.0) regardless of the range |
| SPEED is time-interpolated | Keyframes store wall-clock time; before the first keyframe, interpolate from 1.0 at time zero |
| `events_in_time_range` is exact | It maps to a coarse tick range, then filters by `tick_to_duration(event.tick())` in `[start, end)` |
| `events_in_range` handles `u64::MAX` | Bound matching uses `partition_point` directly, not `end_tick = u64::MAX` |
| `Player::new` consumes the chart | `into_chart()` returns the validated chart |

## Performance benchmarks

The hot-path benchmarks (`benches/player.rs`) quantify per-frame timing,
event, note, and viewport costs.

```bash
cargo bench -p bmsrs-player --bench player -- --quick  # quick estimate
```

> **Performance-related changes must be verified with benchmarks**; do not
> conclude from big-O reasoning. A large constant factor in O(log^2 n) can be
> slower than O(n) on typical charts, and only measurement can tell the
> difference.

Precomputed per-event activation times (the bmz-player import-time model) are
deliberately not used: a measured comparison puts time-range event queries at
about 0.14 us/frame on demand versus 0.08 us/frame with a precomputed table
(a 60 ns difference against a 16.7 ms frame budget), with a 174 us one-time
build for 5,120 events. `TimingTrack` conversion stays on demand.
