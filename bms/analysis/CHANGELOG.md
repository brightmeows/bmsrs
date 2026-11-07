# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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
