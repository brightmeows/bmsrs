# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-v0.1.0) - 2026-09-16

### Bug Fixes

- *(ci)* merge duplicate env block in release workflow
- *(parser)* decode_mine_raw 地雷值始终用 base36（不受 #BASE 62 影响）
- 最终审查修复——benchmark 去重、PMS 检测方法化、TimingCache 排序优化、warning 字段清理
- *(chart)* T2 review——TimingCache 委托 TimingTrack + 清除不可达 arm
- *(processor)* T1 review——更新测试名和 PairedLn 文档
- *(bms-processor)* 将 pair_lntype 分组改回 BTreeMap 消除测试非确定性
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据
- *(chart)* 统一负 BPM 校验为非零有限值
- *(tokenizer)* 行内 // 注释要求前置空白，避免 URL 被截断
- *(control-flow)* try_map_payload 继承原始 FlowDoc 的 warnings
- *(bms-parser)* 保留长音事件中的 "00" 条目
- *(bmson-def)* BMSON version 字段 SemVer 前缀校验
- *(bmson-processor)* 每通道独立 BGM discard，不再使用全局 playable_pulses 集合
- *(bmson-def)* 版本检测扫描跟踪花括号深度，忽略嵌套 version 键
- *(bms-parser)* 非 BGM 通道过滤 "00" 值
- *(player)* Player::new 运行时验证 resolution 和 BPM，自动排序事件
- *(control-flow)* [**breaking**] 未闭合 #RANDOM/#SWITCH 块内容提升到顶层而非丢弃
- *(chart)* tick_to_duration 钳位超出 Duration::MAX 的秒数
- *(processor)* position_to_tick 改用 u128 四舍五入
- *(chart)* 跳过无效 BPM 变更值（0/NaN/Inf）
- *(chart)* TimingCache::new 内部排序 bpm_changes
- *(player)* 修正复杂度标注并添加 TimingCache Debug
- *(bmson-def)* v1→root 回填 per-note ln_type_hint 对齐 v0 路径
- *(bmson-processor)* process_default 对异常 keys 报错，保留 DJ.NEXT judge/life deltas
- *(bms-parser)* finalize 清空 non_event_data 保证幂等
- *(bms-processor)* 修复 LNOBJ 下 ch51-69 静默丢失与 BGA resource_id 不一致
- *(tokenizer)* preprocess 正确保留 UTF-8 多字节字符
- *(bmson-def,processor)* 统一 LN 类型模型，移除冗余 ln_mode 字段
- *(bms-processor,def,tokenizer,facade)* 代码审查问题修复
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- *(chart)* TimingTrack::default 使用安全的 init_bpm=120 而非 0.0
- *(processor)* build_bar_events 适配变拍号（#xxx02）小节线位置
- *(player)* SpeedCache 同脉冲关键帧插值除零守护
- *(bmson-processor)* 从完整事件集计算小节线末尾脉冲
- *(test)* 更新 Extended BPM 与 speed interpolation 测试断言
- *(bms-parser)* Extended BPM 通道 08 中 "00" 应作为休止跳过
- *(tokenizer)* [**breaking**] BmsIndex case-sensitive comparison for Base62 support
- *(parser,processor)* measure length ratio, mine damage, Base62 normalization, SPEED events
- *(bms-parser, bms-processor)* filter BPM 00 rest, add LNOBJ BGM playback
- *(bms-parser)* [**breaking**] implement position-based channel merge and case-insensitive BmsIndex
- *(bmson-processor)* respect continuation flag in slicing algorithm
- *(bms-parser)* correct BGA channel routing for 0A (Layer2) and 05 (Seek)
- *(bms-control-flow)* avoid panic on #RANDOM 0 / #SWITCH 0
- replace confusable Unicode dashes/spaces, add clippy rules and pre-commit hook
- *(bmson-def)* spec compliance fixes, missing fields, and test migration
- *(lint)* allow pub_use

### CI

- migrate release automation and mirror sync to GitHub
- 使用codeberg-small，默认安装semver-check
- 添加 no-module-level-expect-allow pre-commit hook
- reference bms-table-rs workflow patterns
- extend clippy to all-targets and fix test code lint errors
- treat warnings as errors in pre-commit clippy and doc hooks
- *(.forgejo)* use rust-latest docker image to accelerate toolchain setup
- *(.forgejo)* bump release-plz to v0.3.159 and add cargo-semver-checks
- *(.forgejo)* align workflow steps and rust-toolchain-action with bms-table-fetch
- add pre-commit configuration with parallel hooks
- use self-hosted toolchain setup
- persist git credentials for release-plz push auth
- split cache into explicit restore+save to avoid timeout losing cache
- add rustup cache and configure git user for release-plz
- cache rust toolchain
- fix install-rust-toolchain requirement for release-plz calling `cargo metadata`
- add release-plz

### Documentation

- point skill repository links at GitHub ([#2](https://github.com/brightmeows/bmsrs/pull/2))
- 简化提交格式节，新增 .worktrees 约定
- *(tokenizer)* 更新值约束对应字段的文档注释
- *(agents)* 固化三层职责判据与 Bms 保真原则
- *(parser,processor)* 同步 non_event_data 归一化契约变更到 AGENTS.md 和 API 文档
- *(agents)* 记录归一化契约、延迟字段与 IR 限制
- 新增开发用 skill 来源说明与引用规则
- *(agents)* 修正二轮审计发现的三处 API 不一致
- *(agents)* 修正 AGENTS.md 与源码不一致之处
- 添加目录级 AGENTS.md 并精简子 crate
- *(tokenizer,parser)* 补充 #WAVCMD 完整规格文档
- 同步 AGENTS.md 反映近期 API 变更和新增 hook
- 移除职责矩阵的「零依赖？」列
- 记录新增的热路径基准基础设施
- *(chart)* 明确 TimingTrack 与 TimingCache 的职责分工
- 新增私有接口按公有模式设计的建议
- *(bms-processor)* 同步 collect_* 归属 BmsConverter 的现状
- *(chart)* 补充 TimingCache 与 Event::sort_key 说明
- *(player)* 同步 TimingCache 下沉至 chart 的现状
- 标注当前预发布状态，说明接口可随时变动
- 固化测试防重复规则与公开 API 测试放置约定
- *(agents)* 更新中文化约定，中文用于文档、注释与提交信息
- *(bmsrs)* 将文档注释与实现注释中文化
- *(bmson-processor)* 将文档注释与实现注释中文化
- *(bmson-de-chumsky)* 将文档注释与实现注释中文化
- *(bmson-def)* 将文档注释与实现注释中文化
- *(processor)* 将文档注释与实现注释中文化
- *(parser)* 将文档注释与实现注释中文化
- *(control-flow)* 将文档注释与实现注释中文化
- *(tokenizer)* 将文档注释与实现注释中文化
- *(tokenizer)* 将文档注释与实现注释中文化
- *(player)* 将文档注释与实现注释中文化
- *(chart)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容
- *(parser)* document information-preservation philosophy (no implicit subtitle)
- align MSRV with Cargo.toml (1.85 -> 1.88)
- align AGENTS.md with actual code
- add release workflow note to root AGENTS.md
- expand AGENTS.md with lint, edition, pipeline, and structure notes
- add AGENTS.md for new crates and update root crate table
- *(AGENTS.md)* add crate list and commit format
- update AGENTS.md with MSRV and clippy lint restrictions
- move bmson-def API conventions to crate-level AGENTS.md

### Features

- *(gitignore)* 整体忽略 OpenSpec 变更
- *(tokenizer-derive)* 新增 #[derive(BmsIndexNewtype)] proc-macro 简化 12 个索引 newtype
- *(tokenizer)* WavCmdParams pitch 范围 0–127 校验
- *(tokenizer)* StpParams measure 范围 0–999 校验
- *(tokenizer)* ExWavParams 参数范围校验
- *(bms)* 支持扩展音符通道（1A-1Z 等）
- *(processor)* PMS 变体自动检测（Standard / BME-type）
- *(processor)* 新增 process_with_warnings 结构化 warning 收集
- *(processor)* LN 区间内 visible note 移除（配对 + unpaired start）
- *(tokenizer)* 新增编码检测与转换模块
- *(tokenizer)* 结构化解析 #WAVCMD（WavCmdParams）
- *(bms-parser)* 隐式副标题分割（D5）
- *(tokenizer,parser)* 行内注释处理与 Bms::from_text 便捷方法
- *(chart)* TimingTrack 初始 BPM 校验
- *(player)* 添加 visible_tick_range 便捷方法
- *(chart)* 添加 EventKind::map_custom/map_ext 与 filter_map_events
- *(chart)* 添加 ChartData::map_events + Chart::map_events
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(chart)* 添加 TimingTrack::simple(bpm) 便利构造器
- *(bms-processor)* 实现 BmsCustomEvent，保留丢弃的引擎特定通道
- *(processor)* BANNER/BACKBMP/STAGEFILE/PREVIEW 映射至 ChartInfo
- *(bmsrs)* 添加 microquad_player 示例（BMS/BMSON 谱面播放器）
- *(player)* 添加滚动位置累积与 SPEED 间距插值
- 支持负 BPM（逆走）与负 STOP 钳位
- *(tokenizer)* add preprocess() for BMS comment stripping
- *(chart)* add Event::Speed variant for visual note-spacing keyframes
- upgrade MSRV to 1.88, add itertools/serde_with/insta/proptest, apply across codebase
- *(bms-processor)* implement LNTYPE 2 (MGQ) and fix RDM 00 filtering
- *(bmsrs)* add facade re-exports for chart, processors, and player
- *(bmsrs-player)* implement Player simulator with TimingCache
- *(bms-processor)* implement BMS to Chart conversion processor
- *(bmson-processor)* implement BMSON to Chart conversion
- add bms-control-flow and bms-parser crates, update facade
- extend pre-commit hook to catch decorative comment patterns
- *(bmson-de-chumsky)* implement chumsky-based JSON parser + bmson deserializer
- replace hand-written header match with proc-macro derive
- *(bmsrs)* add root re-export crate
- add bms/tokenizer/AGENTS.md
- enable clippy::missing_docs_in_private_items lint
- *(bmson-def)* implement multi-version bmson type definitions
- add serde support to workspace and bms-tokenizer types
- *(bms-tokenizer)* implement BMS tokenizer
- add workspace metadata, lints, clippy and deny configs
- add workspace structure with empty lib crates

### Other

- OpenSpec Config
- *(cargo)* allow multiple_crate_versions
- OpenSpec config
- *(parser,bmson-def)* 引号反引号化 + depth u8→u32
- *(example)* normalize_event 改为 const fn 修复预存 clippy 警告
- *(tokenizer)* 合并 many_single_char_names 为模块级 expect
- 修复公开工具函数的 clippy lint
- 更新 workspace 依赖至最新版本
- *(player)* 扩展 events_in_range 基准并覆盖 BGM 密集场景
- *(player)* 新增 criterion 热路径基准
- 启用 clippy::clone_on_ref_ptr 要求显式 Arc::clone()
- pin release-plz to v0.3.158 and add release-plz.toml
- set MSRV 1.85, reorganize clippy lints by category, add expect_used lint
- add package metadata for crates.io publishing
- upgrade workspace resolver from v2 to v3
- remove all //===== section divider comments
- bmson-ser
- *(ignore)* ignore /docs/blueprint
- init

### Performance

- 稠密消息优化——流式 NonZeroChunks 迭代器 + 相邻同值 BPM 去重
- *(chart)* TimingCache::duration_to_tick 由 O(log² n) 降为 O(log n)
- *(player)* advance/seek 改用 TimingCache 快路径
- *(chart)* [**breaking**] AudioAsset.path 改用 Arc<Path> 共享所有权
- *(chart)* TimingTrack.build_events() 惰性缓存
- *(player)* 为 scroll_rate_at 添加预计算 ScrollCache

### Refactoring

- *(bmsrs)* 将 facade crate 从 bmsrs/ 子目录移至工作区根目录
- *(bmson)* [**breaking**] 将 bmson-de-chumsky 合并为 bmson-def 的示例，不再作为独立 crate 导出
- *(tokenizer)* [**breaking**] 移除 C 泛型参数，字符串字段统一使用 String
- *(tokenizer)* 启用 derive_more::try_unwrap 消除 8 个 TryFrom impl
- *(de-chumsky)* 抽取 deser_err 和 join_errors 辅助函数消除重复
- *(parser)* 移除 norm_as! 宏，直接内联 BmpIndex::from(id.normalize(base))
- *(parser)* [**breaking**] Position 字段私有化，提供访问器，移除防御性检查
- *(tokenizer)* [**breaking**] base36_digit_value 提升为 pub，消除 parser 本地副本
- *(bms)* [**breaking**] NonEventData.values 从 Vec<String> 改为 Vec<BmsIndex>
- *(parser)* [**breaking**] Position::new 强制 denom > 0 不变式
- *(tokenizer)* [**breaking**] ExWavParams 结构化 flags+values → pan/volume/frequency
- *(tokenizer)* [**breaking**] WavCmdParams 枚举化 command_id，wav_index 改用 WavIndex
- *(parser)* [**breaking**] 移除 Bms.detected_base 字段
- *(bms)* [**breaking**] MineEvent 改存 raw_value: Option<u16>，伤害计算移至 processor
- 消费点迁移至新 TimingTrack API，TimingCache 标记弃用
- *(chart)* TimingTrack 分段化——O(log n) 查询，合并 TimingCache 功能
- *(core)* Rust 化改进——提取 build_sorted_events, 元组排序, HashMap/HashSet, 直解构, .ok().ok_or()→map_err, if+unwrap_or→match, for→find_map
- *(processor)* 两个 processor 统一通过 ChartData::sort_events() 排序事件
- *(chart,processor)* 引入 BpmLookup 结构体消除 bpm_at_tick 自由函数
- *(parser)* 用 iter_nonzero_chunks 消除通道消息分派的重复拆分+过滤
- *(parser)* [**breaking**] 在 parser 层归一化 non_event_data 索引值，消除 processor 的重复归一化
- *(player)* [**breaking**] Player::new 返回 Result，消除 panic 路径
- *(chart,processor)* BPM 校验集中化到 TimingTrack::validate
- *(tokenizer,parser)* WavCmdParams.value f64→u32 + from_text 返回 warnings
- *(player,parser)* 恢复排序诊断 + 更新 parser 契约
- *(tokenizer)* detect_encoding 移为 BmsEncoding::detect 关联方法
- *(parser,processor)* 将 non_event_data 改为类型安全 NonEventData 结构体
- *(example)* 用 use 替换全量路径引用
- *(example)* 使用新 API 简化 normalize 并修复错误处理与内存
- *(example)* 简化 microquad_player 示例
- *(bmson-processor)* [**breaking**] 引入 BmsonConverter struct，消除自由函数 &mut 参数
- *(bms-processor)* bga_resources 构建内化到 build_metadata
- *(bms-processor)* stops 循环内化到 BmsConverter::collect_stop_events
- *(bms-processor)* [**breaking**] BmsConverter 内化 events 缓冲区（方案B）
- *(tokenizer)* [**breaking**] 封装 ChannelIndex/BmsMessage 字段，ChartData 增加 sort_events
- *(chart)* [**breaking**] 封装 TimingTrack 的 pub 字段以保护缓存不变量
- *(bmson-def,de-chumsky)* [**breaking**] 公开自由函数挂到类型
- *(control-flow)* [**breaking**] #IF-#ELSEIF-#ELSE 重构为互斥链模型
- *(tokenizer)* [**breaking**] #SKIP 改为无参数符合 BMS 规范
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- *(bmsrs,tokenizer)* 消除门面模块重复导出，增加 tokenize_owned 便捷方法
- *(tokenizer,parser)* 用 expect 取代静默回退值，修复 clippy semicolon 警告
- 用切片模式取代大部分 indexing_slicing expect
- *(tokenizer)* 将 is_base62/base36_decode 等绑定到 BmsBase 方法
- *(parser)* 消除 messages.rs 中所有模块级 #![allow]/#![expect]
- *(parser)* 提取 finalize 通道分派逻辑到独立方法消除 too_many_lines
- *(bmson-processor)* BmsonNoteExt ln_* 字段改用 typed enum 而非 String
- *(tokenizer,parser)* 收拢 base36_decode 等公共工具函数到 tokenizer 并公开导出
- *(chart)* [**breaking**] 移除 ChartData/Chart 的非法 Default
- *(bmson-processor)* 移除 slice_channel 冗余排序
- *(bms-processor)* 引入 BmsConverter 收拢转换步骤归属
- *(bmson-processor)* slice_channel 改用 TimingCache 加速切片
- *(chart)* 将 TimingCache 从 player 下沉至 chart
- *(chart)* 新增 Event::sort_key 收敛同脉冲排序约定
- *(bms-processor)* 用 partition_point 替换线性 BPM 查找
- *(bms-processor)* 引入 BmsEvent 别名消除重复类型签名
- *(chart)* 事件排序显式使用 (tick, priority) 元组
- *(chart)* [**breaking**] NoteKind::Mine f64 → Damage newtype，开启 Eq 派生
- *(processor)* find_max_measure 用宏减少重复迭代
- move public-API tests from src/ to tests/
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- forbid unsafe and remove the last unsafe blocks
- *(chart)* [**breaking**] rename NoteSide::ONE/TWO to P1/P2
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype
- *(bms-processor)* [**breaking**] eliminate panic via type-driven BmsChannel.player
- *(bms-tokenizer)* [**breaking**] replace BmsIndex generic+tag with newtype+Deref pattern
- enable eight additional clippy lints
- elevate warn-by-default clippy groups to deny and add restriction lints
- enable additional clippy restriction/nursery lints
- [**breaking**] move layout mapping to processor crates, rename Bme to Beat on BMSON side
- *(chart)* [**breaking**] add BmsChannel type for type-safe BMS channel parameter in BmsLayout
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model
- eliminate expect attributes via better implementations
- adopt stricter lint config from bms-rs
- *(bmsrs)* move root crate to bmsrs/ directory
- *(bms-control-flow)* [**breaking**] replace DeterministicRng with rand::RngExt blanket impl
- *(bms-control-flow)* rename FlowTree -> FlowDoc
- *(bms-control-flow)* use tuple struct + Deref for FlowTree<P>
- *(bms-control-flow)* [**breaking**] redesign as generic FlowTree<P>
- rename core/ to common/ and move player/ under common/
- *(pre-commit)* inline external script, consolidate repos, add comments
- *(workspace)* move all local path deps to [workspace.dependencies]
- remove decorative divider comments, add bmson version detection
- categorize errors and use thiserror throughout
- type header values and unify error handling
- *(bmson-def)* migrate key string/path fields to borrowed references
- *(bmson-def)* migrate path fields to PathBuf, add Eq/Copy, improve error handling
- *(bms-tokenizer)* split BmsHeader sub-enums into dedicated submodules
- trim obvious content from AGENTS.md files
- *(bmson-def)* move V0LnType to common.rs, rename to LnMode
- *(bmson-def)* [**breaking**] replace X enum with raw u64 for NoteEvent.x
- rename bmsrs-processor to bmsrs-chart

### Testing

- 测试函数名补齐期望后缀
- *(bmson-def)* 版本检测边缘用例
- *(bmson-processor)* 地雷通道伤害值测试
- *(bms-processor)* LNTYPE2 (MGQ) 长音配对集成测试
- *(player)* BGM 与 BGA 事件查询测试
- *(chart)* TimingTrack 极端值测试
- *(parser)* non_event_data 合并行为与 merge_channel 多分辨率集成测试
- *(bms-parser)* 添加 Base62 消息事件大小写敏感索引的集成测试
- *(bms-control-flow)* 添加空 #SWITCH 块的集成测试
- *(bms-control-flow)* 添加深层嵌套 #RANDOM / #SWITCH 的集成测试
- *(processor)* BMSON 反向滚动事件和 key_channels 不可见音符测试
- *(tokenizer)* BGA 裁剪坐标边界测试
- *(bmsrs-chart)* 负 BPM roundtrip 测试（A12）
- *(bms-processor)* STOP 与 BPM 同 tick 时序测试（A11）
- *(player)* 添加 InvalidBpm panic 测试覆盖缺失的验证路径
- 合并重复测试文件，清理 bmspec 英文草稿注释
- 统一 bmspec 测试编号与注释格式
- *(bms-processor)* 添加 BMSpec 端到端兼容性测试
- *(bmsrs-player)* 添加播放器仿真层集成测试
- *(bmsrs-chart)* 添加核心数据模型集成测试
- *(bmson-processor)* 添加 BMSON 处理器集成测试
- *(bmson-de-chumsky)* 添加解析器与 JSON 解析集成测试
- *(bmson-def)* 添加版本检测、serde 往返与 v0/v1 转换测试
- move public-API unit tests to integration test directories
