# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.0.4](https://github.com/brightmeows/bmsrs/compare/bmsrs-bmson-processor-v0.0.3...bmsrs-bmson-processor-v0.0.4) - 2026-11-28

### Bug Fixes

- *(bmson-processor)* keep Hln through the LN hint conversion

### Documentation

- correct stale passages found in the full read
- apply the three CodeRabbit findings on the prune
- prune AGENTS.md and co-locate the boundary registers
- refresh the release version and record the visibility rules

### Features

- *(facade)* [**breaking**] accept Input on the BMSON entry points (migration: bmson::ParseError::Utf8 is replaced by Decode(TextDecodeError) and the enum is non_exhaustive with a new Format variant; SourceFormat::from_path now maps .json to Bmson.)

### Refactoring

- *(pipeline)* [**breaking**] rename Outcome warnings to diagnostics (migration: Outcome::warnings is now Outcome::diagnostics, and Outcome::map_warnings is now Outcome::map_diagnostics.)
- *(processor,writer,facade)* [**breaking**] name results and namespace types after their module (migration: BmsonProcessed, BmsonProcessWarning, and BmsonParseError lose their Bmson prefix; BmsProcessor and BmsonProcessor are Processor, BmsWriter is Writer, BmsLayout and BmsonLayout are Layout; prelude no longer exports the bmson Processed, ProcessWarning, ParseError, or Layout.)
- *(bmson-model)* [**breaking**] validate init_bpm on every schema conversion (migration: From<v1::Bmson> for Bmson and From<Bmson> for v1::Bmson are now TryFrom with UpgradeError; v0::TryFromV0Error is replaced by UpgradeError and BmsonError::V0Conversion by BmsonError::Upgrade.)
- *(facade)* [**breaking**] collapse the parse entry points behind Input (migration: `bms::parse_bytes`, `parse_bytes_with`, and
`parse_file` are removed; build an `Input` (`Input::text(text, format)`,)
- *(processor)* [**breaking**] take layouts by value through process_with (migration: `BmsProcessor::process::<L>(bms)` becomes
`process_with(bms, L)`; `BmsonProcessor::process::<L>(bmson)` becomes
`process_with(bmson, L)`; `process_nkeys(bmson, keys)` becomes
`process_with(bmson, GenericLayout::new(keys)?)`;)
- *(bmson-processor)* derive the standard traits on BmsonProcessor
- *(pipeline)* [**breaking**] split Diagnostic::new into required fields plus setters (migration: `Diagnostic::new` now takes `(severity, message)`; chain
`with_span(span)` and `with_code(code)` for the optional fields.)
- *(chart)* [**breaking**] put resolution second in TimingTrack::new (migration: `TimingTrack::new` now takes `(init_bpm, resolution,
bpm_changes, stops)`; move the resolution argument from fourth to second.)
## [0.0.3](https://github.com/brightmeows/bmsrs/compare/bmsrs-bmson-processor-v0.0.2...bmsrs-bmson-processor-v0.0.3) - 2026-11-22

### Bug Fixes

- *(bmson)* map filters to every sliced asset
- *(bmson)* finish clip offset semantics from review feedback

### Documentation

- record format provenance and tighten the sidecar API
- *(readme)* flip install instructions to the released 0.0.2

### Features

- [**breaking**] add KaleidBeat bmson extensions and sidecar (migration: BmsonProcessed replaces the bare Chart return; Bmson and v1::Bmson gain extension fields; LnMode/LnType gain Hln.)
- [**breaking**] keep bmson total percent and add caller-probed asset checks (migration: ChartInfo gains a field, so struct literals need
total_percent; ChartInfo is non-exhaustive by convention (Default exists).)
- feat!(chart): add frame-sequence BGA resources

### Refactoring

- [**breaking**] remove duplicated sidecar views (migration: BmsKaleid.options and BmsonKaleid.speed_events are removed.)
- [**breaking**] tighten the bmson KaleidBeat sidecar (migration: BmsonKaleid.filters is BTreeMap<AudioIndex, SoundFilter>; BmsonProcessed no longer dereferences to Chart.)
## [0.0.2](https://github.com/brightmeows/bmsrs/compare/bmsrs-bmson-processor-v0.0.1...bmsrs-bmson-processor-v0.0.2) - 2026-11-10

### Documentation

- *(readme)* flip install instructions to the released 0.0.1
## [0.0.1](https://github.com/brightmeows/bmsrs/releases/tag/bmsrs-bmson-processor-v0.0.1) - 2026-11-07

### Bug Fixes

- *(bmson-processor)* accumulate same-pulse stop durations per bmson spec
- *(processor)* [**breaking**] GenericLayout 键数收窄为 u8
- *(bmson)* [**breaking**] BGA resource_id 稠密化 (migration: BMSON 谱面的 BgaResource.id 从原始 id 变为稠密下标)
- *(bms)* [**breaking**] 修复索引归一化缺口并接通被丢弃的 BGA/视频数据 (migration: ChartInfo/BgaResource 新增公开字段，Bms 新增 detected_base，)
- *(chart)* 统一负 BPM 校验为非零有限值
- *(bmson-processor)* 每通道独立 BGM discard，不再使用全局 playable_pulses 集合
- *(bmson-processor)* process_default 对异常 keys 报错，保留 DJ.NEXT judge/life deltas
- *(bmson-def,processor)* 统一 LN 类型模型，移除冗余 ln_mode 字段
- *(bmson-def)* v1 BmsonInfo 缺失 ln_type，v0 ln_type 未传递到 ln_type_hint
- *(bmson-processor)* 从完整事件集计算小节线末尾脉冲
- *(bmson-processor)* respect continuation flag in slicing algorithm

### Documentation

- *(readme)* bump pre-release wording to 0.0.1 and simplify the quick start
- sync AGENTS.md files with the event-sync changes
- *(readme)* translate README to English and add a Chinese guide
- *(bmson-processor)* translate comments and docs to English
- *(readme)* 新增根 README
- *(agents)* 修正二轮审计发现的三处 API 不一致
- *(agents)* 修正 AGENTS.md 与源码不一致之处
- 添加目录级 AGENTS.md 并精简子 crate
- *(bmson-processor)* 将文档注释与实现注释中文化
- 全面重写 AGENTS.md 为结构化中文内容
- add AGENTS.md for new crates and update root crate table

### Features

- *(chart)* [**breaking**] represent unspecified long-note type hint as None (migration: `ChartData::ln_type_hint()` returns
`Option<LnTypeHint>`; `with_ln_type_hint` takes `Option<LnTypeHint>`;)
- *(bmson-processor)* [**breaking**] fall back resolution 0 to 240 per spec (migration: deserialize_resolution_nonzero is replaced by
deserialize_resolution, which keeps 0; BmsonProcessWarning gains
ZeroResolutionFallback.)
- *(chart)* [**breaking**] carry stop durations as precise time (migration: EventKind::Stop, StopEvent, and Event::stop change the
duration field from u64 ticks to Duration.)
- *(bms-processor)* [**breaking**] carry RANK, TOTAL, and VOLWAV in ChartInfo (migration: BmsGameplay loses rank, def_ex_rank, total, and
vol_wav; ChartInfo gains judge_rank, total, and volwav_percent.)
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
- *(processor)* 规范外 mode_hint 回退产生 UnknownModeHint 警告
- *(processor)* [**breaking**] 警告体系补 thiserror 与位置上下文，值损坏纳入可观测面 (migration: ProcessWarning/BmsonProcessWarning 多个变体字段
变更；新增 UnparsableValue 变体；有效谱面输出不变，坏值谱面新增
警告信号)
- *(processor)* 新增 process_with_warnings 结构化 warning 收集
- *(chart)* TimingTrack 初始 BPM 校验
- *(chart)* 添加 Event 便利构造器（bar/bgm/bpm/stop/scroll/speed）
- *(chart)* 添加 TimingTrack::simple(bpm) 便利构造器
- 支持负 BPM（逆走）与负 STOP 钳位
- *(bmson-processor)* implement BMSON to Chart conversion
- add workspace metadata, lints, clippy and deny configs
- add workspace structure with empty lib crates

### Other

- Merge pull request #64 from brightmeows/refactor/processor-outcome
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- Merge remote-tracking branch 'origin/main' into HEAD
- *(lints)* 清偿 clippy 1.99 pedantic 债务
- 修复公开工具函数的 clippy lint
- set MSRV 1.85, reorganize clippy lints by category, add expect_used lint
- add package metadata for crates.io publishing

### Performance

- *(chart)* [**breaking**] AudioAsset.path 改用 Arc<Path> 共享所有权
- *(chart)* TimingTrack.build_events() 惰性缓存

### Refactoring

- refactor!(processor): return warnings as Outcome and drop facade wrappers
- *(chart)* [**breaking**] make ChartData and Chart invariant-preserving
- *(chart)* [**breaking**] make the event stream the single timing source
- *(workspace)* [**breaking**] 统一 crate 命名空间为 bmsrs-*
- *(bmson-processor)* 收敛为 Processed 结果类型
- *(processor)* bmson BGA 三层收集收敛为层数组单循环
- *(chart)* [**breaking**] 删除 ChartData.resolution 冗余字段
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
- *(chart)* [**breaking**] replace PlayerSide enum with NoteSide newtype (migration: PlayerSide is removed; use NoteSide. The Player1/Player2
variants are gone; use NoteSide::ONE / NoteSide::TWO, or NoteSide::new for
arbitrary indices. BmsChannel::player_side is renamed to note_side.)
- elevate warn-by-default clippy groups to deny and add restriction lints
- [**breaking**] move layout mapping to processor crates, rename Bme to Beat on BMSON side (migration: Layout types/traits removed from bmsrs-chart.
Import from bms_processor::layout / bmson_processor::layout instead.)
- [**breaking**] replace hardcoded mode layouts with unified mode-family mapping and semantic note model (migration: Chart::lane_count removed; Beat5k/Beat7k/Beat10k/Beat14k/
Popn5k/Popn9k/GenericLayout and Layout trait removed; BmsMapping/BmsonMapping
renamed to BmsLayout/BmsonLayout with new method signatures returning
NoteData; DefaultNoteData renamed to NoteData; NoteData trait renamed to
NoteDataLike; Player::notes_in_lane takes (PlayerSide, Lane, …) instead of
u16; processor process() returns Chart<NoteData> instead of Chart<L::NoteData>.)
- adopt stricter lint config from bms-rs
- *(chart,player,bms-processor,bmson-processor)* [**breaking**] use Duration instead of f64 for time domain API

### Testing

- *(bmson-processor)* 地雷通道伤害值测试
- *(processor)* BMSON 反向滚动事件和 key_channels 不可见音符测试
- *(bmson-processor)* 添加 BMSON 处理器集成测试
- move public-API unit tests to integration test directories
