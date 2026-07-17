# bms-parser

## 定位

BMS 语义分析：flat token 流（无控制流命令）→ 结构化 `Bms` 模型。

## 设计哲学

**`Bms` 保真映射。** `Bms` 字段是 BMS 文件内容的直接结构化表示，不存需要计算或语义解释才能得到的派生值。域级判据见 `bms/AGENTS.md` 职责判据。

| 不做什么 | 理由 |
|----------|------|
| 值解释（路径、URL、邮箱） | 字符串原样保留 |
| 控制流展开 | 由 `bms-control-flow` 处理 |
| 派生值计算 | 归 processor（如伤害 = base36/2、引用→绝对值） |
| 引用-定义配对解析 | `Bms` 保真存引用形态（如 `BpmValue::Reference`）；查表归 processor |

> `Metadata::parse_implicit_subtitle` 是例外：作为 opt-in 工具方法提供，
> 不在 `from_flat_tokens` 中自动调用，调用方显式选择是否执行副标题推断。

## 生命周期

`Messages` 使用两阶段设计：

| 阶段 | 方法 | 作用 |
|------|------|------|
| 1. 收集 | `concat_raw` | 追加 body 字符串到 `(measure, channel)` |
| 2. 最终化 | `finalize(base)` | 解析事件，position-merge，归一化索引 |

`Bms::from_flat_tokens` 自动调用 `finalize`；手动构造时须显式调用。
`finalize` 可重复调用（先 clear），但建议只调用一次。

## Header 分发

| 状态 | 变体 | 存储策略 |
|------|------|----------|
| 存储 | Metadata, Gameplay, Timing, Display, Audio, Visual, `Fallback` | Option(最后胜出) / BTreeMap(按 ID) |
| 跳过 | ControlFlow | 由 `bms-control-flow` 处理 |

### 消息通道处理策略

| 通道类型 | 策略 |
|----------|------|
| BGM（ch 01） | 每行独立处理（多声部支持）|
| 小节长（ch 02） | 最后一行胜出 |
| 选项（ch A6）及其他通道 | position-merge（非 `"00"` 覆盖，`"00"` 保留）|

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| 小节长值为比值 | `1.0` = 4/4，`0.75` = 3/4，`2.0` = 8/4。非百分比。 |
| `finalize(base)` 参数 | 控制索引归一化：Base36 → 大写，Base62 → 保留原大小写 |
| 事件索引均通过 `normalize(base)` | 确保与定义表的 key 大小写匹配 |
| `"00"` 在不同通道语义不同 | note 通道 = 无音符（skip），BPM ch03 = 休止（skip），LN/ext 通道 = 有效值 |
| 归一化契约（全 def 键统一） | `Timing`/`Visual`/`Gameplay` 的所有 `apply` 均接收 `base` 并对索引键 `normalize`（含 `ex_rank_defs`/`change_option_defs`）。`Gameplay::apply` 也已纳入此契约 |
| `detected_base` 字段 | `Bms.detected_base` 由 `from_flat_tokens` 写入 `detect_base()` 结果（**首个** `#BASE` 胜出）。`finalize` 用它归一化所有索引。注意 `Gameplay.base` 是**最后胜出**，多 `#BASE` 文件二者分歧——以 `detected_base` 为准 |
| `non_event_data` 预归一化 | 非事件通道（opacity/ARGB/text/option/seek 等）的合并串在 `finalize_merged` 中拆分为 2-char 值后，逐值按 `base` 归一化，存入 `NonEventData.values`。processor 查表无需再次归一化（消除了此前 F1/F2 标记的重复归一化）|

## Always / Ask / Never

### Always

- 所有索引值（Wav/Bmp/Bpm/Stop/Scroll/Speed/ExRank/ChangeOption）通过 `normalize(base)` 创建
- `Timing`/`Visual`/`Gameplay` 的 `apply` 均接收 `base` 参数并对索引键归一化
- 添加新事件类型时在 `Messages` 和 `lib.rs` re-export 中注册
- 在 AGENTS.md 中记录新事件类型的通道/语义

### Ask

- 修改 `finalize` 的处理策略（影响通道合并行为）
- 新增需要特殊合并语义的通道

### Never

- 在 parser 层自动执行引擎特定的推断（`parse_implicit_subtitle` 等需调用方显式触发）
- 假设 `"00"` 在所有通道都表示“无操作”
- 存储 processor 才需要的派生值（见“设计哲学”派生值禁止）

## 测试

公开 API 测试在 `tests/` 目录中；`merge_channel` 算法测试保留 inline。
