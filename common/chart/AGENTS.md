# bmsrs-chart

## Role

Format-agnostic chart data model: the central IR between format processors and
the player.

**Zero external dependencies.** Pure `std`, with no `serde`, `thiserror`, or
`serde_json`.

## Time model

| Domain | Type | Use |
|--------|------|-----|
| Tick | `u64` | all event positions, LN durations |
| Stop pause | `Duration` | `EventKind::Stop`'s `duration`; precise time, not quantized to ticks |
| Time | `Duration` | `AudioAsset.start`, `Player::duration()` |
| Musical time | `Duration` | `TimingTrack::tick_to_musical_duration`, excluding STOP pauses; used for audio slicing |

`resolution` defines ticks per quarter note (default 240).

`ChartData` stores the scalar timing configuration (`init_bpm`, `resolution`)
and the event stream. BPM changes and stops live only in
`EventKind::Bpm`/`EventKind::Stop`; `bmsrs-player` derives its `TimingTrack`
from them. There is no second timing source to keep in sync.

## Generic parameters

`Chart<T, C>` has two generic parameters, both with defaults:

| Parameter | Default | Use |
|-----------|---------|-----|
| `T: NoteExt` | `()` | per-note extension data (volume, pan, LN mode, and so on) |
| `C: CustomEvent` | `NoCustomEvent` | format-specific custom event type |

Core note fields:

| Type | Purpose |
|------|---------|
| `Note<T>` | playable note payload: side, lane, kind, role, audio, ext; `EventKind::Note(Note<T>)` |
| `NoteAudio` | audio layer set: `None` / `One(AudioIndex)` / `Many(Box<[AudioIndex]>)`; `Many` has at least two layers |
| `NoteRole` | `Normal` / `Release`, orthogonal to `NoteKind`, survives erasing `NoteExt` |
| `AudioIndex` / `BgaResourceId` | dedicated newtypes preventing audio and BGA indices from being assigned to each other |
| `SwBgaResourceId` / `BgaResourceKind` | `SwBgaResourceId` indexes `ChartInfo.swbga_resources`; `BgaResource.kind()` infers Static/Video from the file extension |

## Event priority

`Event` enum variants sort by priority:

```
Bar(0) < Note/BGA/BGM(1) < BPM(2) < Stop(3) < Scroll(4) < Speed(5) < Custom(6)
```

The stable sort preserves insertion order at the same tick.

`JudgeRank` shares priority 2 with BPM: a same-tick judgment width change takes
effect before the notes at that tick.

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
| `Player::duration()` is the render duration of `last_tick` | no extra tail after the last event |
| `AudioAsset.start` is in the Time domain | defines the trigger offset of an audio asset |
| `TimingTrack::default()` is 120 BPM | a safe default, not 0.0; `ChartData` still does not implement `Default` |
| `LnTypeHint`/`LnJudgeHint`/`LnLifeHint` | typed enums for long-note hints in `note.rs`. `LnTypeHint` is filled by both BMS `#LNMODE` and BMSON `ln_type_hint` |
| chart-level `ln_type_hint` is `Option` | `None` means the source did not declare it (BMS without `#LNMODE`, BMSON without the field); the unspecified state travels to the caller, matching bmz's `long_note_mode_defined` |
| `Event`'s `tick()` method | uniform access to the tick field of every variant |
| `Event::sort_key()` | the `(tick, priority)` composite key and the only same-tick ordering entry point |
| `ChartData::new(init_bpm, resolution, events, audio_assets)` | sorts events, validates, and returns the only public constructor result; fields are private |
| `ChartData::validate()` | private constructor guard: positive finite `init_bpm`, positive `resolution`, event ordering, positive finite BPM events, finite SCROLL/SPEED rates, and note/BGM audio indices |
| `ChartData::try_map_events` / `try_filter_map_events` | re-sort and revalidate after a transformation; return `Result` |
| `Chart::new(song, chart, data)` | validates BGA resource references; the embedded `ChartData` is already valid |
| `Player::new` | infallible; consumes an already validated `Chart` |
| BPM/Stop single source | `EventKind::Bpm`/`EventKind::Stop` are authoritative; `TimingTrack::from_events` derives the timing cache |
| Negative BPM | changes are dropped with a processor warning; a negative initial BPM is rejected. Negative SCROLL rates, which reverse only the visual scrolling, are supported. bmz-player clamps invalid BPM changes to `1.0` instead; deliberately not followed (beatoraja drops them) |
| Same-tick STOP | BMS keeps the last stop (matching beatoraja); BMSON sums durations per the bmson specification; `TimingTrack::new` applies last-wins defensively. jbms-parser keeps the last bmson stop instead; the specification (and bmz-player) sum, and bmsrs follows the specification |
| Stop durations are time, not ticks | `EventKind::Stop`/`StopEvent` hold `Duration`; processors convert `#STOP` through the tick's BPM and `#STP`/BMSON pulses directly, so no tick quantization remains |
| `ChartInfo.judge_rank` / `total` / `volwav_percent` | raw source values (`JudgeRankSpec` carries the `BmsRank`/`DefExRank`/`BmsonJudgeRank` kind); the percentage-to-window mapping is engine policy and stays with the caller |
| `ChartInfo.miss_text` | nanasi `#TEXT00`; belongs to the miss system and never becomes a channel event |
| `EventKind::Bga` clears layers | `resource_id: None` means the layer shows nothing (undefined BMP reference); `Some` indexes `bga_resources` |
| `TextDisplay` carries its body | the resolved text travels in the event (`Arc<str>`); an empty string clears the display; `text_index` no longer exists |
| `ChartInfo.swbga_resources` | key-bound BGA animation definitions (`#SWBGAxx`); `BmsCustomEvent::BgaKeyBound.swbga_id` indexes this table, and pattern frames reference `bga_resources` (`None` is a blank frame) |
| `ChartData::with_start_margin` | shifts the chart so the first judgeable note lands at least `margin` after tick 0 (beatoraja `setStartNoteTime`); the leading bar line stays at tick 0, and late-enough charts return unchanged |
| `NoteExt` LN hint accessors | `ln_type_hint`/`ln_judge_hint`/`ln_life_hint` default to `None`; BMSON overrides them per note |
| `judge_multiplier` / `life_multiplier` | `Option<f64>`; `None` means unspecified by the source, `Some(1.0)` means standard |
| `ChartInfo.key_count` | total playable keys; `None` means not provided or not inferable |
| `ChartInfo.subartists` / `title_image` | aligned with BMSON v2's chart-level ownership; BMS `#SUBARTIST` also maps to `subartists` |
| `TimingTrack::tick_to_musical_duration` | unlike `tick_to_duration`, excludes STOP pauses; used for audio slicing |
| `BgaResource.crop` | source crop rectangle and target offset (BMS `#BGA`/`#@BGA`). `None` means the whole image. BMSON has no crop concept and always uses `None` |
| `BgaResource` has no `id` | the vector index into `bga_resources` is the unique resource id; events reference it through `BgaResourceId` |
| `BgaResource.kind()` | `Static` or `Video`, inferred from the file extension (`#BMPxx` may reference a video) |
| `ChartInfo.video` | background video (BMS `#VIDEOFILE`/`#MOVIE`). BMSON has no such concept and always uses `None` |
| `VideoAsset` equality | `fps: Option<f64>` keeps the type at `PartialEq`; no manual `Eq` promise. `colors`/`delay_frames` use `u32` to avoid the issue |
| `CropRect` is all integers | source rectangle and offset are `i32`, so `Eq` derives naturally |

## Known limitations (not in the IR)

| Item | Reason |
|------|--------|
| BMS `#EXBMP` per-image transparent color / `#ARGB` per-layer tinting (as resource attributes) | already carried as events through `BmsCustomEvent::BgaArgb` (channels A1-A4); not modeled again as resource attributes |
