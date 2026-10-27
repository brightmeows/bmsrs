# osu-processor

## 定位

`rosu-map::Beatmap` → 格式无关 `Chart` 的 osu!mania 处理器。
解析由 `rosu-map` 负责，本 crate 只做转换与验证。

## 管道

```text
.osu bytes → rosu_map::Beatmap → OsuProcessor::process → Chart<OsuNoteExt, OsuCustomEvent>
```

## 支持范围

- 只接受 `Mode: 3`（osu!mania）。
- circle → `NoteKind::Normal`；hold → `NoteKind::Long`。
- `x` 坐标与 `CircleSize` 计算列号，统一使用 `NoteSide::P1`。
- hit sample 存入 `OsuNoteExt`；元数据、难度、Kiai、Break 存入
  `OsuCustomEvent`。
- 非继承 timing point → `EventKind::Bpm` 与 `OsuCustomEvent::Timing`。
- 继承 timing point 的滚动速度 → `EventKind::Scroll`。
- 不生成 `Bar`；渲染器可按 `Bpm` 与 `Timing` 自行生成小节线。

## 严格性

| 情况 | 行为 |
|------|------|
| 非 mania | `UnsupportedMode` |
| `CircleSize` 不是 1..=18 的整数 | `InvalidKeyCount` |
| 缺少非继承 timing point | `MissingTimingPoint` |
| `AudioFilename` 为空 | `MissingAudioFilename` |
| 负音符时间 | `NegativeHitObjectTime` |
| hold 结束不晚于开始 | `InvalidHold` |
| slider / spinner | `UnsupportedObject` |
| 毫秒到 tick 溢出 | `TimeOverflow` |

负 timing point 时间钳到 0，用于确定 tick 0 的初始 BPM。

## 时间模型

- tick 分辨率固定为 1000（每四分音符）。
- 非正时间映射到 tick 0。
- `TimingTrack` 只包含 BPM 变更，不包含 STOP。
- `AudioLeadIn` 与 `PreviewTime` 只记录在 `OsuChartMeta`，不改变 tick；
  渲染器/播放器需要自行应用音频前导静音。

## 已知限制

- `rosu-map` 的便捷解析入口会静默忽略单行语义错误；本 crate 只能验证
  成功进入 `Beatmap` 的数据。
- 不解析故事板与视频；`ChartInfo.video` 恒为 `None`。
- 不支持 standard / taiko / catch、slider、spinner、PP 与判定窗口。

## 测试

```bash
cargo test -p osu-processor
```

测试只使用 `tests/fixtures/` 下的自制合成谱面，不引入真实谱面、音频或图片。
