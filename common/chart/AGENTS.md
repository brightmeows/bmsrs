# bmsrs-chart

## 定位

格式无关的谱面数据模型——格式处理器与播放器之间的中央 IR。

**零外部依赖。** 纯 `std`，无 `serde`、`thiserror`、`serde_json`。

## 时间模型

| 域 | 类型 | 用途 |
|------|------|------|
| Tick | `u64` | 所有事件位置、停止时长、LN 时长 |
| Time | `Duration` | `AudioAsset.start`、`Chart::duration()` |

`resolution` 定义每四分音符的 tick 数（默认 240）。

## 泛型参数

`Chart<T, C>` — 两个泛型参数，均有默认值：

| 参数 | 默认值 | 用途 |
|------|--------|------|
| `T: NoteExt` | `()` | 每音符扩展数据（音量、pan、LN 模式等）|
| `C: CustomEvent` | `NoCustomEvent` | 格式特有自定义事件类型 |

## 事件优先级

`Event` 枚举变体按优先级排序：

```
Bar(0) < Note/BGA/BGM(1) < BPM(2) < Stop(3) < Scroll(4) < Speed(5) < Custom(6)
```

稳定排序保留同 tick 的插入顺序。

## 数据所有者

模式族（mode families）**不在此定义**。每个格式处理器拥有自己的 `layout` 模块。
`Chart` 不存储模式信息——音符已携带 `(PlayerSide, Lane)`。

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| `Chart::duration()` = `last_tick` 的渲染时长 | 最后事件之后无额外尾音 |
| `AudioAsset.start` 是 Time 域 | 用于定义音频素材的触发起点 |
| `TimingTrack::default()` = 120 BPM | 安全默认值，非 0.0；`ChartData` 仍不实现 `Default` |
| `LnTypeHint`/`LnJudgeHint`/`LnLifeHint` | BMSON 长音提示的 typed enum，位于 `note.rs` |
| `Event` 的 `tick()` 方法 | 统一访问所有变体的 tick 字段 |
| `Event::sort_key()` | `(tick, priority)` 复合键，同脉冲排序的唯一入口 |
