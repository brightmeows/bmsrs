# bmsrs-chart

## Role

Format-agnostic chart data model: the central IR between format processors and
the player.

**Zero external dependencies.** Pure `std`, with no `serde`, `thiserror`, or
`serde_json`.

## Time model

| Domain | Type | Use |
|--------|------|-----|
| Tick | `u64` | all event positions, stop durations, LN durations |
| Time | `Duration` | `AudioAsset.start`, `Chart::duration()` |
| Musical time | `Duration` | `TimingTrack::tick_to_musical_duration`, excluding STOP pauses; used for audio slicing |

`resolution` defines ticks per quarter note (default 240).

## Generic parameters

`Chart<T, C>` has two generic parameters, both with defaults:

| Parameter | Default | Use |
|-----------|---------|-----|
| `T: NoteExt` | `()` | per-note extension data (volume, pan, LN mode, and so on) |
| `C: CustomEvent` | `NoCustomEvent` | format-specific custom event type |

Core note fields:

| Type | Purpose |
|------|---------|
| `NoteAudio` | audio layer set: `None` / `One(AudioIndex)` / `Many(Box<[AudioIndex]>)`; `Many` has at least two layers |
| `NoteRole` | `Normal` / `Release`, orthogonal to `NoteKind`, survives erasing `NoteExt` |
| `AudioIndex` / `BgaResourceId` | dedicated newtypes preventing audio and BGA indices from being assigned to each other |

## Event priority

`Event` enum variants sort by priority:

```
Bar(0) < Note/BGA/BGM(1) < BPM(2) < Stop(3) < Scroll(4) < Speed(5) < Custom(6)
```

The stable sort preserves insertion order at the same tick.

## Data ownership

Mode families are **not defined here**. Each format processor owns its own
`layout` module. `Chart` does not store mode information; notes already carry
`(NoteSide, Lane)`.

## BPM lookup

| Type | Method | Use |
|------|--------|-----|
| [`BpmLookup`] | `new(init_bpm, &[BpmChange])` | query BPM before constructing [`TimingTrack`] (for example, STP milliseconds to ticks) without a full timing track |
| [`BpmLookup`] | `bpm_at_tick(tick) -> f64` | BPM in effect at a tick (binary search) |

`BpmLookup::bpm_at_tick` matches `TimingTrack::bpm_at_tick` semantically; the
former needs no full timing track and serves lightweight pre-construction
queries.

## Non-obvious rules

| Rule | Explanation |
|------|-------------|
| `Chart::duration()` is the render duration of `last_tick` | no extra tail after the last event |
| `AudioAsset.start` is in the Time domain | defines the trigger offset of an audio asset |
| `TimingTrack::default()` is 120 BPM | a safe default, not 0.0; `ChartData` still does not implement `Default` |
| `LnTypeHint`/`LnJudgeHint`/`LnLifeHint` | typed enums for long-note hints in `note.rs`. `LnTypeHint` is filled by both BMS `#LNMODE` and BMSON `ln_type_hint` |
| `Event`'s `tick()` method | uniform access to the tick field of every variant |
| `Event::sort_key()` | the `(tick, priority)` composite key and the only same-tick ordering entry point |
| `ChartData::validate()` | checks event ordering, then note/BGM audio indices and BGA resource indices; `UnsortedEvents` is a separate error |
| `judge_multiplier` / `life_multiplier` | `Option<f64>`; `None` means unspecified by the source, `Some(1.0)` means standard |
| `ChartInfo.key_count` | total playable keys; `None` means not provided or not inferable |
| `ChartInfo.subartists` / `title_image` | aligned with BMSON v2's chart-level ownership; BMS `#SUBARTIST` also maps to `subartists` |
| `TimingTrack::tick_to_musical_duration` | unlike `tick_to_duration`, excludes STOP pauses; used for audio slicing |
| `BgaResource.crop` | source crop rectangle and target offset (BMS `#BGA`/`#@BGA`). `None` means the whole image. BMSON has no crop concept and always uses `None` |
| `BgaResource` has no `id` | the vector index into `bga_resources` is the unique resource id; events reference it through `BgaResourceId` |
| `ChartInfo.video` | background video (BMS `#VIDEOFILE`/`#MOVIE`). BMSON has no such concept and always uses `None` |
| `VideoAsset`'s `Eq` | `fps: Option<f64>` prevents deriving `Eq`; implemented manually (guaranteed no NaN, following `Damage`). `colors`/`delay_frames` use `u32` to avoid the issue |
| `CropRect` is all integers | source rectangle and offset are `i32`, so `Eq` derives naturally |

## Known limitations (not in the IR)

| Item | Reason |
|------|--------|
| BMS `#SWBGA` (scanline transition animation) | runtime animation that depends on the renderer, not static resource data |
| BMS `#EXBMP` per-image transparent color / `#ARGB` per-layer tinting (as resource attributes) | already carried as events through `BmsCustomEvent::BgaArgb` (channels A1-A4); not modeled again as resource attributes |
