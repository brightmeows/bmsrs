# bmsrs-bms-syntax

## Role

BMS syntax layer: builds a rowan CST over decoded text and handles lexing,
error-tolerant parsing, nested control flow, and branch-selection queries. It
does not handle byte encoding (`bmsrs-bms-source`) or semantic value parsing
(`bmsrs-bms-parser`).

## Core invariants

| Invariant | Meaning |
|-----------|---------|
| Lossless | Every character enters a token; `SyntaxNode::text()` reconstructs the input |
| Error-tolerant | Invalid lines, unclosed blocks, and orphan directives never panic; errors become `Diagnostic` |
| Line-oriented | Every line has at least one `Line` node; control-flow blocks nest under `SourceFile` |
| Semantics-free | Does not construct `BmsHeader` / `BmsMessage`; keeps only command names and value text |
| Branches optional | The CST keeps all branches; `select_lines` returns the selected lines for a random value |

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

## API

- `SourceFile::parse(text)` returns `Parse`; `Parse::source_file()` / `errors()`
- `SourceFile::parse_with(text, &ParseOptions)` supports custom header prefixes
- `SourceFile::parse_document(&Document)` / `parse_document_with` enter the CST
  from a lossless document
- `SourceFile::lines()` iterates all lines depth-first
- `Line::command_name()` / `value()` / `measure()` / `channel()` / `body_chunks()`
- `SourceFile::select_lines(rng)` returns `(Vec<Line>, BranchSelection)`;
  `rng(max)` returns `[1, max]`, and `#SETRANDOM` / `#SETSWITCH` do not call it
- `ParseOptions.header_prefixes` defaults to `['#', '%']`; non-default prefixes
  produce a `Prefix` token

## Control-flow selection

| Block | Rule |
|-------|------|
| `#RANDOM` | One random value per block; each `#IF` chain matches independently from the top; `#ELSE` is the fallback |
| `#SETRANDOM` | Uses a fixed value and does not call the RNG |
| `#SWITCH` | One random value per block; after the first matching `#CASE`, falls through until a case with `#SKIP` |
| `#DEF` | Always matches |
| Zero value | `#RANDOM 0` / `#SWITCH 0` are treated as no match and do not panic |

## Tests

```bash
cargo test -p bmsrs-bms-syntax
```

Public API tests live in `tests/` and cover losslessness, inline tokens, nested
control flow, error recovery, and branch selection. Add a lossless assertion
when adding a syntax node.

## Always / Ask / Never

### Always

- Add a new token/node to `SyntaxKind` first, then update the lexer, parser, and
  builder together
- Keep every character in exactly one token; use `SyntaxNode::text()` for
  regression checks
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
