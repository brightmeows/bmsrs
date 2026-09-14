# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bms-tokenizer-v0.1.0) - 2026-09-14

### Bug Fixes

- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据
- *(tokenizer)* 行内 // 注释要求前置空白，避免 URL 被截断
- *(tokenizer)* preprocess 正确保留 UTF-8 多字节字符
- *(bms-processor,def,tokenizer,facade)* 代码审查问题修复
- *(tokenizer)* [**breaking**] BmsIndex case-sensitive comparison for Base62 support
- *(bms-parser)* [**breaking**] implement position-based channel merge and case-insensitive BmsIndex
- replace confusable Unicode dashes/spaces, add clippy rules and pre-commit hook
- *(bms-tokenizer)* correct StpParams position range, enrich semantic docs

### CI

- extend clippy to all-targets and fix test code lint errors
- treat warnings as errors in pre-commit clippy and doc hooks

### Documentation

- *(tokenizer)* 更新值约束对应字段的文档注释
- *(agents)* 固化三层职责判据与 Bms 保真原则
- *(agents)* 修正 AGENTS.md 与源码不一致之处
- 添加目录级 AGENTS.md 并精简子 crate
- *(tokenizer,parser)* 补充 #WAVCMD 完整规格文档
- *(tokenizer)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容
- update AGENTS.md type references after lifetime removal

### Features

- *(tokenizer-derive)* 新增 #[derive(BmsIndexNewtype)] proc-macro 简化 12 个索引 newtype
- *(tokenizer)* WavCmdParams pitch 范围 0–127 校验
- *(tokenizer)* StpParams measure 范围 0–999 校验
- *(tokenizer)* ExWavParams 参数范围校验
- *(tokenizer)* 新增编码检测与转换模块
- *(tokenizer)* 结构化解析 #WAVCMD（WavCmdParams）
- *(tokenizer,parser)* 行内注释处理与 Bms::from_text 便捷方法
- *(tokenizer)* add preprocess() for BMS comment stripping
- upgrade MSRV to 1.88, add itertools/serde_with/insta/proptest, apply across codebase
- *(bms-tokenizer)* add BmsChannel enum for channel classification at tokenizer level
- *(bms-tokenizer, bms-tokenizer-derive, bms-control-flow, bms-parser)* [**breaking**] add generic string container parameter C to all types
- add bms-control-flow and bms-parser crates, update facade
- *(bms-tokenizer)* add From/TryFrom conversions between token types
- extend pre-commit hook to catch decorative comment patterns
- *(bms-tokenizer)* [**breaking**] rewrite tokenizer with typed params, unified BmsValue, and single derive
- replace hand-written header match with proc-macro derive
- add bms/tokenizer/AGENTS.md
- enable clippy::missing_docs_in_private_items lint
- add serde support to workspace and bms-tokenizer types
- *(bms-tokenizer)* implement BMS tokenizer
- add workspace metadata, lints, clippy and deny configs
- add workspace structure with empty lib crates

### Other

- *(tokenizer)* 合并 many_single_char_names 为模块级 expect
- 修复公开工具函数的 clippy lint
- set MSRV 1.85, reorganize clippy lints by category, add expect_used lint
- add package metadata for crates.io publishing

### Performance

- *(bms-tokenizer)* eliminate allocations and use eq_ignore_ascii_case in derive macro

### Refactoring

- *(tokenizer)* [**breaking**] 移除 C 泛型参数，字符串字段统一使用 String
- *(tokenizer)* 启用 derive_more::try_unwrap 消除 8 个 TryFrom impl
- *(tokenizer)* [**breaking**] base36_digit_value 提升为 pub，消除 parser 本地副本
- *(tokenizer)* [**breaking**] ExWavParams 结构化 flags+values → pan/volume/frequency
- *(tokenizer)* [**breaking**] WavCmdParams 枚举化 command_id，wav_index 改用 WavIndex
- *(tokenizer,parser)* WavCmdParams.value f64→u32 + from_text 返回 warnings
- *(tokenizer)* detect_encoding 移为 BmsEncoding::detect 关联方法
- *(tokenizer)* [**breaking**] 封装 ChannelIndex/BmsMessage 字段，ChartData 增加 sort_events
- *(tokenizer)* [**breaking**] #SKIP 改为无参数符合 BMS 规范
- *(bmsrs,tokenizer)* 消除门面模块重复导出，增加 tokenize_owned 便捷方法
- *(tokenizer,parser)* 用 expect 取代静默回退值，修复 clippy semicolon 警告
- 用切片模式取代大部分 indexing_slicing expect
- *(tokenizer)* 将 is_base62/base36_decode 等绑定到 BmsBase 方法
- *(parser)* 消除 messages.rs 中所有模块级 #![allow]/#![expect]
- *(tokenizer,parser)* 收拢 base36_decode 等公共工具函数到 tokenizer 并公开导出
- move public-API tests from src/ to tests/
- forbid unsafe and remove the last unsafe blocks
- *(bms-tokenizer)* [**breaking**] replace BmsIndex generic+tag with newtype+Deref pattern
- elevate warn-by-default clippy groups to deny and add restriction lints
- enable additional clippy restriction/nursery lints
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model
- eliminate expect attributes via better implementations
- adopt stricter lint config from bms-rs
- *(bms-tokenizer)* make error types generic over C instead of 'a
- [**breaking**] remove 'a lifetime from token types, remove BmsStr trait
- *(bms-tokenizer, bms-parser)* [**breaking**] split concat_and_parse into two-phase concat_raw + finalize
- *(bms-tokenizer,bms-parser)* [**breaking**] rename BmsChannelId to BmsIndex, restructure BmsMessage
- *(workspace)* move all local path deps to [workspace.dependencies]
- *(bms-tokenizer-derive)* deduplicate field parse codegen, add derive macro tests
- *(bms-tokenizer-derive)* [**breaking**] replace {value} with {} for unnamed fields, add prefix config
- *(bms-tokenizer)* #BASE → enum, add control flow typos, reorganize AGENTS.md
- *(bms-tokenizer)* enrich docs with semantic meaning/usage/caveats, fix indexed #TEXT/#SONG/#CHANGEOPTION
- *(bms-tokenizer)* [**breaking**] unify error model with IntoTokensError, remove all error-related attributes
- *(bms-tokenizer)* [**breaking**] replace bare Rank(u8) with enum, unify #STP into derive flow
- *(bms-tokenizer)* [**breaking**] replace free functions with BmsTokenizer builder
- remove decorative divider comments, add bmson version detection
- categorize errors and use thiserror throughout
- type header values and unify error handling
- *(bms-tokenizer)* split BmsHeader sub-enums into dedicated submodules

### Testing

- 测试函数名补齐期望后缀
- *(tokenizer)* BGA 裁剪坐标边界测试
