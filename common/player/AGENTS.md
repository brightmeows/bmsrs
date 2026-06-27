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

`TimingCache` 和 `TimingTrack`（来自 `bmsrs-chart`）结果一致——由测试验证。
Cache 是 O(log n) 优化，覆盖 `TimingTrack` 的 O(n) 线性扫描。

**两者故意分离：不要合并。**

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
| TimingCache != TimingTrack | 两者实现不同但结果等价 |
| Duration 是唯一时间接口 | 外部不暴露 tick，转换在内部 |
| 播放器无判定 | 判定逻辑由上层（渲染器/UI）实现 |

## 测试

```bash
cargo test -p bmsrs-player
```
