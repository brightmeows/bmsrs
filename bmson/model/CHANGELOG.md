# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-bmson-model-v0.0.1...bmsrs-bmson-model-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1

### Other

- *(bmson-model)* drop the chumsky-based bmson_parser example
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bmson-model-v0.0.1) - 2026-11-07

### Bug Fixes

- *(bmson-model)* preserve explicit ln hints across conversions

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- *(readme)* translate README to English and add a Chinese guide
- *(readme)* 新增根 README

### Features

- *(chart)* [**breaking**] represent unspecified long-note type hint as None (migration: `ChartData::ln_type_hint()` returns
`Option<LnTypeHint>`; `with_ln_type_hint` takes `Option<LnTypeHint>`;)
- *(bmson-processor)* [**breaking**] fall back resolution 0 to 240 per spec (migration: deserialize_resolution_nonzero is replaced by
deserialize_resolution, which keeps 0; BmsonProcessWarning gains
ZeroResolutionFallback.)
- *(facade)* 增加字节入口与统一诊断出口

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD

### Refactoring

- refactor!(facade): name the default branch selector and polish prelude
- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
