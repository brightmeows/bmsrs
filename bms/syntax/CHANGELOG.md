# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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
