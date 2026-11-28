# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.4](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-writer-v0.0.3...bmsrs-bms-writer-v0.0.4) - 2026-11-28

### Documentation

- prune AGENTS.md and co-locate the boundary registers
- refresh the release version and record the visibility rules

### Features

- *(facade)* [**breaking**] accept Input on the BMSON entry points (migration: bmson::ParseError::Utf8 is replaced by Decode(TextDecodeError) and the enum is non_exhaustive with a new Format variant; SourceFormat::from_path now maps .json to Bmson.)

### Refactoring

- *(processor,writer,facade)* [**breaking**] name results and namespace types after their module (migration: BmsonProcessed, BmsonProcessWarning, and BmsonParseError lose their Bmson prefix; BmsProcessor and BmsonProcessor are Processor, BmsWriter is Writer, BmsLayout and BmsonLayout are Layout; prelude no longer exports the bmson Processed, ProcessWarning, ParseError, or Layout.)
- *(model)* [**breaking**] drop the Owned parameter duplicates (migration: OwnedExWavParams, OwnedExBmpParams, and OwnedSwBgaParams are removed; read ExWavParams, ExBmpParams, and SwBgaParams from bmsrs-bms-values instead.)
- *(facade)* [**breaking**] collapse the parse entry points behind Input (migration: `bms::parse_bytes`, `parse_bytes_with`, and
`parse_file` are removed; build an `Input` (`Input::text(text, format)`,)
- *(parser)* [**breaking**] take ControlFlowRng in the parser and facade entry points (migration: `BmsParser::parse`, `parse_with`, `parse_document`, and
`parse_document_with` take `&mut R` (`R: ControlFlowRng`) instead of a
closure by value; pass `&mut |_| 1` or `&mut FirstBranch`. The facade's
`parse_with`, `parse_with_recording`, and `parse_bytes_with` follow
suit, `bms::parse_with_rng` is removed, and `ParseOptions::first_branch`
is replaced by `bmsrs_bms_syntax::FirstBranch` (re-exported as)
## [0.0.3](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-writer-v0.0.2...bmsrs-bms-writer-v0.0.3) - 2026-11-22

### Bug Fixes

- *(release)* make internal dev-dependencies path-only

### Documentation

- record format provenance and tighten the sidecar API
- *(readme)* flip install instructions to the released 0.0.2

### Features

- [**breaking**] add BMD synthesis and source-format detection (migration: Bms gains format; Audio gains wav_sources; BmsKaleid gains synth; ParseOptions gains format.)
- [**breaking**] preserve KaleidBeat conditional blocks (migration: Bms gains conditional blocks; BmsKaleid gains conditional; BmsHeader gains Conditional.)
- [**breaking**] add the KaleidBeat declaration and object sidecar (migration: BmsChannel gains Extension; Processed gains kaleid; Gameplay.option and BmsGameplay.option become Vec<OptionWords>.)
- feat!(processor): support CNOBJ/HCNOBJ/HLNOBJ typed LN markers

### Refactoring

- [**breaking**] unify WAV sources behind WavSource (migration: Audio.wav_files is removed; Audio.wav_sources now stores WavSource values.)
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-writer-v0.0.1...bmsrs-bms-writer-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1

### Refactoring

- *(bms-model)* [**breaking**] stop re-exporting the values crate wholesale (migration: import values types from bmsrs-bms-values (or bmsrs::raw::values) instead of bmsrs-bms-model.)
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bms-writer-v0.0.1) - 2026-11-07

### Bug Fixes

- *(bms-writer)* 复用共享合并规则

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- *(readme)* translate README to English and add a Chinese guide
- *(bms-writer)* translate comments and docs to English
- *(readme)* 新增根 README

### Features

- *(facade)* 增加字节入口与统一诊断出口
- *(bmsedit)* 谱面编辑 CLI
- *(writer)* Bms 模型全量重写生成器

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD

### Refactoring

- *(bms-parser)* [**breaking**] make Messages a finalized builder product
- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(bmsrs)* [**breaking**] 删除旧解析兼容层
- *(parser)* 类型化单行语法迁入 bms-parser
