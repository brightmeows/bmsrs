# bmsrs-bms-syntax

## Role

BMS syntax layer: builds a rowan CST over decoded text and handles lexing,
error-tolerant parsing, nested control flow, and branch-selection queries. It
does not handle byte encoding (`bmsrs-bms-source`) or semantic value parsing
(`bmsrs-bms-parser`).

## Tree shape

```text
SourceFile
  Line (HeaderLine / MessageLine / CommentLine / BlankLine / TextLine / ErrorLine)
  RandomBlock
    RandomDirective
    RandomChain
      RandomBranch
        IfDirective
        Item*
        ElseIfDirective
        Item*
        ElseDirective
        Item*
        EndIfDirective
    EndRandomDirective
  SwitchBlock
    SwitchDirective
    SwitchCase
      CaseDirective / DefDirective
      Item*
      SkipDirective
    EndSwDirective
```

- Inline tokens: `Hash` / `Percent` / `CommandName` / `Value` / `Measure` /
  `Channel` / `Colon` / `BodyChunk`, plus `Whitespace` / `Comment` / `Newline`
- Invalid content is wrapped in `ErrorLine` or `ErrorToken`, preserving the
  original text where possible
- `Diagnostic` carries `Severity` and `DiagnosticCode` from `bmsrs-pipeline`,
  so later layers copy them instead of translating a category enum

## API notes

- `IncrementalParser` reuses the green tree for single-line content edits,
  re-lexes only the changed line, and falls back to a full parse for
  control-flow or structural edits
- `replay_lines` is strict: drift in block count, kind, or declared
  bound fails with `BranchMismatch`; `#SETRANDOM` / `#SETSWITCH` values are
  taken from the current file, not the recording
- `ParseOptions.header_prefixes` defaults to `['#', '%']`

## Control-flow selection

| Block | Rule |
|-------|------|
| `#RANDOM` | One random value per block; each `#IF` chain matches independently from the top; `#ELSE` is the fallback |
| `#SETRANDOM` | Uses a fixed value and does not call the RNG |
| `#SWITCH` | One random value per block; after the first matching `#CASE`, falls through until a case with `#SKIP` |
| `#DEF` | Always matches |
| Zero value | `#RANDOM 0` / `#SWITCH 0` are treated as no match and do not panic; `ControlFlowCompat::Jbms` clamps to `1` and draws once, while LR2's `GetRand(-1)` consumes a word and selects `1` |
| `#ELSE` / `#ELSEIF` | a real fallback branch; `ControlFlowCompat::Jbms` reproduces jbms-parser, which reserves only RANDOM / IF / ENDIF / ENDRANDOM, so the directive is ignored and every following body stays under the preceding `#IF` |
| RNG ownership | `select_lines(&mut rng, compat)` leaves the generator with the caller so the stream continues (LR2's RANDOM lane option relies on it); `random` provides `FirstBranch` and `Lr2Mt19937`, `java_random` provides beatoraja's `JavaRandom`, and every `FnMut(u64) -> u64` closure implements `ControlFlowRng` through a blanket impl |

## Tests

Completion: `cargo test -p bmsrs-bms-syntax` green; the incremental benchmark
(`--bench incremental -- --quick`) compares single-line edits against a full
parse.

## Always / Ask / Never

### Always

- Add a new token/node to `SyntaxKind` first, then update the lexer, parser, and
  builder together
- Keep every character in exactly one token; use `SyntaxNode::text()` for
  regression checks; add a lossless assertion to the integration tests when
  adding a syntax node
- Control-flow directive lines produce structure nodes only; do not stuff
  payload into them

### Ask

- Changing control-flow selection semantics (fall-through, `#SKIP`, multiple
  chains)
- Changing the tree shape (flattening, merging nodes)

### Never

- Perform encoding detection or byte write-back in the syntax layer
- Construct `BmsHeader` / `BmsMessage` semantic values in the syntax layer
- Panic on invalid input or return an empty tree early
