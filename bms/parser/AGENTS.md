# bmsrs-bms-parser

## Role

BMS semantic parser: CST to `Bms`. The whole-file model lives in
`bmsrs-bms-model`; this crate owns branch selection, line tokenization, parse
warnings, and the `BmsParser` entry points.

## Entry points

- `BmsParser::parse` / `parse_with`: build from the selected branches of a
  `SourceFile`; warnings cover selected lines only.
- `BmsParser::parse_doc` / `parse_doc_with`: tokenize every line from a
  `Document`; errors in unselected branches also become warnings.
- `tokenize_lines` / `tokenize_lines_with`: `LineToken` with 1-based line
  numbers.
- `ErrorStrategy::CollectAll` collects everything; `FailFast` returns the first
  `BmsParseWarning` sorted by line number.

## Warnings

| Type | Content |
|------|---------|
| `BmsParseWarning` | line number, text range, and category |
| `BmsParseWarningKind::Syntax` | CST syntax or control-flow diagnostic |
| `BmsParseWarningKind::Tokenize` | single-line value parse error |
| `BmsParseWarningKind::Decode` | source text decode error |

## Tests

```bash
cargo test -p bmsrs-bms-parser
```

Coverage: branch selection, selected vs unselected warnings, custom header
prefixes, and tokenizer error handling.

## Always / Ask / Never

### Always

- Keep `BmsParser` as a zero-sized type; operations are associated functions
- Route model construction through `Bms::from_flat_tokens`
- Keep line numbers 1-based in `LineToken` and `BmsParseWarning`

### Ask

- Changing branch-selection semantics (affects every consumer)
- Changing the warning collection strategy

### Never

- Re-implement the `Bms` model or its normalization rules in this crate
- Add encoding detection or byte write-back here

