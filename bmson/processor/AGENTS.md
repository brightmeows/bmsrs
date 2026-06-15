# bmson-processor

Converts `bmson_def::Bmson` (v2 root schema) → `Chart<T>` via a
`BmsonMapping` layout.

## Pipeline

```text
bmson_def::Bmson → BmsonProcessor::process(bmson, layout) → Chart<L::NoteData>
```

For v0/v1 files, convert to the root schema first via `Bmson::from`.

## Sound-channel slicing

Each `SoundChannel` is pre-sliced into `AudioAsset`s at every unique note
pulse so the player looks up a pre-sized asset by index at runtime.
See the internal `slice` module.

## Non-obvious rules

- BGM notes (`x: 0`) are discarded when the same pulse has playable notes.
- Event ordering at the same tick: Note/BGA → BPM change → Stop.

## Built-in layouts

`Beat7k`, `Beat5k`, `Beat14k`, `Beat10k`, `Popn5k`, `Popn9k`.
`BmsonProcessor::process_default` uses `Beat7k`.

## Tests

```bash
cargo test -p bmson-processor
```
