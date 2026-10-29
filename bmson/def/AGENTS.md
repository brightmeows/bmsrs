# bmsrs-bmson-def

## Role

Type definitions for the bmson JSON format (v0/v1/v2).

## Module structure

| Path | Content |
|------|---------|
| `common.rs` | common types (`NoteEvent`, `BpmEvent`, `BGA`, `ModeHint`, and so on), re-exported from the crate root |
| `v0.rs` / `v1.rs` | version-specific types plus `From`/`TryFrom` conversion traits |
| `lib.rs` (root) | v2-specific types (`Bmson` v2, `SongInfo`, `ChartInfo`, `ChartData`), defined at the crate root without a separate module |

`v0` and `v1` are **independent modules**, each with its own `Bmson`, `BmsonInfo`,
and `SoundChannel`. Import them from `bmsrs_bmson_def::v0` or `bmsrs_bmson_def::v1`.

## Version detection

`DetectedVersion::detect(json)` returns a `DetectedVersion` (V0/V1/V2) by scanning
the JSON `"version"` field. The implementation is lightweight and does not need
`serde_json`.

## Dependencies

`serde_json` and `chumsky` are **dev-only** dependencies; the library itself does
not depend on any JSON library. Tests interact with version-specific types
directly through `serde_json::from_str`/`to_string`. `chumsky` is used by the
`bmson_parser` example (an error-recovering JSON parser demonstration).

## Example

```bash
cargo run --example bmson_parser -p bmsrs-bmson-def
```

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| v2 has no separate module | Unlike v0/v1, v2 lives at the crate root rather than in a directory |
| `resolution: 0` is rejected | Stricter than the specification: bmson says 0 falls back to 240, but this implementation rejects the whole file (`deserialize_resolution_nonzero`); authoring-tool defects are not silently rewritten. A 0 on the v0/v1 upgrade path is rejected by downstream `TimingTrack` construction with a correct error label |
| Conversions between versions use `From` | `Bmson::from(v0_bmson)` upgrades to the v2 schema uniformly |
