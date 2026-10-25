# common/ — 格式无关公共层

## 定位

格式无关的谱面数据模型与仿真层。BMS / BMSON 两条管道在此汇合——两边的 processor 都输出 `Chart<T>`。

**零外部依赖。** 纯 `std`，无 `serde`、`thiserror`、`serde_json`。

## 架构

```mermaid
flowchart LR
    BProc[bms-processor] --> Chart["bmsrs-chart (Chart&lt;T&gt;)"]
    BMProc[bmson-processor] --> Chart
    Chart --> Player[bmsrs-player]
```

| Crate | 职责 |
|-------|------|
| `bmsrs-chart` | `Chart<T>` 中央 IR——事件、时序、音符的格式无关模型 |
| `bmsrs-player` | `Chart<T>` 纯仿真层——基于时间的查询接口 |

## 时间模型

| 域 | 类型 | 用途 |
|------|------|------|
| Tick | `u64` | 所有事件位置、停止时长、LN 时长 |
| Time | `Duration` | `AudioAsset.start`、`Chart::duration()`、播放器公开 API |

`TimingTrack`（预计算段，查询 O(log n)）桥接两域；`BpmLookup` 服务构造 `TimingTrack` 前的轻量 BPM 查询。两者同处 `bmsrs-chart`。

## 设计原则

| 原则 | 含义 |
|------|------|
| 零外部依赖 | 纯 `std`——序列化、错误库等由上游 crate 自行引入 |
| 无副作用 | 不含 I/O、渲染、判定、计分——纯数据与查询 |
| 模式信息不存储 | `Chart` 不存储模式族，音符已携带 `(NoteSide, Lane)` |

## 跨格式豁免

有意不做的统一，重判时点均为第三个格式的对应用例到场；
在此之前不要顺手合并。

| 决策 | 理由 |
|------|------|
| `BmsLayout` 与 `BmsonLayout` 不统一为单一 trait | 输入维度本质不同：BMS 为已验证的 `(player, lane)` 对，BMSON 为一维 `x`；两格式独立演进，统一只会引入泛型间接层 |
| 两侧 `ProcessError::InvalidBpm` 不上提到共享 crate | 共享收益仅一个枚举变体，代价是错误类型跨 crate 耦合 |
| `TimingTrack` 正查段与逆查段各自累计时间 | 同一数学变换的两份实现（`new` 与 `merge_inv`）；若出现第三种时间畸变事件（如渐变 BPM），先重判再写第三遍 |

## 测试

```bash
cargo test -p bmsrs-chart
cargo test -p bmsrs-player
cargo bench -p bmsrs-player          # 热路径性能基准
```
