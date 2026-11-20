# bmsrs-bms-analysis

## Role

BMS analysis layer: produces editor data on top of the `bmsrs-bms-syntax` CST. It does
not depend on LSP types; protocol adaptation belongs to the separate `bms-lsp`
repository.

## Output

`Analysis` carries CST/tokenizer diagnostics, duplicate-definition and
missing-reference warnings, a symbol index over eleven resource kinds, and
incremental refresh through `Analysis::update`.

`docs.rs` is a manually maintained summary; the BMS command memo linked from
each entry is authoritative. Check the memo before changing a summary; do not
treat `docs.rs` as the specification text.

## Index rules

- Definitions and reference rules live in the rustdoc on `Analysis` and
  `SymbolIndex`; `docs.rs` stays a summary, not the specification.
- Typed `KaleidBeat` declaration and channel lines are modeled by the values and
  processor crates; `#BMD` remains the only `extension` Information diagnostic.
  Conditional blocks are modeled structurally and get `conditional` warnings for
  ordering, duplicate/missing `#DEFAULT`, nesting, and unclosed blocks. The
  shipped manual is authoritative; bmz-player's `docs/kaleid.md` is a secondary
  contract.
- References: non-`00` two-character values in channel bodies; the channel kind
  determines the resource kind
- Non-canonical addresses (such as `#1:0101`) fall back to
  `BmsMessage::parse` on Header lines and still build reference indices
- BGA channels (04/06/07/0A) may fall back to `#BMPxx` definitions
- IDs are uppercased for case-insensitive matching

## Diagnostic rules

- CST syntax error gives Error; control-flow structure error gives Warning
- Tokenizer value parse error gives Error
- Multiple definitions of the same kind and ID produce one duplicate warning per
  definition
- A reference with no definition produces one missing warning at the reference

## Tests

Completion: `cargo test -p bmsrs-bms-analysis` green.

## Always / Ask / Never

### Always

- When adding a resource kind, update `SymbolKind`, the definition prefix table,
  the channel mapping, and diagnostics together
- Custom header prefixes go through `Analysis::from_text_with(&ParseOptions)`
- Store `(line, line-offset)` on definitions and references and expose absolute
  ranges through `SymbolIndex::range_of_definition` / `range_of_reference`
- Use longest-prefix matching for documentation lookup so `BPM` does not steal
  `EXBPM`

### Ask

- Changing what counts as a definition or reference (affects diagnostic noise)
- Adding dependencies that need filesystem access or LSP types

### Never

- Depend on LSP protocol types in this crate
- Treat branch-selection results as the only truth; the index must cover every
  branch
- Pile the same diagnostic onto the same range repeatedly
