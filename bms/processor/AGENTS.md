# bmsrs-bms-processor

## Role

Processor that converts `Bms` into a format-agnostic `Chart`. BMS player and
key mapping are decoupled through the `Layout` mode family.

The processor performs **lossy conversions**: merge priority (`#BGA`/`#BMP`),
reference resolution (WAV/BPM), and table lookup (def tables) all live here. See
`bms/AGENTS.md` for domain-level criteria.

`Processed` carries three sidecars in addition to `chart`; recoverable warnings
come back in the `bmsrs_pipeline::Outcome` wrapper:

| Field | Content |
|-------|---------|
| `gameplay` | raw values of `#PLAYER`/`#EXRANKxx`/`#OPTION`/`#CHANGEOPTIONxx` |
| `audio` | raw values of `#EXWAV` parameters, `#WAVCMD`, `#CDDA`, `#MIDIFILE`, `#PATH_WAV` |
| `kaleid` | `KaleidBeat` declarations, extension-channel objects, conditional blocks, and BMD synth definitions; `None` for ordinary charts. `#OPTION` stays only in `gameplay.option` |

`#PATH_WAV` is applied to `AudioAsset.path` as well; the sidecar keeps the raw
prefix.

## Long-note modes and stop conversion

Long-note mode detection: LNOBJ when `#LNOBJ` is defined, otherwise LNTYPE 1
(RDM) pairing by default, LNTYPE 2 (MGQ) with `#LNTYPE 2`. Pairing algorithms
live in `long_note.rs`. `#STP` header milliseconds are never quantized;
same-position entries accumulate per the specification.

## Mode families

Mode families implement the `Layout` trait through a `&self` receiver, so a
zero-sized family and a runtime-selected value take the same parameter:
`Processor::process_with(bms, layout)`. Each family is a mapping table from
`(player, lane)` to `Option<(NoteSide, Lane)>`.

Standard families are in the public `layout` module. `Bme` covers
5K/7K/10K/14K; `#PLAYER` does not affect the mapping (consistent with mainstream
engines), so automatic family selection by `#PLAYER` is not implemented.
`PmsLayout::detect(bms)` picks the PMS variant from the chart's channels and
implements the trait by delegating to `Pms` or `PmsBme`.

## Public modules

| Module | Content |
|--------|---------|
| `layout` | `Layout` trait plus standard mode families (`Bme` and others) |
| `custom_event` | `BmsCustomEvent` enum: BMS-specific events (BGA opacity, text, options, and so on) used as the `CustomEvent` type parameter of `Chart` |

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| Default BPM 130 | Matches the BMS specification; not `0.0` |
| Event ordering | Bar(0), then Note/BGA/BGM(1), then BPM(2), then Stop(3), then Scroll(4), then Speed(5), then Custom(6) |
| Same-position notes | Later definition wins with `DuplicateNotePosition` (matching beatoraja); mines never overwrite and are dropped with `MineNoteConflict`; LNOBJ end markers displace with a warning, ch51-69 ends silently |
| LNOBJ end silence | The end marker's WAV does not play (matching beatoraja/LR2/bmz); the RDM-spec end-marker BGM behavior is deliberately not implemented |
| ch51-69 under LNOBJ | Mutually exclusive with LNOBJ (memo/10 leaves it undefined); not discarded, kept as ordinary visible notes |
| Mine `damage` | `MineEvent.raw_value` (`Option<u16>`) goes through `mine_damage()`: `Some(1295)` gives INFINITY, `Some(n)` gives n/2, `None` gives 1.0 |
| Visible notes under a long-channel pair | move to BGM with `NoteCoveredByLongNote` (matching beatoraja); a same-WAV note on the long-note start is dropped as redundant |
| BGA clear semantics | channel value `00` is a rest; an undefined BMP reference emits `MissingBmpDefinition` and clears the layer (`resource_id: None`) instead of dropping the event |
| SWBGA indexing | the A5 value indexes the SWBGA definition table, not the BMP table; pattern frames resolve against `bga_resources` |
| nanasi miss text | `#TEXT00` maps to `ChartInfo.miss_text` and never becomes a channel event |
| Video SEEK resolution | `#SEEK` resolves milliseconds (not the raw Base36 value); undefined ids are skipped |
| Negative `#STOP` values | clamped to `0` and skipped with a warning; beatoraja (jbms-parser) takes the absolute value instead, deliberately not followed |
| `process_default` layout | Uses `DscOctFp` when `#OCT/FP` is true, otherwise `Bme`; `#PLAYER` is still ignored, and modes such as PMS go through `process_with(bms, PmsLayout::detect(bms))` |
| `key_count` | Sum of the maximum `Lane::Key` number per side across the event stream; `None` when there are no key notes |
| Judge/life multipliers | BMS `#RANK`/`#TOTAL` are not converted into the multiplier fields; they enter `ChartInfo.judge_rank`/`ChartInfo.total` as raw values instead, and both multiplier fields stay `None` for BMS |
| Undefined reference warnings | Undefined BMP/TEXT/SWBGA/EXRANK references emit `MissingBmpDefinition`/`MissingTextDefinition`/`MissingSwBgaDefinition`/`MissingExRankDefinition` with the resolved key and tick; the corresponding event is dropped or carries a clear/blank semantics |
| `#PATH_WAV` | Joined when constructing `AudioAsset.path`; the raw prefix stays in `Processed.audio.path_wav` |
| Asset existence | `AssetLookup::Probe` asks a caller-supplied closure about each audio and BGA path and reports `MissingAssetFile`; the processor itself never touches the file system (`Declared` is the default) |
| Warning shape | `ProcessWarning { span, kind }`; `ProcessWarningKind` carries the payload, `span` is the source line when available |
| `#BPM 0` | Returns `ProcessError::ZeroBpm` instead of falling back to 130; the current time model cannot represent zero BPM |
| Conversion steps belong to `BmsConverter` | `collect_*`/`build_*` are methods on the private internal `BmsConverter`, not free functions |
| `non_event_data` index normalization | Done in the parser layer: every 2-character value was normalized in `finalize_merged` using the pre-scanned `#BASE`, so processor lookups hit directly without another `normalize(base)`. The `BmsConverter.base` field was removed because it is no longer needed |
| `bpm_at_tick` source | Delegates to `BpmLookup::new(init_bpm, changes).bpm_at_tick(tick)` (the former local duplicate is gone). STP-to-tick conversion still has to happen before `TimingTrack` construction, but the algorithm is unified |
| BGA crop source path | The `bmp_index` of `#BGA`/`#@BGA` is a decimal source number; it is matched against `bmp_files` keys by hexadecimal value (for example, `16` gives `"10"`), and only 0-255 is referenceable; `#@BGA` (w/h) is normalized to the bottom-right corner. Crop ids whose source path cannot be resolved are skipped |

## Deliberate deferral (parsed but not converted here)

The following fields are parsed by the parser into `Bms` but **deliberately not**
converted into `Chart` by the processor. Their semantics belong to
engine-specific judging, life, and playback logic, which is the player layer's
responsibility. Raw values enter the `Processed.gameplay` / `Processed.audio`
sidecars, and the corresponding `Chart` slots stay `None`.

| Field | Source | Why deferred |
|-------|--------|--------------|
| `gameplay.ex_rank_defs` | `#EXRANKxx` | The A0 channel resolves its references into `Event::JudgeRank` percentages; the raw table stays in the sidecar for the judgment system's `#DEFEXRANK` multiplication design |
| `audio.ex_wav_params` / `wav_cmd` / `cdda` / `midifile` | audio definitions | The core `AudioAsset` carries only paths and slices; the remaining parameters go to the `Processed.audio` sidecar |

## Known limitations

These are boundaries intentionally left open. Check them before assuming a
feature is missing by accident.

| Item | Status | Reassess when |
|------|--------|---------------|
| `#CDDA` / `#MIDIFILE` | only stored in the `audio` sidecar; no playback abstraction | a player needs CD or MIDI sources |
| KaleidBeat free zones | BMS text carries them as `BmsChannel::Extension`; `BmsKaleid.objects` holds them as `KaleidObjectKind::FreeZone`, and free-zone judgment semantics stay in the player | a player needs a shared free-zone evaluation model |
| KaleidBeat conditional branches | structure, conditions, and branch rows are preserved; condition evaluation, DEFAULT-based scoring, and branch replay stay in the player | a player needs the shared evaluation model |
| BMD synthesis audio | synthetic `AudioAsset.path` values (`bmd:<wav-id>`) and `BmsKaleid.synth`; a player without the sidecar sees them as missing files, and the processor never probes those paths | a generic player needs a first-class synth source |
| BGM layering | BGM events stay separate; only judgeable notes merge `NoteAudio`; beatoraja (`bgnotes`) and bmz-player (`bgm_events`) keep them separate as well | a real chart needs BGM layers merged |
| Warning precision | line-level `SourceSpan`; the exact note or cell is not stored; jbms-parser logs are line-based and bmz-player warnings carry keys but no cells | an editor needs cell-level warning highlighting |

## Cross-engine divergences

Where engines disagree, the default follows the specification and an opt-in
switch or a documented refusal covers the compatibility case. Do not "fix" one
side silently.

| Case | bmsrs default | Other engine | Where |
|------|---------------|--------------|-------|
| `#LNTYPE 2` | MGQ-style pairing on channels 51-69, per the BMS specification | KaleidBeat's manual defines it as LNOBJ-terminated long notes | documented only; the standard reading stays |

## Always / Ask / Never

### Always

- Register new event types in a `collect_*` method on `BmsConverter` and call it
  from `Processor::process`
- When adding an `EventKind` variant, update `priority` and
  `map_custom`/`map_ext` in `bmsrs-chart` at the same time

### Ask

- Adding a mode family (changes the channel mapping in the `layout` module)
- Changing the event priority order (affects ordering of same-tick events)

### Never

- Add I/O, rendering, or judging logic to the processor
- Modify the `Chart` data model (belongs to the `bmsrs-chart` crate)
