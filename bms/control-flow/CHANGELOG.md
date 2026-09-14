# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bms-control-flow-v0.1.0) - 2026-09-14

### Bug Fixes

- *(control-flow)* try_map_payload 继承原始 FlowDoc 的 warnings
- *(control-flow)* [**breaking**] 未闭合 #RANDOM/#SWITCH 块内容提升到顶层而非丢弃
- *(bms-control-flow)* avoid panic on #RANDOM 0 / #SWITCH 0

### CI

- extend clippy to all-targets and fix test code lint errors
- treat warnings as errors in pre-commit clippy and doc hooks

### Documentation

- 添加目录级 AGENTS.md 并精简子 crate
- *(control-flow)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容

### Features

- upgrade MSRV to 1.88, add itertools/serde_with/insta/proptest, apply across codebase
- *(bms-tokenizer, bms-tokenizer-derive, bms-control-flow, bms-parser)* [**breaking**] add generic string container parameter C to all types
- add bms-control-flow and bms-parser crates, update facade

### Refactoring

- *(tokenizer)* [**breaking**] 移除 C 泛型参数，字符串字段统一使用 String
- *(control-flow)* [**breaking**] #IF-#ELSEIF-#ELSE 重构为互斥链模型
- *(tokenizer)* [**breaking**] #SKIP 改为无参数符合 BMS 规范
- elevate warn-by-default clippy groups to deny and add restriction lints
- enable additional clippy restriction/nursery lints
- adopt stricter lint config from bms-rs
- *(bms-control-flow)* [**breaking**] replace DeterministicRng with rand::RngExt blanket impl
- *(bms-control-flow)* rename FlowTree -> FlowDoc
- *(bms-control-flow)* use tuple struct + Deref for FlowTree<P>
- *(bms-control-flow)* [**breaking**] redesign as generic FlowTree<P>
- [**breaking**] remove 'a lifetime from token types, remove BmsStr trait
- *(workspace)* move all local path deps to [workspace.dependencies]

### Testing

- 测试函数名补齐期望后缀
- *(bms-control-flow)* 添加空 #SWITCH 块的集成测试
- *(bms-control-flow)* 添加深层嵌套 #RANDOM / #SWITCH 的集成测试
