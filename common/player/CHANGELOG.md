# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-player-v0.0.1...bmsrs-player-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-player-v0.0.1) - 2026-11-07

### Bug Fixes

- *(player)* include stop-tick events in time-range queries
- *(player)* [**breaking**] advance 改时间驱动状态机，修复帧步进无法穿过 STOP (migration: current_tick() 失去 const；STOP 期间 current_time()
的返回值从暂停前时刻变为真实流逝时间)
- 最终审查修复——benchmark 去重、PMS 检测方法化、TimingCache 排序优化、warning 字段清理
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据 (migration: ChartInfo/BgaResource 新增公开字段，Bms 新增 detected_base，)
- *(chart)* 统一负 BPM 校验为非零有限值
- *(player)* Player::new 运行时验证 resolution 和 BPM，自动排序事件
- *(player)* 修正复杂度标注并添加 TimingCache Debug
- *(bmson-processor)* process_default 对异常 keys 报错，保留 DJ.NEXT judge/life deltas
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- *(player)* SpeedCache 同脉冲关键帧插值除零守护
- *(test)* 更新 Extended BPM 与 speed interpolation 测试断言
- *(bms-parser)* [**breaking**] implement position-based channel merge and case-insensitive BmsIndex (migration: Messages::raw type changed from
BTreeMap<u16, BTreeMap<BmsChannel, String>> to
BTreeMap<u16, BTreeMap<BmsChannel, Vec<String>>>

Sub-project A — Channel merge:
- Change Messages::raw to store individual lines per (measure, channel)
  as Vec<String> instead of concatenated String
- Add merge_channel() for position-based merge: later lines overwrite
  non-00 positions, 00 preserves existing values
- BGM channel (01) processed independently per line (polyphony support)
- Measure length (02) uses last-line-wins semantics

Sub-project B — Case-insensitive BmsIndex:
- BmsIndex now uses case-insensitive PartialEq/Eq/Ord/Hash
- BmsBaseMode removed (unified with BmsBase in index.rs)
- Add BmsIndex::normalize(base) for explicit normalization
- Parser pre-scans for #BASE header before processing tokens
- All BTreeMap insertions in parser normalize keys via normalize()
- pair_lnobj uses BmsIndex comparison instead of string comparison
- Fix pre-existing broken doc links in bmsrs-chart and decorative
  comments in bmsrs-player)

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- *(player)* keep on-demand event-time conversion over precomputed tables
- document the event-sourced simulation contracts
- *(readme)* translate README to English and add a Chinese guide
- *(player)* translate comments and docs to English
- *(readme)* 新增根 README
- *(agents)* 修正二轮审计发现的三处 API 不一致
- 添加目录级 AGENTS.md 并精简子 crate
- 记录新增的热路径基准基础设施
- *(player)* 同步 TimingCache 下沉至 chart 的现状
- *(player)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容

### Features

- *(chart)* [**breaking**] carry stop durations as precise time (migration: EventKind::Stop, StopEvent, and Event::stop change the
duration field from u64 ticks to Duration.)
- *(bms-processor)* [**breaking**] clear BGA layer on undefined BMP references (migration: EventKind::Bga.resource_id changes from
BgaResourceId to Option<BgaResourceId>.)
- *(player)* add delta-range viewport queries for reverse scroll
- *(facade)* 增加字节入口与统一诊断出口
- *(chart)* [**breaking**] 引入分层音符与释放音角色
- *(chart)* [**breaking**] Chart::validate 承载 BGA 引用完整性 (migration: ChartDataError 新增变体；手构谱面 BGA 引用
越界可被拒绝)
- *(chart)* [**breaking**] ChartData::validate 补交叉不变量与音频索引完整性 (migration: ChartDataError 新增两个变体（#[non_exhaustive]
下游 match 不受影响）；手构谱面新增两处可被拒绝的状态)
- *(chart)* TimingTrack 初始 BPM 校验
- *(player)* 添加 visible_tick_range 便捷方法
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(chart)* 添加 TimingTrack::simple(bpm) 便利构造器
- *(player)* 添加滚动位置累积与 SPEED 间距插值
- 支持负 BPM（逆走）与负 STOP 钳位
- *(bms-processor)* implement LNTYPE 2 (MGQ) and fix RDM 00 filtering

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- *(lints)* 清偿 clippy 1.99 pedantic 债务
- 更新 workspace 依赖至最新版本
- *(player)* 扩展 events_in_range 基准并覆盖 BGM 密集场景
- *(player)* 新增 criterion 热路径基准

### Performance

- *(player)* advance/seek 改用 TimingCache 快路径
- *(chart)* TimingTrack.build_events() 惰性缓存
- *(player)* 为 scroll_rate_at 添加预计算 ScrollCache

### Refactoring

- refactor!(player): remove the time_to_tick_f64 alias
- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(chart)* [**breaking**] remove Eq from float-bearing types
- *(chart)* [**breaking**] make the event stream the single timing source
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(chart)* [**breaking**] 删除 ChartData.resolution 冗余字段
- *(player)* [**breaking**] 裁剪同模板浅过滤查询 (migration: 删除 Player::notes_in_lane、bgm_in_range、
bar_lines_in_range、bga_events_in_range)
- 消费点迁移至新 TimingTrack API，TimingCache 标记弃用
- *(player)* [**breaking**] Player::new 返回 Result，消除 panic 路径 (migration: 所有 Player::new 调用方需处理 Result。)
- *(player,parser)* 恢复排序诊断 + 更新 parser 契约
- *(tokenizer)* [**breaking**] 封装 ChannelIndex/BmsMessage 字段，ChartData 增加 sort_events (migration: ChannelIndex 内部字段不再 pub（移除 Deref，改为显式
方法）；BmsMessage.track/channel 不再是 pub 字段。)
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- 用切片模式取代大部分 indexing_slicing expect
- *(chart)* [**breaking**] 移除 ChartData/Chart 的非法 Default (migration: ChartData 与 Chart 不再实现 Default。)
- *(chart)* 将 TimingCache 从 player 下沉至 chart
- *(chart)* [**breaking**] NoteKind::Mine f64 → Damage newtype，开启 Eq 派生
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- *(chart)* [**breaking**] rename NoteSide::ONE/TWO to P1/P2
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype (migration: PlayerSide is removed; use NoteSide. The Player1/Player2
variants are gone; use NoteSide::ONE / NoteSide::TWO, or NoteSide::new for
arbitrary indices. BmsChannel::player_side is renamed to note_side.)
- enable eight additional clippy lints
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model (migration: Chart::lane_count removed; Beat5k/Beat7k/Beat10k/Beat14k/
Popn5k/Popn9k/GenericLayout and Layout trait removed; BmsMapping/BmsonMapping
renamed to BmsLayout/BmsonLayout with new method signatures returning
NoteData; DefaultNoteData renamed to NoteData; NoteData trait renamed to
NoteDataLike; Player::notes_in_lane takes (PlayerSide, Lane, …) instead of
u16; processor process() returns Chart<NoteData> instead of Chart<L::NoteData>.)
- adopt stricter lint config from bms-rs
- rename core/ to common/ and move player/ under common/

### Testing

- *(player)* player_audio_assets 断言改为匹配有效夹具
- 测试函数名补齐期望后缀
- *(player)* BGM 与 BGA 事件查询测试
- *(player)* 添加 InvalidBpm panic 测试覆盖缺失的验证路径
- 合并重复测试文件，清理 bmspec 英文草稿注释
- *(bmsrs-player)* 添加播放器仿真层集成测试
