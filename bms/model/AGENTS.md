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
correct radix, and finishes the builder with `MessagesBuilder::finish(base)`;
the builder's type-state lifecycle is documented on `MessagesBuilder`.
`Messages` keeps the raw store private and exposes it through `raw()` for the
writer. Control-flow headers are skipped here (handled by
`bmsrs-bms-syntax`); every other group is stored `Option`-last-wins or
`BTreeMap` by ID.

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| `Bms` is rebuildable | fields roundtrip through `bmsrs-bms-writer` |
| `.bmd` format | `Bms.format` is `Bmd`; `#WAV` values that parse as `SynthParams` go to `Audio.wav_sources`, while file names stay in `Audio.wav_files`; the effective base is Base62 without rewriting `Gameplay.base` |
| `"00"` differs per channel | note channels mean no note; BPM ch03 means rest; LN/ext channels treat it as a value |
| Normalization contract | every `apply` on `Timing`/`Visual`/`Gameplay` receives `base` and normalizes index keys |
| Derived values are forbidden | store the raw literal; move damage, reference resolution, and table lookup to the processor |
| Message source spans | `BmsMessage.span` is a whole-line `SourceSpan`; `Messages` keeps one span per `(measure, channel)` and exposes `line_span()` |

## Tests

Completion: `cargo test -p bmsrs-bms-model` and `cargo test -p bmsrs-bms-parser`
green.

The parser integration tests exercise the model through `BmsParser` and
`Bms::from_flat_tokens`.

