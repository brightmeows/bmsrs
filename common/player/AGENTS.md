# bmsrs-player

## 定位

`Chart<T>` 的纯仿真层。提供基于时间的查询接口，用于游戏渲染和调度。

**无 I/O、渲染、判定、计分。**

## 时间 API

所有公开时间参数使用 `Duration`：

| 方法 | 作用 |
|------|------|
| `Player::new(chart)` | 先排序事件再校验 `ChartData`，返回 `Result` |
| `advance(Duration)` | 纯累加播放时间（O(1)），脉冲按需派生 |
| `seek(Duration)` | 跳到指定位置 |
| `current_time() -> Duration` | 当前播放位置（状态本身）|
| `current_tick() -> u64` | 当前脉冲位置（从时间派生，O(log n)）|

## 查询

| 查询类型 | 作用 |
|----------|------|
| 音符 | `events_in_range` + 调用方 kind 过滤；`notes_for_judgement` 预置判定相关过滤（Normal/Long）|
| BGM/音频 | 时间轴上的音频触发 |
| BGA（三层） | Base / Layer / Layer2 动画事件 |
| 视觉效果 | Bar line、scroll/speed 变化 |

见 `Player` impl 块。

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| Duration 是唯一时间接口 | 外部不暴露 tick，转换在内部 |
| 播放器无判定 | 判定逻辑由上层（渲染器/UI）实现 |
| 时间驱动状态机 | `current_time` 是唯一状态字段；`current_tick()` 每次派生。停止段内多个时刻对应同一脉冲，`tick → time` 不单射——以 tick 为状态会让帧步进的小步 `advance` 永久卡在 stop 内 |
| STOP 期间语义 | `current_tick()` 停住；`current_time()` 继续流逝。需“暂停前”时刻语义时用 `current_tick` 经 `tick_to_duration` 换算 |

## 性能基准

热路径基准（`benches/player.rs`）量化播放循环每帧调用的耗时，
为计时查询与查询路径的复杂度优化提供数据支撑。

```bash
cargo bench -p bmsrs-player --bench player -- --quick  # 快速估算
```

覆盖 `duration_to_tick`、`tick_to_duration`、`advance`、`events_in_range`。

> **性能相关变更必须用基准验证**，不要凭 big-O 推理下结论——
> O(log² n) 的大常数因子在典型谱面上可能比 O(n) 更慢，
> 唯有实测能区分。
