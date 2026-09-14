# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bmson-processor-v0.1.0) - 2026-09-14

### Bug Fixes

- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据
- *(chart)* 统一负 BPM 校验为非零有限值
- *(bmson-processor)* 每通道独立 BGM discard，不再使用全局 playable_pulses 集合
- *(bmson-processor)* process_default 对异常 keys 报错，保留 DJ.NEXT judge/life deltas
- *(bmson-def,processor)* 统一 LN 类型模型，移除冗余 ln_mode 字段
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- *(bmson-processor)* 从完整事件集计算小节线末尾脉冲
- *(bmson-processor)* respect continuation flag in slicing algorithm

### Documentation

- *(agents)* 修正二轮审计发现的三处 API 不一致
- *(agents)* 修正 AGENTS.md 与源码不一致之处
- 添加目录级 AGENTS.md 并精简子 crate
- *(bmson-processor)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容
- add AGENTS.md for new crates and update root crate table

### Features

- *(processor)* 新增 process_with_warnings 结构化 warning 收集
- *(chart)* TimingTrack 初始 BPM 校验
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(chart)* 添加 TimingTrack::simple(bpm) 便利构造器
- 支持负 BPM（逆走）与负 STOP 钳位
- *(bmson-processor)* implement BMSON to Chart conversion
- add workspace metadata, lints, clippy and deny configs
- add workspace structure with empty lib crates

### Other

- 修复公开工具函数的 clippy lint
- set MSRV 1.85, reorganize clippy lints by category, add expect_used lint
- add package metadata for crates.io publishing

### Performance

- *(chart)* [**breaking**] AudioAsset.path 改用 Arc<Path> 共享所有权
- *(chart)* TimingTrack.build_events() 惰性缓存

### Refactoring

- 消费点迁移至新 TimingTrack API，TimingCache 标记弃用
- *(core)* Rust 化改进——提取 build_sorted_events, 元组排序, HashMap/HashSet, 直解构, .ok().ok_or()→map_err, if+unwrap_or→match, for→find_map
- *(processor)* 两个 processor 统一通过 ChartData::sort_events() 排序事件
- *(chart,processor)* BPM 校验集中化到 TimingTrack::validate
- *(bmson-processor)* [**breaking**] 引入 BmsonConverter struct，消除自由函数 &mut 参数
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- *(bmson-processor)* BmsonNoteExt ln_* 字段改用 typed enum 而非 String
- *(bmson-processor)* 移除 slice_channel 冗余排序
- *(bmson-processor)* slice_channel 改用 TimingCache 加速切片
- *(chart)* 新增 Event::sort_key 收敛同脉冲排序约定
- *(chart)* 事件排序显式使用 (tick, priority) 元组
- *(chart)* [**breaking**] NoteKind::Mine f64 → Damage newtype，开启 Eq 派生
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- *(chart)* [**breaking**] rename NoteSide::ONE/TWO to P1/P2
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype
- elevate warn-by-default clippy groups to deny and add restriction lints
- [**breaking**] move layout mapping to processor crates, rename Bme to Beat on BMSON side
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model
- adopt stricter lint config from bms-rs
- *(chart,player,bms-processor,bmson-processor)* [**breaking**] use Duration instead of f64 for time domain API

### Testing

- *(bmson-processor)* 地雷通道伤害值测试
- *(processor)* BMSON 反向滚动事件和 key_channels 不可见音符测试
- *(bmson-processor)* 添加 BMSON 处理器集成测试
- move public-API unit tests to integration test directories
