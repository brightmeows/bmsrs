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

During a STOP, `current_time()` keeps flowing while ticks freeze. Use
`current_tick_f64()` for rendering positions; convert a tick through
`tick_to_duration` when "before the pause" semantics are needed.

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
