# bmson-def

bmson format type definitions (v0/v1/v2).

## Module Layout

- `v0` and `v1` are **independent modules** — each with its own `Bmson`,
  `BmsonInfo`, `SoundChannel`. Import from `bmson_def::v0` or `bmson_def::v1`.
- Common types (`NoteEvent`, `BpmEvent`, `BGA`, `ModeHint`, …) are
  defined in `common.rs` and re-exported from the crate root.
- Version-specific types and conversion traits (`From`/`TryFrom`) live
  in each version submodule (`v0.rs`, `v1.rs`), not in `common.rs`.

## Version Detection

- `detect_version()` returns `DetectedVersion` (V0/V1/V2) by scanning
  the JSON `"version"` field (lightweight, no `serde_json` needed).

## Dependency Usage

- `serde_json` is a **dev-only** dependency — the lib itself does not
  depend on any JSON library.  Tests use `serde_json::from_str`/`to_string`
  directly with the version-specific types.
