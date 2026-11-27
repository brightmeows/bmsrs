## `KaleidBeat` sidecar

`Processed` carries the chart plus `Option<BmsonKaleid>`. The sidecar holds the extension metadata, notes, conditional blocks, and filter objects; `speed_events` also map to core Speed events. Ordinary files leave the sidecar as `None`.
# bmsrs-bmson-processor

## Role

Processor that converts `bmsrs_bmson_model::Bmson` (v2 root schema) into `Chart`. The
entry point is `Processor` (a zero-sized type); all functionality is
exposed through `Processor::process_with` / `process_default`. Key mapping
is decoupled through the `Layout` mode family.

## Mode families

| Type | Implements | Use |
|------|------------|-----|
| Stateless ZST | `impl Layout` | fixed modes, `Processor::process_with(bmson, Beat)` |
| Stateful `GenericLayout` | `impl Layout` through the same `&self` receiver | `generic-nkeys`, `process_with(bmson, GenericLayout::new(keys)?)`; `keys` is a `NonZeroU8`, so zero keys is not representable |

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
| Same-pulse STOP accumulation | Stop durations at the same `y` are summed before `TimingTrack` construction, per the bmson specification (beatoraja keeps the last instead); the folded stops feed both the timing track and the event stream |
| Event ordering | Note/BGA, then BPM, then Stop |
| Layered notes | The same `(side, lane, tick)` across `SoundChannel`s merges into one `Note`, with audio layers collected in `NoteAudio`; mismatched long-note lengths produce a `LayeredNoteLengthMismatch` warning |
| Release notes | `up: true` maps to the core `NoteRole::Release` and no longer lives in `BmsonNoteExt` |
| Key count | `ChartInfo.key_count` is inferred from `mode_hint`; unknown or extension hints give `None` |
| Resource paths | `SoundChannel`/`mine_channels`/`key_channels`/BGA/image paths all go through `normalize_path`: absolute paths, drive prefixes, `..`, NUL, and empty paths are rejected; backslashes become forward slashes |
| Metadata | `subartists`, `title_image`, and `judge_rank` (as `JudgeRankSpec` with the `BmsonJudgeRank` kind) map to `ChartInfo`; `ln_mode` was removed, beatoraja `t` overrides `ln_type_hint`, and the chart-level hint carries `None` through when the source omits the field |
| Resolution fallback | `resolution: 0` falls back to 240 with `ZeroResolutionFallback`; negative JSON values take their absolute value at deserialization |

## Known limitations

These are boundaries intentionally left open. Check them before assuming a
feature is missing by accident.

| Item | Status | Reassess when |
|------|--------|---------------|
| KaleidBeat free zones | `free_zone_channels` is typed into `BmsonKaleid` and exposed through `Processed`; free-zone judgment semantics stay in the player | a player needs a shared free-zone evaluation model |
| BMSON MSS | `Long + Release` is exposed as a descriptive note point; there is no dedicated MSS judgment state | a player implements MSS scoring or input |
| Layer-event `condition` / `interval` / `id_set` | clip selection conditions are not modeled; `bga_sequence` clips and clip references (poor/layer) are modeled, so poor animations still play the referenced clip | an imported chart needs condition-driven clip selection |

## Cross-engine divergences

| Case | bmsrs default | Other engine | Where |
|------|---------------|--------------|-------|
| `info.total` | `ChartInfo.total_percent` keeps the raw percentage and `ChartInfo.total` stays `None` | bmz-player converts it to a concrete gauge total against beatoraja's note-count formula | `total_percent` |
