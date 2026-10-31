# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bms-values-v0.1.0) - 2026-10-31

### Documentation

- *(config)* translate crate descriptions and dependency comments
- *(readme)* translate README to English and add a Chinese guide
- *(bms-values)* remove the last CJK example from PLAYLEVEL docs
- *(bms-values)* translate comments and docs to English
- *(readme)* 新增根 README

### Features

- *(facade)* 增加字节入口与统一诊断出口

### Refactoring

- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(bms-values)* 新增 BodyCells 统一切块
- *(values)* 抽出 bms-values 单行语法层
