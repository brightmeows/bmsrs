# bms-processor

## 定位

`Bms` → 格式无关 `Chart` 的转换处理器。
通过 `BmsLayout` 模式族解耦 BMS 的玩家/键位映射。

processor 承担**信息损失型转换**——合并优先级（`#BGA`/`#BMP`）、引用解析（WAV/BPM）、查表（def 表）均在此层。域级判据见 `bms/AGENTS.md` 职责判据。

## 关键转换

| 转换 | 输入 | 输出 | 说明 |
|------|------|------|------|
| 音符 | `NoteEvent` + `LongNoteEvent` + `MineEvent` | `Event::Note` | LNOBJ 消耗的 note 从普通音符中排除 |
| 时序 | `BpmChange` + `StopEvent` + `MeasureTable` | `TimingTrack` | 小节长通过 `length_ratio` (f64) 计算 |
| BGM | `BgmEvent` | `Event::Bgm` | LNOBJ 终点标记也作为 BGM 播放 |
| BGA | `BgaEvent` | `Event::Bga` | 四层（Base / Poor / Layer / Layer2）|
| 显示元数据 | `BmsHeaderDisplay` | `ChartInfo` | `#BANNER`/`#BACKBMP`/`#STAGEFILE`/`#PREVIEW` ∊ `build_metadata` |
| SCROLL | `ScrollEvent` | `Event::Scroll` | 卷轴速度倍率 |
| SPEED | `SpeedEvent` | `Event::Speed` | 视觉间距关键帧，线性插值 |
| 长音模式提示 | `#LNMODE` | `LnTypeHint` | 1:1 映射 Ln/Cn/Hcn（`ln_type_hint` 方法）|
| BGA 裁剪 | `#BGA`/`#@BGA` | `BgaResource.crop` | `build_bmp_map` 合并三命名空间；`#@BGA`(w/h) 归一为右下角 |
| 视频资源 | `#VIDEOFILE`/`#MOVIE` | `ChartInfo.video` | `#VIDEOFILE` 循环优先于 `#MOVIE` 单次；附 `#VIDEOf/s`/`#VIDEOCOLORS`/`#VIDEODLY` |
| 视频 SEEK | `#SEEK` 定义 + ch 05 | `BmsCustomEvent::VideoSeek` | 查 `seek_defs` 取毫秒（非 base36 原值）；未定义 id 跳过 |
| `non_event_data` 自定义事件 | `NonEventData.values` | `BmsCustomEvent` | 所有 2-char 索引已在 parser 层归一化，查表直接命中，无需再次 `normalize(base)` |

## 长音模式（自动检测）

| 模式 | 检测条件 | 配对算法 |
|------|----------|----------|
| LNOBJ | `#LNOBJ` 已定义 | 常规音符与 LNOBJ 标记配对的起止对 |
| LNTYPE 1 (RDM) | 默认（或 `#LNTYPE 1`） | ch 51–69，过滤 `"00"`，连续配对 |
| LNTYPE 2 (MGQ) | `#LNTYPE 2` | ch 51–69，`"00"` = LN 终点 |

## 定位转换

`MeasureTable` 预计算每小节的累计 tick 偏移，以处理变拍子：

```text
tick = measure_starts[measure] + numer * measure_len / denom
```

每小节长度 = `resolution * 4 * length_ratio`，其中 `length_ratio` 为 `#xxx02` 值。

## 停止转换

| 来源 | 转换公式 |
|------|----------|
| `#STOP` (ch 09) | `raw / 192.0 * resolution * 4` ticks |
| `#STP` (header) | `ms → ticks via bpm_at_tick` |

## 模式族

模式族是零大小类型，实现 `BmsLayout` trait。每个族是一个 `(player, lane) → Option<NoteData>` 的映射表。

标准族见公开 `layout` 模块。`Bme` 族覆盖 5K/7K/10K/14K——`#PLAYER` 不影响映射（与主流引擎一致）。

## 公开模块

| 模块 | 内容 |
|------|------|
| `layout` | `BmsLayout` trait + 标准模式族（`Bme` 等） |
| `custom_event` | `BmsCustomEvent` 枚举——BMS 特有事件（BGA opacity / 文本 / 选项等）作为 `Chart` 的 `CustomEvent` 类型参数 |

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| 默认 BPM 130 | 符合 BMS 规范，非 `0.0` |
| 事件排序 | Bar(0) → Note/BGA/BGM(1) → BPM(2) → Stop(3) → Scroll(4) → Speed(5) → Custom(6) |
| LNOBJ 终点 BGM | 终点标记过判定线时播放定义的 WAV |
| LNOBJ 下 ch51-69 | 与 LNOBJ 互斥（memo/10 未定义）；不丢弃，作为普通可见音符保留 |
| 地雷 `damage` | `MineEvent.raw_value`（`Option<u16>`）经 `mine_damage()` 转换：`Some(1295)` → INFINITY，`Some(n)` → n/2，`None` → 1.0 |
| `process_default` 使用 `Bme` | 而非基于 `#PLAYER` 推断；PMS 等模式需显式指定 |
| 转换步骤归属 `BmsConverter` | `collect_*`/`build_*` 是内部 `BmsConverter`（私有）的方法，非自由函数 |
| `non_event_data` 索引归一化 | parser 层完成 | processor 消费 | `non_event_data` 的每个 2-char 值已在 parser 的 `finalize_merged` 中按 `detected_base` 归一化，processor 查表直接命中。`BmsConverter.base` 字段已因不复需要而移除 |
| `bpm_at_tick` 来源 | 委托 `BpmLookup::new(init_bpm, changes).bpm_at_tick(tick)`（原 local 重复已消除）。STP→tick 仍需在 `TimingTrack` 构造前完成，但算法统一 |
| BGA 裁剪源路径 | `#BGA`/`#@BGA` 的 `bmp_index` 为十进制源编号，经 base36 数值匹配 `bmp_files` 键（如 `"01"`→1）。源路径不可解析的裁剪 id 被跳过 |

## Deliberate 延迟（解析了但不在此转换）

下列字段由 parser 解析存入 `Bms`，但 processor **有意不**转换到 `Chart`——
其语义属引擎特定判定/血量逻辑，归 player 层。`Chart` 对应槽位保留默认值。

| 字段 | 来源 | 延迟原因 |
|------|------|----------|
| `gameplay.rank` | `#RANK` | RANK→判定窗口倍率的换算引擎特定（beatoraja≠LR2），不属 processor |
| `gameplay.def_ex_rank` | `#DEFEXRANK` | 同上，精细判定难度 |
| `gameplay.total` | `#TOTAL` | 血槽最大增量，血量逻辑归 player |
| `gameplay.vol_wav` | `#VOLWAV` | 主音量，播放/混音归 player |
| `gameplay.ex_rank_defs` | `#EXRANKxx` | 已解析但 A0 通道当前直传 base36 值（`JudgeOverride.rank`），未查此表——逐位置判定覆盖需配合上述判定系统整体设计，**已知未闭合** |

## Always / Ask / Never

### Always

- 新事件类型在 `BmsConverter` 的 `collect_*` 方法中注册 + 在 `BmsProcessor::process` 中调用
- 更新 `Event::tick()` match 表达式以包含新变体

### Ask

- 新增模式族（修改 `layout` 模块的通道映射）
- 修改事件优先级顺序（影响同 tick 事件排序）

### Never

- 在处理器中引入 I/O、渲染、判定逻辑
- 修改 `Chart` 的数据模型（属于 `bmsrs-chart` crate）
