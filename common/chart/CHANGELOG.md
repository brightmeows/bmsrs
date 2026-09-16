# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-chart-v0.1.0) - 2026-09-16

### Bug Fixes

- 最终审查修复——benchmark 去重、PMS 检测方法化、TimingCache 排序优化、warning 字段清理
- *(chart)* T2 review——TimingCache 委托 TimingTrack + 清除不可达 arm
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据
- *(chart)* 统一负 BPM 校验为非零有限值
- *(player)* Player::new 运行时验证 resolution 和 BPM，自动排序事件
- *(chart)* tick_to_duration 钳位超出 Duration::MAX 的秒数
- *(chart)* 跳过无效 BPM 变更值（0/NaN/Inf）
- *(chart)* TimingCache::new 内部排序 bpm_changes
- *(player)* 修正复杂度标注并添加 TimingCache Debug
- *(bmson-processor)* process_default 对异常 keys 报错，保留 DJ.NEXT judge/life deltas
- *(bmson-def,processor)* 统一 LN 类型模型，移除冗余 ln_mode 字段
- *(bms-processor,def,tokenizer,facade)* 代码审查问题修复
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- *(chart)* TimingTrack::default 使用安全的 init_bpm=120 而非 0.0
- *(bms-parser)* [**breaking**] implement position-based channel merge and case-insensitive BmsIndex

### Documentation

- *(agents)* 记录归一化契约、延迟字段与 IR 限制
- *(agents)* 修正 AGENTS.md 与源码不一致之处
- 添加目录级 AGENTS.md 并精简子 crate
- 同步 AGENTS.md 反映近期 API 变更和新增 hook
- *(chart)* 明确 TimingTrack 与 TimingCache 的职责分工
- *(chart)* 补充 TimingCache 与 Event::sort_key 说明
- *(chart)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容

### Features

- *(chart)* TimingTrack 初始 BPM 校验
- *(chart)* 添加 EventKind::map_custom/map_ext 与 filter_map_events
- *(chart)* 添加 ChartData::map_events + Chart::map_events
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(chart)* 添加 TimingTrack::simple(bpm) 便利构造器
- *(bms-processor)* 实现 BmsCustomEvent，保留丢弃的引擎特定通道
- 支持负 BPM（逆走）与负 STOP 钳位
- *(chart)* add Event::Speed variant for visual note-spacing keyframes

### Performance

- *(chart)* TimingCache::duration_to_tick 由 O(log² n) 降为 O(log n)
- *(player)* advance/seek 改用 TimingCache 快路径
- *(chart)* [**breaking**] AudioAsset.path 改用 Arc<Path> 共享所有权
- *(chart)* TimingTrack.build_events() 惰性缓存

### Refactoring

- 消费点迁移至新 TimingTrack API，TimingCache 标记弃用
- *(chart)* TimingTrack 分段化——O(log n) 查询，合并 TimingCache 功能
- *(core)* Rust 化改进——提取 build_sorted_events, 元组排序, HashMap/HashSet, 直解构, .ok().ok_or()→map_err, if+unwrap_or→match, for→find_map
- *(chart,processor)* 引入 BpmLookup 结构体消除 bpm_at_tick 自由函数
- *(chart,processor)* BPM 校验集中化到 TimingTrack::validate
- *(tokenizer)* [**breaking**] 封装 ChannelIndex/BmsMessage 字段，ChartData 增加 sort_events
- *(chart)* [**breaking**] 封装 TimingTrack 的 pub 字段以保护缓存不变量
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- *(bmson-processor)* BmsonNoteExt ln_* 字段改用 typed enum 而非 String
- *(chart)* [**breaking**] 移除 ChartData/Chart 的非法 Default
- *(chart)* 将 TimingCache 从 player 下沉至 chart
- *(chart)* 新增 Event::sort_key 收敛同脉冲排序约定
- *(chart)* 事件排序显式使用 (tick, priority) 元组
- *(chart)* [**breaking**] NoteKind::Mine f64 → Damage newtype，开启 Eq 派生
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- *(chart)* [**breaking**] rename NoteSide::ONE/TWO to P1/P2
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype
- enable eight additional clippy lints
- elevate warn-by-default clippy groups to deny and add restriction lints
- enable additional clippy restriction/nursery lints
- [**breaking**] move layout mapping to processor crates, rename Bme to Beat on BMSON side
- *(chart)* [**breaking**] add BmsChannel type for type-safe BMS channel parameter in BmsLayout
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model
- adopt stricter lint config from bms-rs
- rename core/ to common/ and move player/ under common/

### Testing

- 测试函数名补齐期望后缀
- *(chart)* TimingTrack 极端值测试
- *(bmsrs-chart)* 负 BPM roundtrip 测试（A12）
- *(player)* 添加 InvalidBpm panic 测试覆盖缺失的验证路径
- 合并重复测试文件，清理 bmspec 英文草稿注释
- *(bmsrs-chart)* 添加核心数据模型集成测试
