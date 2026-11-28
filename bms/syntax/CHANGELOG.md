# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.4](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-syntax-v0.0.3...bmsrs-bms-syntax-v0.0.4) - 2026-11-28

### Bug Fixes

- *(facade)* honor FailFast on the replay entry point

### Documentation

- *(syntax)* rename select_lines_with in the API notes
- prune AGENTS.md and co-locate the boundary registers
- refresh the release version and record the visibility rules

### Features

- *(facade)* [**breaking**] accept Input on the BMSON entry points (migration: bmson::ParseError::Utf8 is replaced by Decode(TextDecodeError) and the enum is non_exhaustive with a new Format variant; SourceFormat::from_path now maps .json to Bmson.)

### Refactoring

- *(facade)* [**breaking**] collapse the parse entry points behind Input (migration: `bms::parse_bytes`, `parse_bytes_with`, and
`parse_file` are removed; build an `Input` (`Input::text(text, format)`,)
- *(parser)* [**breaking**] carry severity through the parse pipeline (migration: `BmsParseWarning` / `BmsParseWarningKind` are
`BmsDiagnostic` / `BmsDiagnosticKind` with a `severity` field and the
`Syntax` variant renamed `Cst`; `FailFast` fails only on error-severity
diagnostics. The facade's `FromTextError`, `FromTextWarning`, and
`ReplayParseError` are replaced by `ParseError { Diagnostic(BmsDiagnostic),
Branch(BranchMismatch), Io(io::Error) }`; every `bms::parse*` entry
returns `Result<Outcome<Bms, BmsDiagnostic>, ParseError>`.)
- *(parser)* [**breaking**] take ControlFlowRng in the parser and facade entry points (migration: `BmsParser::parse`, `parse_with`, `parse_document`, and
`parse_document_with` take `&mut R` (`R: ControlFlowRng`) instead of a
closure by value; pass `&mut |_| 1` or `&mut FirstBranch`. The facade's
`parse_with`, `parse_with_recording`, and `parse_bytes_with` follow
suit, `bms::parse_with_rng` is removed, and `ParseOptions::first_branch`
is replaced by `bmsrs_bms_syntax::FirstBranch` (re-exported as)
- *(syntax)* [**breaking**] move JavaRandom from the facade into the syntax crate (migration: `JavaRandom::choice` is gone; call)
- *(syntax)* [**breaking**] replace the select_lines_* family with select_lines and replay_lines (migration: `SourceFile::select_lines(next)` becomes
`select_lines(&mut rng, ControlFlowCompat::Ideal)` (a closure still
works through the `ControlFlowRng` blanket impl);
`select_lines_with_compat` and `select_lines_with_rng` merge into
`select_lines`; `select_lines_with(selection)` is renamed
`replay_lines(selection)`.)
- *(syntax)* [**breaking**] implement ControlFlowRng for closures and add FirstBranch (migration: the trait gained a blanket implementation over
closures, so a downstream `ControlFlowRng` impl for a closure type is
no longer possible.)
- *(syntax)* [**breaking**] move the first branch selector onto ParseOptions (migration: `bmsrs::bms::first_branch` is removed; call)
- *(pipeline)* [**breaking**] make DiagnosticCode the only diagnostic vocabulary (migration: `syntax::DiagnosticKind` is removed in favour of)
- [**breaking**] route error strategy through ParseOptions (migration: parse_with, parse_bytes_with, and
parse_with_recording lose the trailing ErrorStrategy parameter; set
ParseOptions.error_strategy instead. ErrorStrategy is re-exported
from the facade bms module as before.)
## [0.0.3](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-syntax-v0.0.2...bmsrs-bms-syntax-v0.0.3) - 2026-11-22

### Bug Fixes

- *(syntax)* iterate the MT state through a deref for MSRV 1.95

### Documentation

- record format provenance and tighten the sidecar API
- consolidate AGENTS.md toward single sources of truth
- move AGENTS-only details into rustdoc
- *(readme)* flip install instructions to the released 0.0.2

### Features

- [**breaking**] add BMD synthesis and source-format detection (migration: Bms gains format; Audio gains wav_sources; BmsKaleid gains synth; ParseOptions gains format.)
- [**breaking**] keep bmson total percent and add caller-probed asset checks (migration: ChartInfo gains a field, so struct literals need
total_percent; ChartInfo is non-exhaustive by convention (Default exists).)
- *(syntax)* standard control-flow RNGs and jbms compat mode
- *(syntax)* add JavaRandom and strict branch-selection replay
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-syntax-v0.0.1...bmsrs-bms-syntax-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bms-syntax-v0.0.1) - 2026-11-07

### Bug Fixes

- *(syntax)* 识别 IFEND/RONDAM/ENDSWITCH 别名

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- *(config)* translate crate descriptions and dependency comments
- *(readme)* translate README to English and add a Chinese guide
- *(bms-syntax)* translate comments and docs to English
- *(readme)* 新增根 README

### Features

- *(facade)* 增加字节入口与统一诊断出口
- *(bmsrs)* 补回 bmsedit 所需的兼容 API
- *(syntax)* 新增 rowan CST 与从 CST 构建 Bms

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD

### Refactoring

- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(bmsrs)* [**breaking**] 删除旧解析兼容层

### Testing

- *(bms-syntax)* 锁定 source 与 syntax 的行边界
