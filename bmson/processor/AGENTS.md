# bmson-processor

Converts `bmson_def::Bmson` (v2 root schema) → `Chart<NoteData>`
via a [`BmsonLayout`] mode family (defined in this crate's [`layout`] module).

## Pipeline

```text
bmson_def::Bmson → BmsonProcessor::process::<L>(bmson) → Chart<NoteData>
```

where `L: BmsonLayout` is a stateless mode family ([`Beat`] or [`Pms`]).
For stateful n-keys, use `process_nkeys(bmson, keys)`.

For v0/v1 files, convert to the root schema first via `Bmson::from`.

## Sound-channel slicing

Each `SoundChannel` is pre-sliced into `AudioAsset`s at every unique note
pulse so the player looks up a pre-sized asset by index at runtime.
See the internal `slice` module.

## Non-obvious rules

- BGM notes (`x: 0`) are discarded when the same pulse has playable notes.
- Event ordering at the same tick: Note/BGA → BPM change → Stop.

## Mode families

Two kinds of families:

1. **Stateless ZST** (implements `BmsonLayout`) — a `match` on `x` that
   produces `Option<NoteData>`. Used via `process::<T>(bmson)`.
2. **Stateful decoder** (does NOT implement `BmsonLayout`) — carries
   runtime config (key count). Used via `process_nkeys(bmson, keys)`.

`process_default` dispatches on `mode_hint`: beat/dj → `Beat`, popn → `Pms`,
generic → `GenericLayout`. See the `layout` module for the current set of
families and their x-to-lane tables.

## Tests

```bash
cargo test -p bmson-processor
```
