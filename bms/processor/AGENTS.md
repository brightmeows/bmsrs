# bms-processor

Converts `bms_parser::Bms` → `Chart<NoteData>` via a [`BmsLayout`]
mode family (defined in this crate's [`layout`] module).

## Pipeline

```text
bms_parser::Bms → BmsProcessor::process::<L>(bms) → Chart<NoteData>
```

where `L: BmsLayout` is a stateless mode family (e.g. `Bme`, `Pms`, `Nanasi`).

## Long-note modes

Three LN notations, auto-detected:

- **LNOBJ**: `#LNOBJ` designates a WAV index; regular notes paired by
  that index form LNs.
- **LNTYPE 2 (MGQ)**: channels 51–69 per `(player, lane)`. A `"00"`
  entry acts as a release for any active LN on that channel.
- **LNTYPE 1 (RDM)**: channels 51–69 per `(player, lane)`. `"00"`
  entries are skipped; remaining events form consecutive start-end pairs.

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

A mode family is a ZST implementing `BmsLayout` (stateless trait).
Each family is a single-table mapping: a `match` on `BmsChannel::lane()`
or `(player(), lane())` that produces `Option<NoteData>`.

Standard families cover BMS play modes — see the `layout` module for the
current catalogue of families and their channel-to-lane tables.

## Tests

```bash
cargo test -p bms-processor
```
