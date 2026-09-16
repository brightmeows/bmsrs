# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bmson-def-v0.1.0) - 2026-09-16

### Bug Fixes

- *(bmson-def)* BMSON version 字段 SemVer 前缀校验
- *(bmson-def)* 版本检测扫描跟踪花括号深度，忽略嵌套 version 键
- *(bmson-def)* v1→root 回填 per-note ln_type_hint 对齐 v0 路径
- *(bmson-def,processor)* 统一 LN 类型模型，移除冗余 ln_mode 字段
- *(bms-processor,def,tokenizer,facade)* 代码审查问题修复
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- replace confusable Unicode dashes/spaces, add clippy rules and pre-commit hook
- *(bmson-def)* spec compliance fixes, missing fields, and test migration

### CI

- extend clippy to all-targets and fix test code lint errors

### Documentation

- *(agents)* 修正二轮审计发现的三处 API 不一致
- 添加目录级 AGENTS.md 并精简子 crate
- *(bmson-def)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容
- move bmson-def API conventions to crate-level AGENTS.md

### Features

- upgrade MSRV to 1.88, add itertools/serde_with/insta/proptest, apply across codebase
- enable clippy::missing_docs_in_private_items lint
- *(bmson-def)* implement multi-version bmson type definitions
- add workspace metadata, lints, clippy and deny configs
- add workspace structure with empty lib crates

### Other

- *(parser,bmson-def)* 引号反引号化 + depth u8→u32
- set MSRV 1.85, reorganize clippy lints by category, add expect_used lint
- add package metadata for crates.io publishing
- remove all //===== section divider comments

### Refactoring

- *(bmson)* [**breaking**] 将 bmson-de-chumsky 合并为 bmson-def 的示例，不再作为独立 crate 导出
- *(bmson-def,de-chumsky)* [**breaking**] 公开自由函数挂到类型
- elevate warn-by-default clippy groups to deny and add restriction lints
- enable additional clippy restriction/nursery lints
- adopt stricter lint config from bms-rs
- remove decorative divider comments, add bmson version detection
- *(bmson-def)* migrate key string/path fields to borrowed references
- *(bmson-def)* migrate path fields to PathBuf, add Eq/Copy, improve error handling
- trim obvious content from AGENTS.md files
- *(bmson-def)* move V0LnType to common.rs, rename to LnMode
- *(bmson-def)* [**breaking**] replace X enum with raw u64 for NoteEvent.x

### Testing

- 测试函数名补齐期望后缀
- *(bmson-def)* 版本检测边缘用例
- *(bmson-def)* 添加版本检测、serde 往返与 v0/v1 转换测试
