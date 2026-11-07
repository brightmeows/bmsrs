# bmsrs-bmson-model

## Role

Type definitions for the bmson JSON format (v0/v1/v2).

## Module structure

| Path | Content |
|------|---------|
| `common.rs` | common types (`NoteEvent`, `BpmEvent`, `BGA`, `ModeHint`, and so on), re-exported from the crate root |
| `v0.rs` / `v1.rs` | version-specific types plus `From`/`TryFrom` conversion traits |
| `lib.rs` (root) | v2-specific types (`Bmson` v2, `SongInfo`, `ChartInfo`, `ChartData`), defined at the crate root without a separate module |

`v0` and `v1` are **independent modules**, each with its own `Bmson`, `BmsonInfo`,
and `SoundChannel`. Import them from `bmsrs_bmson_model::v0` or `bmsrs_bmson_model::v1`.

## Version detection

`DetectedVersion::detect(json)` returns a `DetectedVersion` (V0/V1/V2) by scanning
the JSON `"version"` field. The implementation is lightweight and does not need
`serde_json`.

## Dependencies

`serde_json` is a **dev-only** dependency; the library itself does not depend
on any JSON library. Tests interact with version-specific types directly
through `serde_json::from_str`/`to_string`.

## Example

```bash
cargo run --example bmson_parser -p bmsrs-bmson-model
```

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| v2 has no separate module | Unlike v0/v1, v2 lives at the crate root rather than in a directory |
| Negative `resolution` takes its absolute value; `0` is kept | Matches the specification: negatives take the absolute value at deserialization (`deserialize_resolution`), and a kept `0` falls back to 240 in the processor with a warning |
| Conversions between versions use `From` | `Bmson::from(v0_bmson)` upgrades to the v2 schema uniformly |
| v2 `ChartData.ln_type_hint` is `Option<LnType>` | a missing JSON field deserializes to `None` (the source did not declare it); the former `#[serde(default)] = ln` folding is gone, so "absent" and "explicit `ln`" stay distinguishable. `None` is omitted from JSON output (`skip_serializing_if`), and downgrades to v0/v1 write an explicit `ln` back as an explicit value |
