# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-chart-v0.0.1...bmsrs-chart-v0.0.2) - 2026-11-10

### Documentation

- *(chart)* fix TimingTrack attribution in crate docs
- *(readme)* flip install instructions to the released 0.0.1
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-chart-v0.0.1) - 2026-11-07

### Bug Fixes

- *(bmson-processor)* accumulate same-pulse stop durations per bmson spec
- *(processor)* [**breaking**] BPM 无效值统一过滤并产生警告
- *(processor)* 消除 BMS 侧事件向量的重复排序
- 最终审查修复——benchmark 去重、PMS 检测方法化、TimingCache 排序优化、warning 字段清理
- *(chart)* T2 review——TimingCache 委托 TimingTrack + 清除不可达 arm
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据 (migration: ChartInfo/BgaResource 新增公开字段，Bms 新增 detected_base，)
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
- *(agents)* record engine-divergence notes for timing semantics
- sync AGENTS.md files with the event-sync changes
- document the event-sourced simulation contracts
- *(readme)* translate README to English and add a Chinese guide
- *(chart)* translate comments and docs to English
- *(readme)* 新增根 README
- *(chart)* 写明变换 API 的消费者场景
- *(chart)* BpmLookup 补 init_bpm 不校验的前置条件
- *(agents)* 记录归一化契约、延迟字段与 IR 限制
- *(agents)* 修正 AGENTS.md 与源码不一致之处
- 添加目录级 AGENTS.md 并精简子 crate
- 同步 AGENTS.md 反映近期 API 变更和新增 hook
- *(chart)* 明确 TimingTrack 与 TimingCache 的职责分工
- *(chart)* 补充 TimingCache 与 Event::sort_key 说明
- *(chart)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容

### Features

- *(chart)* [**breaking**] represent unspecified long-note type hint as None (migration: `ChartData::ln_type_hint()` returns
`Option<LnTypeHint>`; `with_ln_type_hint` takes `Option<LnTypeHint>`;)
- *(chart)* add with_start_margin for the leading-note margin
- *(chart)* infer BGA resource kind from the file extension
- *(chart)* [**breaking**] carry stop durations as precise time (migration: EventKind::Stop, StopEvent, and Event::stop change the
duration field from u64 ticks to Duration.)
- *(bms-processor)* [**breaking**] carry RANK, TOTAL, and VOLWAV in ChartInfo (migration: BmsGameplay loses rank, def_ex_rank, total, and
vol_wav; ChartInfo gains judge_rank, total, and volwav_percent.)
- *(bms-processor)* [**breaking**] resolve judgment rank from EXRANK table (migration: BmsCustomEvent::JudgeOverride is removed; channel A0
produces EventKind::JudgeRank { percent: f64 } instead.)
- *(bms-processor)* [**breaking**] model SWBGA definitions and fix A5 key lookup (migration: BmsCustomEvent::BgaKeyBound changes from
{ resource_id: BgaResourceId } to { swbga_id: SwBgaResourceId };
ChartInfo gains swbga_resources.)
- *(bms-processor)* [**breaking**] carry TEXT bodies in events and add miss_text (migration: BmsCustomEvent::TextDisplay changes from
{ text_index: u32 } to { text: Arc<str> }; ChartInfo gains miss_text.)
- *(bms-processor)* [**breaking**] clear BGA layer on undefined BMP references (migration: EventKind::Bga.resource_id changes from
BgaResourceId to Option<BgaResourceId>.)
- *(facade)* 增加字节入口与统一诊断出口
- *(bmson)* [**breaking**] 收口元数据、切片时间与资源路径
- *(chart)* [**breaking**] 引入分层音符与释放音角色
- *(chart)* [**breaking**] Chart::validate 承载 BGA 引用完整性 (migration: ChartDataError 新增变体；手构谱面 BGA 引用
越界可被拒绝)
- *(chart)* [**breaking**] ChartData::validate 补交叉不变量与音频索引完整性 (migration: ChartDataError 新增两个变体（#[non_exhaustive]
下游 match 不受影响）；手构谱面新增两处可被拒绝的状态)
- *(chart)* TimingTrack 初始 BPM 校验
- *(chart)* 添加 EventKind::map_custom/map_ext 与 filter_map_events
- *(chart)* 添加 ChartData::map_events + Chart::map_events
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(chart)* 添加 TimingTrack::simple(bpm) 便利构造器
- *(bms-processor)* 实现 BmsCustomEvent，保留丢弃的引擎特定通道
- 支持负 BPM（逆走）与负 STOP 钳位
- *(chart)* add Event::Speed variant for visual note-spacing keyframes

### Other

- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- *(lints)* 清偿 clippy 1.99 pedantic 债务

### Performance

- *(chart)* TimingCache::duration_to_tick 由 O(log² n) 降为 O(log n)
- *(player)* advance/seek 改用 TimingCache 快路径
- *(chart)* [**breaking**] AudioAsset.path 改用 Arc<Path> 共享所有权
- *(chart)* TimingTrack.build_events() 惰性缓存

### Refactoring

- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(chart)* [**breaking**] remove the unused NoteSide constructor
- *(chart)* [**breaking**] remove Eq from float-bearing types
- *(chart)* [**breaking**] make the event stream the single timing source
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(common/chart)* 过滤无效 BPM 并统一 bpm_at_tick
- *(chart)* [**breaking**] 删除恒真的 TimingTrack::validate
- *(chart)* [**breaking**] 删除 ChartData.resolution 冗余字段
- *(chart)* map_events 与 filter_map_events 的字段重列收敛到私有辅助
- *(chart)* [**breaking**] 删除已废弃的 TimingCache 委托包装
- *(chart)* TimingTrack 改用派生 Clone
- *(chart)* 删除 TimingEvent 未使用的 is_stop 方法
- 消费点迁移至新 TimingTrack API，TimingCache 标记弃用
- *(chart)* TimingTrack 分段化——O(log n) 查询，合并 TimingCache 功能
- *(core)* Rust 化改进——提取 build_sorted_events, 元组排序, HashMap/HashSet, 直解构, .ok().ok_or()→map_err, if+unwrap_or→match, for→find_map
- *(chart,processor)* 引入 BpmLookup 结构体消除 bpm_at_tick 自由函数
- *(chart,processor)* BPM 校验集中化到 TimingTrack::validate
- *(tokenizer)* [**breaking**] 封装 ChannelIndex/BmsMessage 字段，ChartData 增加 sort_events (migration: ChannelIndex 内部字段不再 pub（移除 Deref，改为显式
方法）；BmsMessage.track/channel 不再是 pub 字段。)
- *(chart)* [**breaking**] 封装 TimingTrack 的 pub 字段以保护缓存不变量 (migration: TimingTrack 的 init_bpm/bpm_changes/stops 不再是 pub
字段；改为 init_bpm() -> f64 / bpm_changes() -> &[BpmChange] /
stops() -> &[StopEvent]。)
- *(bmsrs-chart)* [**breaking**] tick 从 Event 各变体提取到外层 Event { tick, kind: EventKind }
- *(bmson-processor)* BmsonNoteExt ln_* 字段改用 typed enum 而非 String
- *(chart)* [**breaking**] 移除 ChartData/Chart 的非法 Default (migration: ChartData 与 Chart 不再实现 Default。)
- *(chart)* 将 TimingCache 从 player 下沉至 chart
- *(chart)* 新增 Event::sort_key 收敛同脉冲排序约定
- *(chart)* 事件排序显式使用 (tick, priority) 元组
- *(chart)* [**breaking**] NoteKind::Mine f64 → Damage newtype，开启 Eq 派生
- *(chart)* [**breaking**] restructure Chart into SongInfo + ChartInfo + ChartData
- *(chart)* [**breaking**] unified Event<T, C> enum with format-extension traits
- *(chart)* [**breaking**] rename NoteSide::ONE/TWO to P1/P2
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype (migration: PlayerSide is removed; use NoteSide. The Player1/Player2
variants are gone; use NoteSide::ONE / NoteSide::TWO, or NoteSide::new for
arbitrary indices. BmsChannel::player_side is renamed to note_side.)
- enable eight additional clippy lints
- elevate warn-by-default clippy groups to deny and add restriction lints
- enable additional clippy restriction/nursery lints
- [**breaking**] move layout mapping to processor crates, rename Bme to Beat on BMSON side (migration: Layout types/traits removed from bmsrs-chart.
Import from bms_processor::layout / bmson_processor::layout instead.)
- *(chart)* [**breaking**] add BmsChannel type for type-safe BMS channel parameter in BmsLayout (migration: BmsLayout::map_channel signature changed from
 to . All callers must
update.)
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model (migration: Chart::lane_count removed; Beat5k/Beat7k/Beat10k/Beat14k/
Popn5k/Popn9k/GenericLayout and Layout trait removed; BmsMapping/BmsonMapping
renamed to BmsLayout/BmsonLayout with new method signatures returning
NoteData; DefaultNoteData renamed to NoteData; NoteData trait renamed to
NoteDataLike; Player::notes_in_lane takes (PlayerSide, Lane, …) instead of
u16; processor process() returns Chart<NoteData> instead of Chart<L::NoteData>.)
- adopt stricter lint config from bms-rs
- rename core/ to common/ and move player/ under common/

### Testing

- 测试函数名补齐期望后缀
- *(chart)* TimingTrack 极端值测试
- *(bmsrs-chart)* 负 BPM roundtrip 测试（A12）
- *(player)* 添加 InvalidBpm panic 测试覆盖缺失的验证路径
- 合并重复测试文件，清理 bmspec 英文草稿注释
- *(bmsrs-chart)* 添加核心数据模型集成测试
