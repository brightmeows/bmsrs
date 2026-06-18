# bms-processor

Converts `bms_parser::Bms` → `Chart<NoteData>` via a `BmsLayout`
mode family (defined in `bmsrs-chart::layout`).

## Pipeline

```text
bms_parser::Bms → BmsProcessor::process(bms, layout) → Chart<NoteData>
```

## Long-note modes

Two LN notations, auto-detected:

- **LNOBJ**: `#LNOBJ` designates a WAV index; regular notes paired by
  that index form LNs.
- **LNTYPE 1 (RDM)**: channels 51–69 are paired consecutively per
  `(player, lane)`.

The `as_str()` method bridges `BmsIndex<LnObjTag>` to `NoteEvent.wav_id`
(`BmsIndex<WavTag>`) since they are distinct phantom-typed indices.

## Position conversion

`MeasureTable` pre-computes cumulative tick offsets per measure to handle
variable meter (`#XXX` measure length changes). Tick at `Position{measure,
numer, denom}` = `measure_starts[measure] + numer * measure_len / denom`.

## Stop conversion

- `#STOP` raw value = fraction of 1/192 measure → `raw/192 * res * 4` ticks.
- `#STP` = milliseconds → ticks via `bpm_at_tick` helper.

## Mode families

The layout argument selects a mode family from `bmsrs_chart::layout`:
`Bme` (beat-5k/7k/10k/14k, uniform), `Pms`, `PmsBme`, `Nanasi`,
`DscOctFp`. Each family decodes `(player, lane)` channel bytes directly
into the `(PlayerSide, Lane)` pair stored on each note.
`BmsProcessor::process_default` uses `Bme` unconditionally (the `#PLAYER`
header does not affect mapping).

## Tests

```bash
cargo test -p bms-processor
```
