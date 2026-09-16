# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bms-parser-v0.1.0) - 2026-09-16

### Bug Fixes

- *(parser)* decode_mine_raw 地雷值始终用 base36（不受 #BASE 62 影响）
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据
- *(tokenizer)* 行内 // 注释要求前置空白，避免 URL 被截断
- *(bms-parser)* 保留长音事件中的 "00" 条目
- *(bms-parser)* 非 BGM 通道过滤 "00" 值
- *(bms-parser)* finalize 清空 non_event_data 保证幂等
- *(bms-processor,def,tokenizer,facade)* 代码审查问题修复
- *(test)* 更新 Extended BPM 与 speed interpolation 测试断言
- *(bms-parser)* Extended BPM 通道 08 中 "00" 应作为休止跳过
- *(tokenizer)* [**breaking**] BmsIndex case-sensitive comparison for Base62 support
- *(parser,processor)* measure length ratio, mine damage, Base62 normalization, SPEED events
- *(bms-parser, bms-processor)* filter BPM 00 rest, add LNOBJ BGM playback
- *(bms-parser)* [**breaking**] implement position-based channel merge and case-insensitive BmsIndex
- *(bms-parser)* correct BGA channel routing for 0A (Layer2) and 05 (Seek)
- *(bms-parser)* use lenient value splitting and fix multi-line event positions

### CI

- extend clippy to all-targets and fix test code lint errors

### Documentation

- *(agents)* 固化三层职责判据与 Bms 保真原则
- *(parser,processor)* 同步 non_event_data 归一化契约变更到 AGENTS.md 和 API 文档
- *(agents)* 记录归一化契约、延迟字段与 IR 限制
- 添加目录级 AGENTS.md 并精简子 crate
- *(tokenizer,parser)* 补充 #WAVCMD 完整规格文档
- *(parser)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容
- *(parser)* document information-preservation philosophy (no implicit subtitle)
- *(bms-parser)* add design goals section to AGENTS.md

### Features

- *(bms)* 支持扩展音符通道（1A-1Z 等）
- *(tokenizer)* 结构化解析 #WAVCMD（WavCmdParams）
- *(bms-parser)* 隐式副标题分割（D5）
- *(tokenizer,parser)* 行内注释处理与 Bms::from_text 便捷方法
- *(bms-processor)* 实现 BmsCustomEvent，保留丢弃的引擎特定通道
- upgrade MSRV to 1.88, add itertools/serde_with/insta/proptest, apply across codebase
- *(bms-processor)* implement LNTYPE 2 (MGQ) and fix RDM 00 filtering
- *(bms-tokenizer)* add BmsChannel enum for channel classification at tokenizer level
- *(bms-tokenizer, bms-tokenizer-derive, bms-control-flow, bms-parser)* [**breaking**] add generic string container parameter C to all types
- *(bms-parser)* [**breaking**] restructure Bms with categorized sub-structs and full token storage
- add bms-control-flow and bms-parser crates, update facade
- add workspace metadata, lints, clippy and deny configs
- add workspace structure with empty lib crates

### Other

- *(parser,bmson-def)* 引号反引号化 + depth u8→u32
- 修复公开工具函数的 clippy lint
- set MSRV 1.85, reorganize clippy lints by category, add expect_used lint
- add package metadata for crates.io publishing

### Performance

- 稠密消息优化——流式 NonZeroChunks 迭代器 + 相邻同值 BPM 去重

### Refactoring

- *(tokenizer)* [**breaking**] 移除 C 泛型参数，字符串字段统一使用 String
- *(parser)* 移除 norm_as! 宏，直接内联 BmpIndex::from(id.normalize(base))
- *(parser)* [**breaking**] Position 字段私有化，提供访问器，移除防御性检查
- *(tokenizer)* [**breaking**] base36_digit_value 提升为 pub，消除 parser 本地副本
- *(bms)* [**breaking**] NonEventData.values 从 Vec<String> 改为 Vec<BmsIndex>
- *(parser)* [**breaking**] Position::new 强制 denom > 0 不变式
- *(tokenizer)* [**breaking**] ExWavParams 结构化 flags+values → pan/volume/frequency
- *(tokenizer)* [**breaking**] WavCmdParams 枚举化 command_id，wav_index 改用 WavIndex
- *(parser)* [**breaking**] 移除 Bms.detected_base 字段
- *(bms)* [**breaking**] MineEvent 改存 raw_value: Option<u16>，伤害计算移至 processor
- *(core)* Rust 化改进——提取 build_sorted_events, 元组排序, HashMap/HashSet, 直解构, .ok().ok_or()→map_err, if+unwrap_or→match, for→find_map
- *(parser)* 用 iter_nonzero_chunks 消除通道消息分派的重复拆分+过滤
- *(parser)* [**breaking**] 在 parser 层归一化 non_event_data 索引值，消除 processor 的重复归一化
- *(tokenizer,parser)* WavCmdParams.value f64→u32 + from_text 返回 warnings
- *(player,parser)* 恢复排序诊断 + 更新 parser 契约
- *(parser,processor)* 将 non_event_data 改为类型安全 NonEventData 结构体
- *(tokenizer)* [**breaking**] 封装 ChannelIndex/BmsMessage 字段，ChartData 增加 sort_events
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- *(tokenizer,parser)* 用 expect 取代静默回退值，修复 clippy semicolon 警告
- *(tokenizer)* 将 is_base62/base36_decode 等绑定到 BmsBase 方法
- *(parser)* 消除 messages.rs 中所有模块级 #![allow]/#![expect]
- *(parser)* 提取 finalize 通道分派逻辑到独立方法消除 too_many_lines
- *(tokenizer,parser)* 收拢 base36_decode 等公共工具函数到 tokenizer 并公开导出
- move public-API tests from src/ to tests/
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- forbid unsafe and remove the last unsafe blocks
- *(bms-tokenizer)* [**breaking**] replace BmsIndex generic+tag with newtype+Deref pattern
- enable additional clippy restriction/nursery lints
- adopt stricter lint config from bms-rs
- [**breaking**] remove 'a lifetime from token types, remove BmsStr trait
- *(bms-tokenizer, bms-parser)* [**breaking**] split concat_and_parse into two-phase concat_raw + finalize
- *(bms-tokenizer,bms-parser)* [**breaking**] rename BmsChannelId to BmsIndex, restructure BmsMessage
- *(workspace)* move all local path deps to [workspace.dependencies]

### Testing

- 测试函数名补齐期望后缀
- *(parser)* non_event_data 合并行为与 merge_channel 多分辨率集成测试
- *(bms-parser)* 添加 Base62 消息事件大小写敏感索引的集成测试
