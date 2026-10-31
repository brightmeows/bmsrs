# bmsrs-bms-parser

## Role

BMS semantic layer: rowan CST to the structured `Bms` model. Typed single-line
syntax (`BmsHeader` / `BmsMessage` / index types) lives in `bmsrs-bms-values`;
control-flow branch selection is done by
`bmsrs-bms-syntax::SourceFile::select_lines`.

## Design philosophy

**`Bms` is a faithful mapping.** `Bms` fields are a direct structured
representation of BMS file content; they do not store derived values that
require computation or semantic interpretation. See `bms/AGENTS.md` for the
domain-level criteria.

`BgaEvent.layer` uses this crate's `BmsBgaLayer`; `bmsrs-bms-parser` does not depend
on `bmsrs-chart`. The mapping from layers to `bmsrs_chart::BgaLayer` belongs to
`bmsrs-bms-processor`.

| Does not do | Reason |
|-------------|--------|
| Value interpretation (paths, URLs, emails) | Strings are kept as-is |
| Control-flow expansion | Handled by `bmsrs-bms-syntax`'s `select_lines` |
| Derived value computation | Belongs to the processor (for example, damage = base36/2, reference to absolute value) |
| Reference-definition pair resolution | `Bms` faithfully stores the reference form (such as `BpmValue::Reference`); lookup belongs to the processor |

> `Metadata::parse_implicit_subtitle` is an exception: it is provided as an
> opt-in utility and is not called automatically in `from_flat_tokens`; callers
> explicitly choose whether to run subtitle inference.

## Lifecycle

`Messages` uses a two-phase design:

| Phase | Method | Purpose |
|-------|--------|---------|
| 1. Collect | `concat_raw` | append body strings to `(measure, channel)` |
| 2. Finalize | `finalize(base)` | parse events, position-merge, normalize indices |

`Bms::from_cst` calls `from_flat_tokens` after selecting branches; the latter
calls `finalize` automatically. Manual construction must call it explicitly.
`finalize` can be called repeatedly (it clears first), but calling it once is
recommended.

## CST / document entry points

- `Bms::from_cst` / `from_cst_with`: build from tokens of selected branches;
  warnings cover only selected lines
- `Bms::from_document` / `from_document_with`: tokenize every line; errors in
  unselected branches also become warnings
- `tokenize_lines` / `tokenize_lines_with`: `LineToken` with 1-based line numbers
- `ErrorStrategy::CollectAll` collects everything; `FailFast` returns the first
  `CstWarning` sorted by line number

## Header dispatch

| Status | Variants | Storage strategy |
|--------|----------|------------------|
| Stored | Metadata, Gameplay, Timing, Display, Audio, Visual, `Fallback` | Option (last wins) / BTreeMap (by ID) |
| Skipped | ControlFlow | Handled by `bmsrs-bms-syntax::select_lines` |

### Message channel handling

| Channel type | Strategy |
|--------------|----------|
| BGM (ch 01) | each line independently (polyphony support) |
| Measure length (ch 02) | last line wins |
| Options (ch A6) and other channels | position-merge (non-`"00"` overrides, `"00"` preserved) |

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| Measure length value is a ratio | `1.0` = 4/4, `0.75` = 3/4, `2.0` = 8/4. Not a percentage. |
| `finalize(base)` parameter | Controls index normalization: Base36 uppercases, Base62 preserves the original case. Base62 applies only to the seven declared families (`#WAVxx`/`#BMPxx`/`#BPMxx`/`#STOPxx`/`#SCROLLxx`/`#SPEEDxx`/`#LNOBJ`); see `family_base` |
| Event indices go through `normalize(family_base)` | Event-channel references are all seven-family (Base62-capable) indices; definition headers outside that list (TEXT/EXRANK/CHANGEOPTION/SEEK/ARGB/SWBGA/#BGA) normalize with Base36 even in Base62 files |
| `"00"` means different things per channel | note channels = no note (skip), BPM ch03 = rest (skip), LN/ext channels = valid value |
| Normalization contract (all def keys) | Every `apply` on `Timing`/`Visual`/`Gameplay` receives `base` and normalizes index keys (including `ex_rank_defs`/`change_option_defs`). `Gameplay::apply` follows this contract too |
| `non_event_data` pre-normalization | For non-event channels (opacity/ARGB/text/option/seek, etc.), merged strings are split into 2-character values in `finalize_merged`, then normalized per value with `BmsBase::Base36` (these channels reference non-Base62 definition families) and stored in `NonEventData.values`. The processor does not need to normalize again when looking up definitions (removing the duplicate normalization previously marked F1/F2) |
| `warnings` channel | 2-character values whose shape is valid but do not satisfy the channel parse rule are skipped and recorded in `Messages.warnings` (`ChannelValueWarning`, with measure/channel/value); missing references (undefined WAV/BPM/STOP keys) belong to the processor layer's `ProcessWarning::Missing*` |

## Always / Ask / Never

### Always

- All index values (Wav/Bmp/Bpm/Stop/Scroll/Speed/ExRank/ChangeOption) are
  created through `normalize(base)`
- `apply` on `Timing`/`Visual`/`Gameplay` receives `base` and normalizes index
  keys
- Register new event types in `Messages` and the `lib.rs` re-exports
- Document the new event type's channel and semantics in AGENTS.md

### Ask

- Changing `finalize` processing strategy (affects channel merge behavior)
- Adding a channel that needs special merge semantics

### Never

- Perform engine-specific inference automatically in the parser layer
  (`parse_implicit_subtitle` and similar require an explicit call)
- Assume `"00"` means "no-op" on every channel
- Store derived values that only the processor needs (see the "derived values"
  prohibition in the design philosophy)

## Tests

Public API tests live in `tests/`; `merge_channel` algorithm tests stay inline.
