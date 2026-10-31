# bmsrs-bmson-processor

## Role

Processor that converts `bmsrs_bmson_def::Bmson` (v2 root schema) into `Chart`. The
entry point is `BmsonProcessor` (a zero-sized type); all functionality is
exposed through `BmsonProcessor::process_default` / `process_nkeys`. Key mapping
is decoupled through the `BmsonLayout` mode family.

## Mode families

| Type | Implements | Use |
|------|------------|-----|
| Stateless ZST | `impl BmsonLayout` | simple modes, `BmsonProcessor::process::<L>(bmson)` |
| Stateful decoder | does not implement `BmsonLayout` | runtime configuration (key count), `BmsonProcessor::process_nkeys(bmson, keys)` |

`process_default` dispatches by `mode_hint`:

| mode_hint | Mode family |
|-----------|-------------|
| beat / dj | `Beat` |
| popn | `Pms` |
| generic | `GenericLayout` |

## Sound channel slicing

Each `SoundChannel` is pre-sliced at startup into an array of `AudioAsset`s,
with each slice bounded by a unique note tick. Slice times use
`TimingTrack::tick_to_musical_duration`; STOP pauses do not enter audio offsets.
Runtime note events look up the pre-sliced asset by index.

See the internal `slice` module.

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| BGM note dropping | When a playable note shares the same tick, the BGM note (`x: 0`) is dropped |
| Event ordering | Note/BGA, then BPM, then Stop |
| Layered notes | The same `(side, lane, tick)` across `SoundChannel`s merges into one `Note`, with audio layers collected in `NoteAudio`; mismatched long-note lengths produce a `LayeredNoteLengthMismatch` warning |
| Release notes | `up: true` maps to the core `NoteRole::Release` and no longer lives in `BmsonNoteExt` |
| Key count | `ChartInfo.key_count` is inferred from `mode_hint`; unknown or extension hints give `None` |
| Resource paths | `SoundChannel`/`mine_channels`/`key_channels`/BGA/image paths all go through `normalize_path`: absolute paths, drive prefixes, `..`, NUL, and empty paths are rejected; backslashes become forward slashes |
| Metadata | `subartists` and `title_image` map to `ChartInfo`; `ln_mode` was removed, and beatoraja `t` overrides `ln_type_hint` |
