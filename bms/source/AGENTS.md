# bmsrs-bms-source

## Role

BMS source-text layer: raw bytes, lines, editing, and write-back. It handles
only the "bytes to lines" part and does not parse BMS syntax; headers, channels,
and control flow belong to the CST in `bmsrs-bms-syntax`.

## Core invariants

| Invariant | Meaning |
|-----------|---------|
| Unedited line bytes are unchanged | `render()` slices raw bytes for unedited lines, without decoding or re-encoding |
| Only edited lines are encoded | Structured edits replace only the content span; prefix and suffix trivia are spliced from raw bytes |
| Line terminators are preserved per line | LF, CRLF, CR, and no terminator are each preserved; inserted lines inherit a neighbor's terminator |
| The BOM is kept separate | The BOM is a document prefix and never enters the first line's content |
| Unrepresentable characters fail explicitly | `EditError::Unmappable`; the document is unchanged and no substitution is made |
| No semantic classification | Lines are only `Blank` / `Comment` / `Text` / `Error(Decode)` |

## Line model

```text
<prefix trivia><content span><suffix trivia><line terminator>
```

- The content span is the first contiguous non-trivia region; `Line::content`
  records its range in the raw bytes
- The prefix and suffix contain indentation, inline `//` comments, and remnants
  of multi-line `/* */` block comments
- `LineKind`: `Blank` / `Comment` / `Text` / `Error`

## Comment state machine

The state machine scans **raw bytes**, not decoded text. The reason: in every
supported ASCII-compatible encoding, comment markers (`/`, `*`, `;`, `"`) and
line terminators never appear in the trailing byte of a multi-byte character
(trailing bytes start at `0x40` in `Shift_JIS` / `GBK` / `Big5` and at `0xA1` in
`EUC-KR` / `EUC-JP`). UTF-16 is scanned as 2-byte code units.

## Encoding

- Supports every ASCII-compatible, stateless encoding plus UTF-16; rejects
  `ISO-2022-JP` (cross-line state) and `replacement` (cannot decode)
- When encoding an edited line, checks the `had_unmappable` flag returned by
  `Encoding::encode`; that function replaces unmappable characters with numeric
  character references, so a true flag discards the result and returns an error

## Tests

Completion: `cargo test -p bmsrs-bms-source` green.

## Always / Ask / Never

### Always

- Before adding encoding support, confirm it is ASCII-compatible and stateless,
  or implement a dedicated line-splitting strategy
- The edit path must leave the document unchanged on failure
- New line classifications stay at the byte/trivia level; semantic
  classification belongs to `bmsrs-bms-syntax`

### Ask

- Changing the byte-fidelity invariants (for example, introducing whole-file
  re-encoding)
- Changing comment semantics (affects token boundaries for every consumer)

### Never

- Perform encoding conversion, line-ending normalization, or BOM insertion and
  removal in the source layer
- Write edited lines back with `Encoding::encode` directly
- Re-encode unedited lines
- Add BMS tokenizer, parser, or control-flow dependencies to this crate
