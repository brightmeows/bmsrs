# bmsrs-osu-processor

## Role

osu!mania processor from `rosu-map::Beatmap` to the format-agnostic `Chart`.
Parsing is delegated to `rosu-map`; this crate only converts and validates.

## Pipeline

```text
.osu bytes -> rosu_map::Beatmap -> OsuProcessor::process -> Chart<OsuNoteExt, OsuCustomEvent>
```

## Supported scope

- Only `Mode: 3` (osu!mania) is accepted.
- circle -> `NoteKind::Normal`; hold -> `NoteKind::Long`.
- Column numbers are computed from the `x` coordinate and `CircleSize`;
  `NoteSide::P1` is always used.
- Hit samples go into `OsuNoteExt`; metadata, difficulty settings, Kiai, and
  breaks go into `OsuCustomEvent`.
- Uninherited timing points -> `EventKind::Bpm` and `OsuCustomEvent::Timing`.
- Inherited timing point scroll speed -> `EventKind::Scroll`.
- No `Bar` is generated; renderers can derive measure lines from `Bpm` and
  `Timing` themselves.

## Strictness

| Case | Behavior |
|------|----------|
| Non-mania mode | `UnsupportedMode` |
| `CircleSize` not an integer in 1..=18 | `InvalidKeyCount` |
| No uninherited timing point | `MissingTimingPoint` |
| Empty `AudioFilename` | `MissingAudioFilename` |
| Negative note time | `NegativeHitObjectTime` |
| Non-finite hold end time | `NonFiniteHold` |
| Hold ends no later than it starts | converted to `NoteKind::Normal` (lazer clamps such holds to zero length, which play as rice) |
| slider / spinner | `UnsupportedObject` |
| Millisecond-to-tick overflow | `TimeOverflow` |

Negative timing point times are clamped to 0 and determine the initial BPM at
tick 0.

## Time model

- The tick resolution is fixed at 1000 (pulses per quarter note).
- Non-positive times map to tick 0.
- BPM changes live only in `EventKind::Bpm` events; no STOP is generated.
- `AudioLeadIn` and `PreviewTime` are recorded only in `OsuChartMeta` and do
  not shift ticks; renderers and players apply the audio lead-in themselves.

## Known limitations

- The `rosu-map` convenience entry points silently ignore single-line
  semantic errors; this crate can only validate data that made it into the
  `Beatmap`.
- Storyboards and videos are not parsed; `ChartInfo.video` is always `None`.
- standard / taiko / catch, sliders, spinners, PP, and judgement windows are
  not supported.

## Tests

```bash
cargo test -p bmsrs-osu-processor
```

Tests only use the synthetic chart under `tests/fixtures/`; no real charts,
audio, or images are included.
