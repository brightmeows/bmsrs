# bmsrs-player

## 定位

`Chart<T>` 的纯仿真层。提供基于时间的查询接口，用于游戏渲染和调度。

**无 I/O、渲染、判定、计分。**

## 时间 API

所有公开时间参数使用 `Duration`：

| 方法 | 作用 |
|------|------|
| `advance(Duration)` | 前进播放游标 |
| `seek(Duration)` | 跳到指定位置 |
| `current_time() -> Duration` | 当前播放位置 |

## TimingCache

预计算 BPM segment 和停止时长累积和，实现 O(log n) 二分查找。

`TimingCache` 定义在 `bmsrs-chart`（与 `TimingTrack` 同 crate），
本 crate 通过 `use bmsrs_chart::TimingCache` 引入。它是
`TimingTrack::tick_to_duration` / `duration_to_tick` 的预计算加速版本——
两者语义一致、结果等价，由 chart 的测试验证。

`TimingTrack`（O(n) 线性）适合一次性构造；`TimingCache`（O(log n) 二分）
适合播放器实时查询、处理器批量切片等频繁换算场景。两者是同一时间换算
概念的两种形态，同处 chart 以避免下游各自重新实现。

## 查询

| 查询类型 | 作用 |
|----------|------|
| 音符 | 可按 lane 或判定范围过滤 |
| BGM/音频 | 时间轴上的音频触发 |
| BGA（三层） | Base / Layer / Layer2 动画事件 |
| 视觉效果 | Bar line、scroll/speed 变化 |

见 `Player` impl 块。

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| TimingCache 与 TimingTrack 同源 | 均在 bmsrs-chart，语义一致；Cache 是预计算加速版 |
| Duration 是唯一时间接口 | 外部不暴露 tick，转换在内部 |
| 播放器无判定 | 判定逻辑由上层（渲染器/UI）实现 |

## 测试

```bash
cargo test -p bmsrs-player
```
