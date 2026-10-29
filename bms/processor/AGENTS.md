# bmsrs-bms-processor

## Role

Processor that converts `Bms` into a format-agnostic `Chart`. BMS player and
key mapping are decoupled through the `BmsLayout` mode family.

The processor performs **lossy conversions**: merge priority (`#BGA`/`#BMP`),
reference resolution (WAV/BPM), and table lookup (def tables) all live here. See
`bms/AGENTS.md` for domain-level criteria.

`Processed` carries two sidecars in addition to `chart` and `warnings`:

| Field | Content |
|-------|---------|
| `gameplay` | raw values of `#PLAYER`/`#RANK`/`#DEFEXRANK`/`#TOTAL`/`#VOLWAV`/`#EXRANKxx`/`#OPTION`/`#CHANGEOPTIONxx` |
| `audio` | raw values of `#EXWAV` parameters, `#WAVCMD`, `#CDDA`, `#MIDIFILE`, `#PATH_WAV` |

`#PATH_WAV` is applied to `AudioAsset.path` as well; the sidecar keeps the raw
prefix.

## Key conversions

| Conversion | Input | Output | Notes |
|------------|-------|--------|-------|
| Notes | `NoteEvent` + `LongNoteEvent` + `MineEvent` | `Event::Note` | notes consumed by LNOBJ are excluded from ordinary notes |
| Timing | `BpmChange` + `StopEvent` + `MeasureTable` | `TimingTrack` | measure length is computed from `length_ratio` (f64) |
| BGM | `BgmEvent` | `Event::Bgm` | LNOBJ end markers also play as BGM |
| BGA | `BgaEvent` | `Event::Bga` | four layers (Base / Poor / Layer / Layer2); `BmsBgaLayer` maps to `chart::BgaLayer` here |
| Display metadata | `BmsHeaderDisplay` | `ChartInfo` | `#BANNER`/`#BACKBMP`/`#STAGEFILE`/`#PREVIEW` are handled by `build_metadata` |
| SCROLL | `ScrollEvent` | `Event::Scroll` | scroll speed multiplier |
| SPEED | `SpeedEvent` | `Event::Speed` | visual spacing keyframes with linear interpolation |
| Long-note mode hint | `#LNMODE` | `LnTypeHint` | 1:1 mapping of Ln/Cn/Hcn (`ln_type_hint` method) |
| BGA crop | `#BGA`/`#@BGA` | `BgaResource.crop` | `build_bmp_map` merges the three namespaces; `#@BGA` (w/h) is normalized to the bottom-right corner |
| Video asset | `#VIDEOFILE`/`#MOVIE` | `ChartInfo.video` | looping `#VIDEOFILE` takes priority over single-play `#MOVIE`; attaches `#VIDEOf/s`/`#VIDEOCOLORS`/`#VIDEODLY` |
| Video SEEK | `#SEEK` definition plus ch 05 | `BmsCustomEvent::VideoSeek` | looks up `seek_defs` for milliseconds (not the raw Base36 value); undefined ids are skipped |
| `non_event_data` custom events | `NonEventData.values` | `BmsCustomEvent` | every 2-character index is already normalized in the parser layer, so lookups hit directly without another `normalize(base)` |
| Value parse warnings | `Messages.warnings` | `ProcessWarning::UnparsableValue` | forwards the parser layer's `ChannelValueWarning` (measure/channel/value); missing references are covered by `Missing*` variants |

## Long-note modes (auto-detection)

| Mode | Detection | Pairing algorithm |
|------|-----------|-------------------|
| LNOBJ | `#LNOBJ` is defined | ordinary notes paired with LNOBJ markers |
| LNTYPE 1 (RDM) | default (or `#LNTYPE 1`) | ch 51-69, filter `"00"`, pair consecutively |
| LNTYPE 2 (MGQ) | `#LNTYPE 2` | ch 51-69, `"00"` means LN end |

## Position conversion

`MeasureTable` precomputes the cumulative tick offset of each measure to handle
variable time signatures:

```text
tick = measure_starts[measure] + numer * measure_len / denom
```

Each measure length is `resolution * 4 * length_ratio`, where `length_ratio` is
the `#xxx02` value.

## Stop conversion

| Source | Conversion formula |
|--------|--------------------|
| `#STOP` (ch 09) | `raw / 192.0 * resolution * 4` ticks |
| `#STP` (header) | milliseconds to ticks through `bpm_at_tick` |

## Mode families

Mode families are zero-sized types implementing the `BmsLayout` trait. Each
family is a mapping table from `(player, lane)` to `Option<NoteData>`.

Standard families are in the public `layout` module. `Bme` covers
5K/7K/10K/14K; `#PLAYER` does not affect the mapping (consistent with mainstream
engines).

## Public modules

| Module | Content |
|--------|---------|
| `layout` | `BmsLayout` trait plus standard mode families (`Bme` and others) |
| `custom_event` | `BmsCustomEvent` enum: BMS-specific events (BGA opacity, text, options, and so on) used as the `CustomEvent` type parameter of `Chart` |

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| Default BPM 130 | Matches the BMS specification; not `0.0` |
| Event ordering | Bar(0), then Note/BGA/BGM(1), then BPM(2), then Stop(3), then Scroll(4), then Speed(5), then Custom(6) |
| LNOBJ end BGM | The WAV defined by an end marker plays when the marker crosses the judgment line |
| ch51-69 under LNOBJ | Mutually exclusive with LNOBJ (memo/10 leaves it undefined); not discarded, kept as ordinary visible notes |
| Mine `damage` | `MineEvent.raw_value` (`Option<u16>`) goes through `mine_damage()`: `Some(1295)` gives INFINITY, `Some(n)` gives n/2, `None` gives 1.0 |
| `process_default` layout | Uses `DscOctFp` when `#OCT/FP` is true, otherwise `Bme`; `#PLAYER` is still ignored, and modes such as PMS must be specified explicitly |
| `key_count` | Sum of the maximum `Lane::Key` number per side across the event stream; `None` when there are no key notes |
| Judge/life multipliers | BMS `#RANK`/`#TOTAL` are not converted into `ChartData`; both fields stay `None` |
| `#PATH_WAV` | Joined when constructing `AudioAsset.path`; the raw prefix stays in `Processed.audio.path_wav` |
| `#BPM 0` | Returns `ProcessError::ZeroBpm` instead of falling back to 130; the current time model cannot represent zero BPM |
| Conversion steps belong to `BmsConverter` | `collect_*`/`build_*` are methods on the private internal `BmsConverter`, not free functions |
| `non_event_data` index normalization | Done in the parser layer; consumed by the processor | Every 2-character value in `non_event_data` was normalized in the parser's `finalize_merged` using the pre-scanned `#BASE`. Processor lookups hit directly. The `BmsConverter.base` field was removed because it is no longer needed |
| `bpm_at_tick` source | Delegates to `BpmLookup::new(init_bpm, changes).bpm_at_tick(tick)` (the former local duplicate is gone). STP-to-tick conversion still has to happen before `TimingTrack` construction, but the algorithm is unified |
| BGA crop source path | The `bmp_index` of `#BGA`/`#@BGA` is a decimal source number; it is matched against `bmp_files` keys by Base36 numeric value (for example, `"01"` gives 1). Crop ids whose source path cannot be resolved are skipped |

## Deliberate deferral (parsed but not converted here)

The following fields are parsed by the parser into `Bms` but **deliberately not**
converted into `Chart` by the processor. Their semantics belong to
engine-specific judging, life, and playback logic, which is the player layer's
responsibility. Raw values enter the `Processed.gameplay` / `Processed.audio`
sidecars, and the corresponding `Chart` slots stay `None`.

| Field | Source | Why deferred |
|-------|--------|--------------|
| `gameplay.rank` | `#RANK` | Converting RANK to a judgment-window multiplier is engine-specific (beatoraja differs from LR2) and does not belong to the processor; the raw value goes to the sidecar |
| `gameplay.def_ex_rank` | `#DEFEXRANK` | Same as above, for fine-grained judgment difficulty; the raw value goes to the sidecar |
| `gameplay.total` | `#TOTAL` | Maximum life gauge increase; life logic belongs to the player; the raw value goes to the sidecar |
| `gameplay.vol_wav` | `#VOLWAV` | Master volume; playback and mixing belong to the player; the raw value goes to the sidecar |
| `gameplay.ex_rank_defs` | `#EXRANKxx` | The A0 channel currently passes the Base36 value through directly (`JudgeOverride.rank`) without consulting this table; per-position judge overrides need the overall judging system design and remain **known open** |
| `audio.ex_wav_params` / `wav_cmd` / `cdda` / `midifile` | audio definitions | The core `AudioAsset` carries only paths and slices; the remaining parameters go to the `Processed.audio` sidecar |

## Always / Ask / Never

### Always

- Register new event types in a `collect_*` method on `BmsConverter` and call it
  from `BmsProcessor::process`
- When adding an `EventKind` variant, update `priority` and
  `map_custom`/`map_ext` in `bmsrs-chart` at the same time

### Ask

- Adding a mode family (changes the channel mapping in the `layout` module)
- Changing the event priority order (affects ordering of same-tick events)

### Never

- Add I/O, rendering, or judging logic to the processor
- Modify the `Chart` data model (belongs to the `bmsrs-chart` crate)
