# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bms-processor-v0.1.0) - 2026-09-16

### Bug Fixes

- 最终审查修复——benchmark 去重、PMS 检测方法化、TimingCache 排序优化、warning 字段清理
- *(processor)* T1 review——更新测试名和 PairedLn 文档
- *(bms-processor)* 将 pair_lntype 分组改回 BTreeMap 消除测试非确定性
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据
- *(chart)* 统一负 BPM 校验为非零有限值
- *(processor)* position_to_tick 改用 u128 四舍五入
- *(bmson-processor)* process_default 对异常 keys 报错，保留 DJ.NEXT judge/life deltas
- *(bms-processor)* 修复 LNOBJ 下 ch51-69 静默丢失与 BGA resource_id 不一致
- *(bms-processor,def,tokenizer,facade)* 代码审查问题修复
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- *(processor)* build_bar_events 适配变拍号（#xxx02）小节线位置
- *(test)* 更新 Extended BPM 与 speed interpolation 测试断言
- *(bms-parser)* Extended BPM 通道 08 中 "00" 应作为休止跳过
- *(parser,processor)* measure length ratio, mine damage, Base62 normalization, SPEED events
- *(bms-parser, bms-processor)* filter BPM 00 rest, add LNOBJ BGM playback
- *(bms-parser)* [**breaking**] implement position-based channel merge and case-insensitive BmsIndex

### Documentation

- *(agents)* 固化三层职责判据与 Bms 保真原则
- *(parser,processor)* 同步 non_event_data 归一化契约变更到 AGENTS.md 和 API 文档
- *(agents)* 记录归一化契约、延迟字段与 IR 限制
- *(agents)* 修正 AGENTS.md 与源码不一致之处
- 添加目录级 AGENTS.md 并精简子 crate
- 同步 AGENTS.md 反映近期 API 变更和新增 hook
- *(bms-processor)* 同步 collect_* 归属 BmsConverter 的现状
- *(processor)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容
- align AGENTS.md with actual code
- add AGENTS.md for new crates and update root crate table

### Features

- *(bms)* 支持扩展音符通道（1A-1Z 等）
- *(processor)* PMS 变体自动检测（Standard / BME-type）
- *(processor)* 新增 process_with_warnings 结构化 warning 收集
- *(processor)* LN 区间内 visible note 移除（配对 + unpaired start）
- *(chart)* TimingTrack 初始 BPM 校验
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(bms-processor)* 实现 BmsCustomEvent，保留丢弃的引擎特定通道
- *(processor)* BANNER/BACKBMP/STAGEFILE/PREVIEW 映射至 ChartInfo
- *(player)* 添加滚动位置累积与 SPEED 间距插值
- 支持负 BPM（逆走）与负 STOP 钳位
- upgrade MSRV to 1.88, add itertools/serde_with/insta/proptest, apply across codebase
- *(bms-processor)* implement LNTYPE 2 (MGQ) and fix RDM 00 filtering
- *(bms-processor)* implement BMS to Chart conversion processor
- add workspace metadata, lints, clippy and deny configs
- add workspace structure with empty lib crates

### Other

- set MSRV 1.85, reorganize clippy lints by category, add expect_used lint
- add package metadata for crates.io publishing

### Performance

- 稠密消息优化——流式 NonZeroChunks 迭代器 + 相邻同值 BPM 去重
- *(chart)* [**breaking**] AudioAsset.path 改用 Arc<Path> 共享所有权
- *(chart)* TimingTrack.build_events() 惰性缓存

### Refactoring

- *(tokenizer)* [**breaking**] 移除 C 泛型参数，字符串字段统一使用 String
- *(parser)* [**breaking**] Position 字段私有化，提供访问器，移除防御性检查
- *(bms)* [**breaking**] NonEventData.values 从 Vec<String> 改为 Vec<BmsIndex>
- *(parser)* [**breaking**] 移除 Bms.detected_base 字段
- *(bms)* [**breaking**] MineEvent 改存 raw_value: Option<u16>，伤害计算移至 processor
- 消费点迁移至新 TimingTrack API，TimingCache 标记弃用
- *(core)* Rust 化改进——提取 build_sorted_events, 元组排序, HashMap/HashSet, 直解构, .ok().ok_or()→map_err, if+unwrap_or→match, for→find_map
- *(processor)* 两个 processor 统一通过 ChartData::sort_events() 排序事件
- *(chart,processor)* 引入 BpmLookup 结构体消除 bpm_at_tick 自由函数
- *(parser)* [**breaking**] 在 parser 层归一化 non_event_data 索引值，消除 processor 的重复归一化
- *(player)* [**breaking**] Player::new 返回 Result，消除 panic 路径
- *(chart,processor)* BPM 校验集中化到 TimingTrack::validate
- *(parser,processor)* 将 non_event_data 改为类型安全 NonEventData 结构体
- *(bms-processor)* bga_resources 构建内化到 build_metadata
- *(bms-processor)* stops 循环内化到 BmsConverter::collect_stop_events
- *(bms-processor)* [**breaking**] BmsConverter 内化 events 缓冲区（方案B）
- *(chart)* [**breaking**] 封装 TimingTrack 的 pub 字段以保护缓存不变量
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- *(bms-processor)* 引入 BmsConverter 收拢转换步骤归属
- *(chart)* 新增 Event::sort_key 收敛同脉冲排序约定
- *(bms-processor)* 用 partition_point 替换线性 BPM 查找
- *(bms-processor)* 引入 BmsEvent 别名消除重复类型签名
- *(chart)* 事件排序显式使用 (tick, priority) 元组
- *(chart)* [**breaking**] NoteKind::Mine f64 → Damage newtype，开启 Eq 派生
- *(processor)* find_max_measure 用宏减少重复迭代
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- *(chart)* [**breaking**] rename NoteSide::ONE/TWO to P1/P2
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype
- *(bms-processor)* [**breaking**] eliminate panic via type-driven BmsChannel.player
- *(bms-tokenizer)* [**breaking**] replace BmsIndex generic+tag with newtype+Deref pattern
- enable eight additional clippy lints
- elevate warn-by-default clippy groups to deny and add restriction lints
- [**breaking**] move layout mapping to processor crates, rename Bme to Beat on BMSON side
- *(chart)* [**breaking**] add BmsChannel type for type-safe BMS channel parameter in BmsLayout
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model
- eliminate expect attributes via better implementations
- adopt stricter lint config from bms-rs
- *(chart,player,bms-processor,bmson-processor)* [**breaking**] use Duration instead of f64 for time domain API

### Testing

- 测试函数名补齐期望后缀
- *(bms-processor)* LNTYPE2 (MGQ) 长音配对集成测试
- *(bms-processor)* STOP 与 BPM 同 tick 时序测试（A11）
- 合并重复测试文件，清理 bmspec 英文草稿注释
- 统一 bmspec 测试编号与注释格式
- *(bms-processor)* 添加 BMSpec 端到端兼容性测试
- move public-API unit tests to integration test directories
