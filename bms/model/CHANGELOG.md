# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-bms-model-v0.0.1...bmsrs-bms-model-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1

### Refactoring

- *(bms-model)* [**breaking**] stop re-exporting the values crate wholesale (migration: import values types from bmsrs-bms-values (or)
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bms-model-v0.0.1) - 2026-11-07

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- *(readme)* translate README to English and add a Chinese guide
- *(readme)* 新增根 README

### Features

- *(bms-processor)* [**breaking**] model SWBGA definitions and fix A5 key lookup (migration: BmsCustomEvent::BgaKeyBound changes from
{ resource_id: BgaResourceId } to { swbga_id: SwBgaResourceId };
ChartInfo gains swbga_resources.)
- *(facade)* 增加字节入口与统一诊断出口

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD

### Refactoring

- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
