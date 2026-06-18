# bmson-processor

Converts `bmson_def::Bmson` (v2 root schema) → `Chart<NoteData>`
via a `BmsonLayout` mode family (defined in `bmsrs-chart::layout`).

## Pipeline

```text
bmson_def::Bmson → BmsonProcessor::process(bmson, layout) → Chart<NoteData>
```

For v0/v1 files, convert to the root schema first via `Bmson::from`.

## Sound-channel slicing

Each `SoundChannel` is pre-sliced into `AudioAsset`s at every unique note
pulse so the player looks up a pre-sized asset by index at runtime.
See the internal `slice` module.

## Non-obvious rules

- BGM notes (`x: 0`) are discarded when the same pulse has playable notes.
- Event ordering at the same tick: Note/BGA → BPM change → Stop.

## Mode families

The layout argument selects a mode family from `bmsrs_chart::layout`:
`Bme` (beat-5k/7k/10k/14k, uniform), `Pms` (popn-9k/5k), `GenericLayout` (n-keys).
`BmsonProcessor::process_default` dispatches on `mode_hint`: `beat-*`/`dj-*`
→ `Bme`, `popn-*` → `Pms`, `generic-nkeys` → `GenericLayout`, else `Bme`.

## Tests

```bash
cargo test -p bmson-processor
```
