# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bms-writer-v0.1.0) - 2026-10-31

### Bug Fixes

- *(bms-writer)* 复用共享合并规则

### Documentation

- *(readme)* translate README to English and add a Chinese guide
- *(bms-writer)* translate comments and docs to English
- *(readme)* 新增根 README

### Features

- *(facade)* 增加字节入口与统一诊断出口
- *(bmsedit)* 谱面编辑 CLI
- *(writer)* Bms 模型全量重写生成器

### Refactoring

- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(bmsrs)* [**breaking**] 删除旧解析兼容层
- *(parser)* 类型化单行语法迁入 bms-parser
