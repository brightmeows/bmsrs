# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.4](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-analysis-v0.0.3...bmsrs-bms-analysis-v0.0.4) - 2026-11-28

### Documentation

- correct stale passages found in the full read
- prune AGENTS.md and co-locate the boundary registers
- refresh the release version and record the visibility rules

### Features

- *(facade)* [**breaking**] accept Input on the BMSON entry points (migration: bmson::ParseError::Utf8 is replaced by Decode(TextDecodeError) and the enum is non_exhaustive with a new Format variant; SourceFormat::from_path now maps .json to Bmson.)

### Refactoring

- *(parser,analysis)* [**breaking**] name layer diagnostics after their module (migration: BmsDiagnostic and BmsDiagnosticKind are renamed Diagnostic and DiagnosticKind; AnalysisDiagnostic is renamed Diagnostic; prelude::Diagnostic is now the parser wrapper, import bmsrs::pipeline::Diagnostic explicitly for the shared vocabulary.)
- *(facade)* [**breaking**] collapse the parse entry points behind Input (migration: `bms::parse_bytes`, `parse_bytes_with`, and
`parse_file` are removed; build an `Input` (`Input::text(text, format)`,)
- *(parser)* [**breaking**] carry severity through the parse pipeline (migration: `BmsParseWarning` / `BmsParseWarningKind` are
`BmsDiagnostic` / `BmsDiagnosticKind` with a `severity` field and the
`Syntax` variant renamed `Cst`; `FailFast` fails only on error-severity
diagnostics. The facade's `FromTextError`, `FromTextWarning`, and
`ReplayParseError` are replaced by `ParseError { Diagnostic(BmsDiagnostic),
Branch(BranchMismatch), Io(io::Error) }`; every `bms::parse*` entry
returns `Result<Outcome<Bms, BmsDiagnostic>, ParseError>`.)
- *(pipeline)* [**breaking**] make DiagnosticCode the only diagnostic vocabulary (migration: `syntax::DiagnosticKind` is removed in favour of)
## [0.0.3](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-analysis-v0.0.2...bmsrs-bms-analysis-v0.0.3) - 2026-11-22

### Bug Fixes

- *(source,analysis)* wide-encoding terminators and marker indexing

### Documentation

- record format provenance and tighten the sidecar API
- consolidate AGENTS.md toward single sources of truth
- *(analysis)* align the dialect reference with the shipped manual
- *(analysis)* record the KaleidBeat dialect's provenance
- *(analysis)* point every KaleidBeat entry at the contract document
- *(analysis)* document KaleidBeat channels and tighten the AGENTS note
- *(readme)* flip install instructions to the released 0.0.2

### Features

- [**breaking**] preserve KaleidBeat conditional blocks (migration: Bms gains conditional blocks; BmsKaleid gains conditional; BmsHeader gains Conditional.)
- [**breaking**] add the KaleidBeat declaration and object sidecar (migration: BmsChannel gains Extension; Processed gains kaleid; Gameplay.option and BmsGameplay.option become Vec<OptionWords>.)
- feat!(processor): support CNOBJ/HCNOBJ/HLNOBJ typed LN markers
- *(analysis)* recognize KaleidBeat dialect commands and channels
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-analysis-v0.0.1...bmsrs-bms-analysis-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1

### Refactoring

- *(bms-analysis)* share the pipeline Severity type
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bms-analysis-v0.0.1) - 2026-11-07

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- *(bms-analysis)* point the docs table at the BMS command memo
- *(agents)* drop personal skill references and fix the terminology rule
- *(config)* translate crate descriptions and dependency comments
- *(readme)* translate README to English and add a Chinese guide
- *(bms-analysis)* translate comments, diagnostics, and editor docs to English
- *(agents)* 收口测试规则与已知限制
- *(readme)* 新增根 README

### Features

- *(facade)* 增加字节入口与统一诊断出口
- *(bmsrs)* 补回 bmsedit 所需的兼容 API
- *(analysis)* 新增诊断、符号索引与命令文档

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- 白小喵 <noreply@pi.dev>
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD

### Refactoring

- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(bms-values)* 新增 BodyCells 统一切块
- *(lsp)* 拆出 bms-lsp 到独立仓库
- *(values)* 抽出 bms-values 单行语法层
- *(parser)* 类型化单行语法迁入 bms-parser

### Testing

- *(bms-analysis)* 锁定资源定义表与 values header 一致
