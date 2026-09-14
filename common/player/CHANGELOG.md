# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-player-v0.1.0) - 2026-09-14

### Bug Fixes

- 最终审查修复——benchmark 去重、PMS 检测方法化、TimingCache 排序优化、warning 字段清理
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据
- *(chart)* 统一负 BPM 校验为非零有限值
- *(player)* Player::new 运行时验证 resolution 和 BPM，自动排序事件
- *(player)* 修正复杂度标注并添加 TimingCache Debug
- *(bmson-processor)* process_default 对异常 keys 报错，保留 DJ.NEXT judge/life deltas
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- *(player)* SpeedCache 同脉冲关键帧插值除零守护
- *(test)* 更新 Extended BPM 与 speed interpolation 测试断言
- *(bms-parser)* [**breaking**] implement position-based channel merge and case-insensitive BmsIndex

### Documentation

- *(agents)* 修正二轮审计发现的三处 API 不一致
- 添加目录级 AGENTS.md 并精简子 crate
- 记录新增的热路径基准基础设施
- *(player)* 同步 TimingCache 下沉至 chart 的现状
- *(player)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容

### Features

- *(chart)* TimingTrack 初始 BPM 校验
- *(player)* 添加 visible_tick_range 便捷方法
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(chart)* 添加 TimingTrack::simple(bpm) 便利构造器
- *(player)* 添加滚动位置累积与 SPEED 间距插值
- 支持负 BPM（逆走）与负 STOP 钳位
- *(bms-processor)* implement LNTYPE 2 (MGQ) and fix RDM 00 filtering

### Other

- 更新 workspace 依赖至最新版本
- *(player)* 扩展 events_in_range 基准并覆盖 BGM 密集场景
- *(player)* 新增 criterion 热路径基准

### Performance

- *(player)* advance/seek 改用 TimingCache 快路径
- *(chart)* TimingTrack.build_events() 惰性缓存
- *(player)* 为 scroll_rate_at 添加预计算 ScrollCache

### Refactoring

- 消费点迁移至新 TimingTrack API，TimingCache 标记弃用
- *(player)* [**breaking**] Player::new 返回 Result，消除 panic 路径
- *(player,parser)* 恢复排序诊断 + 更新 parser 契约
- *(tokenizer)* [**breaking**] 封装 ChannelIndex/BmsMessage 字段，ChartData 增加 sort_events
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- 用切片模式取代大部分 indexing_slicing expect
- *(chart)* [**breaking**] 移除 ChartData/Chart 的非法 Default
- *(chart)* 将 TimingCache 从 player 下沉至 chart
- *(chart)* [**breaking**] NoteKind::Mine f64 → Damage newtype，开启 Eq 派生
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- *(chart)* [**breaking**] rename NoteSide::ONE/TWO to P1/P2
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype
- enable eight additional clippy lints
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model
- adopt stricter lint config from bms-rs
- rename core/ to common/ and move player/ under common/

### Testing

- 测试函数名补齐期望后缀
- *(player)* BGM 与 BGA 事件查询测试
- *(player)* 添加 InvalidBpm panic 测试覆盖缺失的验证路径
- 合并重复测试文件，清理 bmspec 英文草稿注释
- *(bmsrs-player)* 添加播放器仿真层集成测试
