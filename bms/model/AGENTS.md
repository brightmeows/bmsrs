# bmsrs-bms-model

## Role

Whole-file BMS model. `Bms` aggregates metadata, gameplay, timing, display,
audio, visual, messages, and fallback headers. Input is a flat `BmsToken`
stream; output is the faithful `Bms` model. This crate does not know about the
CST, control flow, encoding, or line editing.

## Design philosophy

`Bms` is a faithful mapping: fields are a direct structured representation of
BMS file content and do not store derived values that require computation or
semantic interpretation. Derived values belong to `bmsrs-bms-processor`.

## Lifecycle

`Bms::from_flat_tokens` pre-scans `#BASE`, processes every token with the
correct radix, and finishes the builder with `MessagesBuilder::finish(base)`.
The builder-to-finalized type-state lifecycle is:

| Phase | Method | Purpose |
|-------|--------|---------|
| 1. Collect | `MessagesBuilder::push_message` / `push_stp` | append body strings and `#STP` events |
| 2. Finalize | `MessagesBuilder::finish(base)` | consume the builder, parse events, position-merge, normalize indices |

`MessagesBuilder::finish` consumes the builder, so a caller cannot forget or
repeat the finalize phase. `Messages` keeps the raw store private and exposes
it through `raw()` for the writer.

## Header dispatch

| Status | Variants | Storage |
|--------|----------|---------|
| Stored | Metadata, Gameplay, Timing, Display, Audio, Visual, Fallback | `Option` (last wins) / `BTreeMap` (by ID) |
| Skipped | ControlFlow | handled by `bmsrs-bms-syntax` |

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| `Bms` is rebuildable | fields roundtrip through `bmsrs-bms-writer` |
| `"00"` differs per channel | note channels mean no note; BPM ch03 means rest; LN/ext channels treat it as a value |
| Normalization contract | every `apply` on `Timing`/`Visual`/`Gameplay` receives `base` and normalizes index keys |
| Derived values are forbidden | store the raw literal; move damage, reference resolution, and table lookup to the processor |

## Tests

```bash
cargo test -p bmsrs-bms-model
cargo test -p bmsrs-bms-parser
```

The parser integration tests exercise the model through `BmsParser` and
`Bms::from_flat_tokens`.

