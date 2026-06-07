# bmson-def

bmson format type definitions (v0/v1/v2).

## Module Responsibilities

```
src/
├── lib.rs     # Root (v2) types: Bmson, SongInfo, ChartInfo, ChartData,
│              # JudgementDeltas, LifeDeltas, V0LnType
├── common.rs  # Cross-version shared types: NoteEvent, BpmEvent, StopEvent,
│              # BGA, ModeHint, LnType/LnJudge/LnLife, beatoraja extensions,
│              # SoundChannel, BarLine, helper functions
├── v0.rs      # v0.2.1 legacy types: Bmson, BmsonInfo, BarLine(k), EventNote,
│              # SoundChannel(notes). Conversions: v0 ↔ root.
└── v1.rs      # v1.0.0 flat types: Bmson, BmsonInfo, SoundChannel(notes).
               # Conversions: v1 ↔ root.
```

## Public API Conventions

- `v0` and `v1` are **independent modules** — each exports its own `Bmson`, `BmsonInfo`, `SoundChannel`, and version-specific types. Users import from `bmson_def::v0::*` or `bmson_def::v1::*`.
- All other public types (`NoteEvent`, `BpmEvent`, `BGA`, `ModeHint`, etc.) are re-exported from the **root module** (`bmson_def::*`) via `common.rs`.
- Conversion traits (`From`/`TryFrom`) live in the version modules (`v0.rs`, `v1.rs`), not in `common.rs`.
- Helper functions (`null_to_default`, `default_multiplier`, `default_resolution`) are `pub(crate)` — not part of the public API.
