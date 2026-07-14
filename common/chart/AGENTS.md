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
`Chart` 不存储模式信息——音符已携带 `(NoteSide, Lane)`。

## BPM 查询

| 类型 | 方法 | 用途 |
|------|------|------|
| [`BpmLookup`] | `new(init_bpm, &[BpmChange])` | 在构造 [`TimingTrack`] 前查询 BPM（例如 STP 毫秒→脉冲换算），无须完整计时轨 |
| [`BpmLookup`] | `bpm_at_tick(tick) → f64` | 给定脉冲处生效的 BPM（二分查找） |
| [`TimingCache`] | `bpm_at_tick(tick) → f64` | 语义等价，但需预建 `TimingCache` |

`BpmLookup::bpm_at_tick` 与 `TimingCache::bpm_at_tick` 语义一致，前者无须预建 `TimingCache`。

## 非显而易见的规则

| 规则 | 说明 |
|------|------|
| `Chart::duration()` = `last_tick` 的渲染时长 | 最后事件之后无额外尾音 |
| `AudioAsset.start` 是 Time 域 | 用于定义音频素材的触发起点 |
| `TimingTrack::default()` = 120 BPM | 安全默认值，非 0.0；`ChartData` 仍不实现 `Default` |
| `LnTypeHint`/`LnJudgeHint`/`LnLifeHint` | 长音提示的 typed enum，位于 `note.rs`。`LnTypeHint` 由 BMS `#LNMODE` / BMSON `ln_type_hint` 共同填充 |
| `Event` 的 `tick()` 方法 | 统一访问所有变体的 tick 字段 |
| `Event::sort_key()` | `(tick, priority)` 复合键，同脉冲排序的唯一入口 |
| `BgaResource.crop` | 源裁剪矩形与目标偏移（BMS `#BGA`/`#@BGA`）。`None` = 整图。BMSON 不区分裁剪，恒为 `None` |
| `ChartInfo.video` | 背景视频（BMS `#VIDEOFILE`/`#MOVIE`）。BMSON 无此概念，恒为 `None` |
| `VideoAsset` 的 `Eq` | `fps: Option<f64>` 使其无法派生 `Eq`；手动 `impl Eq`（保证不含 NaN，仿 `Damage`）。`colors`/`delay_frames` 用 `u32` 规避 |
| `CropRect` 全整数 | 源矩形 + 偏移均为 `i32`，自然 `Eq` |

## 已知限制（不进 IR）

| 项 | 原因 |
|----|------|
| BMS `#SWBGA`（扫描线过渡动画） | 运行时动画，依赖 renderer 实现，非静态资源数据 |
| BMS `#EXBMP` 逐图透明色 / `#ARGB` 逐图层着色（作为资源属性） | 已通过 `BmsCustomEvent::BgaArgb`（通道 A1-A4）以事件形式承载，不重复建模为资源属性 |
