# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.4](https://github.com/brightmeows/bmsrs/compare/bmsrs-pipeline-v0.0.3...bmsrs-pipeline-v0.0.4) - 2026-11-28

### Documentation

- *(pipeline)* list ErrorStrategy and SourceFormat in the crate doc
- prune AGENTS.md and co-locate the boundary registers
- refresh the release version and record the visibility rules

### Features

- *(facade)* [**breaking**] accept Input on the BMSON entry points (migration: bmson::ParseError::Utf8 is replaced by Decode(TextDecodeError) and the enum is non_exhaustive with a new Format variant; SourceFormat::from_path now maps .json to Bmson.)

### Refactoring

- *(pipeline)* [**breaking**] rename Outcome warnings to diagnostics (migration: Outcome::warnings is now Outcome::diagnostics, and Outcome::map_warnings is now Outcome::map_diagnostics.)
- *(facade)* [**breaking**] collapse the parse entry points behind Input (migration: `bms::parse_bytes`, `parse_bytes_with`, and
`parse_file` are removed; build an `Input` (`Input::text(text, format)`,)
- *(pipeline)* [**breaking**] make DiagnosticCode the only diagnostic vocabulary (migration: `syntax::DiagnosticKind` is removed in favour of)
- *(pipeline)* [**breaking**] split Diagnostic::new into required fields plus setters (migration: `Diagnostic::new` now takes `(severity, message)`; chain
`with_span(span)` and `with_code(code)` for the optional fields.)
- *(pipeline)* relocate ErrorStrategy next to the shared vocabulary
## [0.0.3](https://github.com/brightmeows/bmsrs/compare/bmsrs-pipeline-v0.0.2...bmsrs-pipeline-v0.0.3) - 2026-11-22

### Documentation

- record format provenance and tighten the sidecar API
- *(readme)* flip install instructions to the released 0.0.2

### Features

- [**breaking**] add BMD synthesis and source-format detection (migration: Bms gains format; Audio gains wav_sources; BmsKaleid gains synth; ParseOptions gains format.)
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-pipeline-v0.0.1...bmsrs-pipeline-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-pipeline-v0.0.1) - 2026-11-07

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- *(readme)* translate README to English and add a Chinese guide
- *(readme)* 新增根 README

### Features

- *(pipeline)* add shared outcome and diagnostic types ([#55](https://github.com/brightmeows/bmsrs/pull/55))
- *(facade)* 增加字节入口与统一诊断出口

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD

### Refactoring

- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
