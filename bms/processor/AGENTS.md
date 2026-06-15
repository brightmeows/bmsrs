# bms-processor

Converts `bms_parser::Bms` → `Chart<T>` via a `BmsMapping` layout.

## Pipeline

```text
bms_parser::Bms → BmsProcessor::process(bms, layout) → Chart<L::NoteData>
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

## Built-in layouts

`Beat7k`, `Beat5k`, `Beat14k`, `Beat10k`.
`BmsProcessor::process_default` uses `Beat7k`.

## Tests

```bash
cargo test -p bms-processor
```
