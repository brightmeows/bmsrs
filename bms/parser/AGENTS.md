# bmsrs-bms-parser

## Role

BMS semantic parser: CST to `Bms`. The whole-file model lives in
`bmsrs-bms-model`; this crate owns branch selection, line tokenization, parse
warnings, and the `BmsParser` entry points.

## Entry points

`BmsParser` and its warning semantics are documented on the type. Every entry
returns `Diagnostic`, which carries the severity the syntax layer assigned;
`FailFast` returns the first error-severity diagnostic as `Err` and leaves
warning-severity ones in the list. Key distinction: `parse`/`parse_with` report
diagnostics from selected branches only; `parse_document` collects them from
every line; `parse_from_selection` replays a recorded `BranchSelection`
strictly.

## Tests

Completion: `cargo test -p bmsrs-bms-parser` green.

## Always / Ask / Never

### Always

- Keep `BmsParser` as a zero-sized type; operations are associated functions
- Route model construction through `Bms::from_flat_tokens`
- Keep line numbers 1-based in `LineToken` and `Diagnostic`

### Ask

- Changing branch-selection semantics (affects every consumer)
- Changing the warning collection strategy

### Never

- Re-implement the `Bms` model or its normalization rules in this crate
- Add encoding detection or byte write-back here

