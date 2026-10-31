# bmsrs-bms-analysis

## Role

BMS analysis layer: produces editor data on top of the `bmsrs-bms-syntax` CST. It does
not depend on LSP types; protocol adaptation belongs to the separate `bms-lsp`
repository.

## Output

| Data | Description |
|------|-------------|
| `Analysis::diagnostics()` | CST syntax and control-flow diagnostics, tokenizer errors, duplicate definitions, missing references |
| `Analysis::index()` | definitions and references for WAV / BMP / BGA / BPM / STOP / SCROLL / SPEED / EXRANK / CHANGEOPTION / TEXT / SEEK |
| `header_doc()` | English header command descriptions, matched by command prefix (`WAV01` gives `WAV`) |
| `channel_doc()` | English channel descriptions, including note-channel groups |

`docs.rs` is a manually maintained summary; the BMS command memo linked from
each entry is authoritative. Check the memo before changing a summary; do not
treat `docs.rs` as the specification text.

## Index rules

- Definitions: `#WAVxx` / `#EXWAVxx`, `#BMPxx` / `#EXBMPxx`, `#BGAxx` /
  `#@BGAxx` / `#SWBGAxx` / `#ARGBxx`, `#BPMxx` / `#EXBPMxx`, `#STOPxx`,
  `#SCROLLxx`, `#SPEEDxx`, `#EXRANKxx`, `#CHANGEOPTIONxx`, `#SEEKxx`,
  `#TEXTxx` / `#SONGxx`
- References: non-`00` two-character values in channel bodies; the channel kind
  determines the resource kind
- Non-canonical addresses (such as `#1:0101`) fall back to
  `parse_message_line` on Header lines and still build reference indices
- BGA channels (04/06/07/0A) may fall back to `#BMPxx` definitions
- IDs are uppercased for case-insensitive matching

## Diagnostic rules

- CST syntax error gives Error; control-flow structure error gives Warning
- Tokenizer value parse error gives Error
- Multiple definitions of the same kind and ID produce one duplicate warning per
  definition
- A reference with no definition produces one missing warning at the reference

## Tests

```bash
cargo test -p bmsrs-bms-analysis
```

Coverage: definition/reference indices, missing references, duplicate
definitions, BGA to BMP fallback, control-flow diagnostics, header value errors,
and documentation lookup.

## Always / Ask / Never

### Always

- When adding a resource kind, update `SymbolKind`, the definition prefix table,
  the channel mapping, and diagnostics together
- Custom header prefixes go through `Analysis::from_text_with(&ParseOptions)`
- Keep `TextRange` on both definitions and references for LSP navigation and
  highlighting
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
