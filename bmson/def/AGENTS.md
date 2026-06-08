# bmson-def

bmson format type definitions (v0/v1/v2).

## Public API Conventions

- `v0` and `v1` are **independent modules** — each exports its own `Bmson`,
  `BmsonInfo`, `SoundChannel`, and version-specific types. Users import
  from `bmson_def::v0::*` or `bmson_def::v1::*`.
- All other public types (`NoteEvent`, `BpmEvent`, `BGA`, `ModeHint`, etc.)
  are re-exported from the **root module** (`bmson_def::*`) via `common.rs`.
- Conversion traits (`From`/`TryFrom`) live in the version modules
  (`v0.rs`, `v1.rs`), not in `common.rs`.
- Helper functions are re-exported from the root module and double as
  `#[serde(deserialize_with)]` targets: `null_to_default`, `null_to_u64`,
  `default_multiplier`, `default_resolution`, `deserialize_resolution_nonzero`,
  `de_path`, `de_opt_path`.
