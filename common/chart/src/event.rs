//! Unified event types and format-extension traits.
//!
//! All timed events live in a single `Vec<Event<T, C>>` sorted by tick, enabling
//! uniform iteration and binary search.
//!
//! [`Event`] separates the tick position from the event kind ([`EventKind`]),
//! avoiding a repeated `tick` field on every variant.
//!
//! Generic parameters:
//! - `T`: per-note extension data (carried in [`EventKind::Note`]).
//! - `C`: format-specific custom event type (carried in [`EventKind::Custom`]).
//!
//! # Ordering
//!
//! Processors sort events by `(tick, priority)`. [`EventKind::priority`] defines
//! the order of events at the same tick:
//!
//! `Bar(0)`, then `Note`/`BGA`/`BGM` (1), then `BPM` (2), then `Stop` (3), then
//! `Scroll` (4), then `Speed` (5), then `Custom` (6).

use std::fmt::Debug;

use crate::audio::AudioIndex;
use crate::mode::{Lane, NoteSide};
use crate::note::{NoteAudio, NoteKind, NoteRole};
use crate::visual::{BgaLayer, BgaResourceId};

// EventKind enum

/// Event kind (without the tick position).
///
/// [`Event`] extracts `tick` into the outer struct; this enum only expresses
/// "what happened".
#[derive(Clone, Debug, PartialEq)]
pub enum EventKind<T, C: CustomEvent = NoCustomEvent> {
    /// Playable note.
    Note {
        /// Player side.
        side: NoteSide,
        /// Key / scratch / pedal.
        lane: Lane,
        /// Note kind (normal, long, mine, invisible).
        kind: NoteKind,
        /// Note role during gameplay (normal or release).
        role: NoteRole,
        /// Audio layers played by this note.
        audio: NoteAudio,
        /// Format-specific extension data (`()` when there is none).
        ext: T,
    },
    /// BGM (background music) event: triggers audio without gameplay interaction.
    Bgm {
        /// Index into [`crate::ChartData::audio_assets`].
        audio_index: AudioIndex,
    },
    /// BPM change.
    Bpm {
        /// New BPM value.
        bpm: f64,
    },
    /// Stop / pause event.
    Stop {
        /// Stop duration in ticks.
        duration: u64,
    },
    /// Scroll speed multiplier change.
    Scroll {
        /// Scroll speed multiplier (`1.0` is standard; negative means reverse).
        rate: f64,
    },
    /// Visual note spacing (SPEED) keyframe change.
    ///
    /// Controls the visual density of notes between keyframes through linear
    /// interpolation. Unlike [`Scroll`](Self::Scroll), which affects scroll
    /// speed, SPEED only affects how tightly notes are packed on screen and is
    /// unrelated to timing.
    Speed {
        /// Spacing multiplier at this keyframe.
        rate: f64,
    },
    /// BGA (background animation) display event.
    Bga {
        /// BGA layer targeted by this event.
        layer: BgaLayer,
        /// Index into [`crate::ChartInfo::bga_resources`].
        resource_id: BgaResourceId,
    },
    /// Barline for visual display.
    Bar,
    /// Format-specific custom event.
    Custom(C),
}

impl<T, C: CustomEvent> EventKind<T, C> {
    /// Return the sort priority (lower numbers come first).
    ///
    /// Events at the same tick sort by ascending priority. The convention is:
    ///
    /// `Bar(0)`, then `Note`/`BGA`/`BGM` (1), then `BPM` (2), then `Stop` (3),
    /// then `Scroll` (4), then `Speed` (5), then `Custom` (6).
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Bar => 0,
            Self::Note { .. } | Self::Bga { .. } | Self::Bgm { .. } => 1,
            Self::Bpm { .. } => 2,
            Self::Stop { .. } => 3,
            Self::Scroll { .. } => 4,
            Self::Speed { .. } => 5,
            Self::Custom(_) => 6,
        }
    }

    /// Map the `Custom` variant's payload from `C` to `D`, rebuilding every other
    /// variant as-is.
    ///
    /// Changes the event's [`CustomEvent`] type parameter while keeping non-
    /// `Custom` variants unchanged. Typical use: dropping engine-specific custom
    /// events to convert `Chart<T, BmsCustomEvent>` into
    /// `Chart<T, NoCustomEvent>`.
    ///
    /// # Example
    ///
    /// ```
    /// use bmsrs_chart::{EventKind, NoCustomEvent};
    ///
    /// let kind: EventKind<(), NoCustomEvent> = EventKind::Bar;
    /// let mapped = kind.map_custom(|_| NoCustomEvent);
    /// assert!(matches!(mapped, EventKind::Bar));
    /// ```
    #[must_use]
    pub fn map_custom<F, D: CustomEvent>(self, f: F) -> EventKind<T, D>
    where
        F: FnOnce(C) -> D,
    {
        match self {
            Self::Note {
                side,
                lane,
                kind,
                role,
                audio,
                ext,
            } => EventKind::Note {
                side,
                lane,
                kind,
                role,
                audio,
                ext,
            },
            Self::Bgm { audio_index } => EventKind::Bgm { audio_index },
            Self::Bpm { bpm } => EventKind::Bpm { bpm },
            Self::Stop { duration } => EventKind::Stop { duration },
            Self::Scroll { rate } => EventKind::Scroll { rate },
            Self::Speed { rate } => EventKind::Speed { rate },
            Self::Bga { layer, resource_id } => EventKind::Bga { layer, resource_id },
            Self::Bar => EventKind::Bar,
            Self::Custom(c) => EventKind::Custom(f(c)),
        }
    }

    /// Map the `Note` variant's `ext` field from `T` to `U`, rebuilding every
    /// other variant as-is.
    ///
    /// Changes the event's [`NoteExt`] type parameter while keeping non-`Note`
    /// variants unchanged. Typical use: erasing BMSON extension data to convert
    /// `Chart<BmsonNoteExt>` into `Chart<()>`.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::NonZeroU8;
    /// use bmsrs_chart::{EventKind, Lane, NoteKind, NoteSide};
    ///
    /// let kind = EventKind::<i32, bmsrs_chart::NoCustomEvent>::Note {
    ///     side: NoteSide::P1,
    ///     lane: Lane::Key(NonZeroU8::new(1).unwrap()),
    ///     kind: NoteKind::Normal,
    ///     role: bmsrs_chart::NoteRole::Normal,
    ///     audio: bmsrs_chart::NoteAudio::none(),
    ///     ext: 42,
    /// };
    /// let mapped = kind.map_ext(|_| ());
    /// if let EventKind::Note { ext, .. } = mapped {
    ///     assert_eq!(ext, ());
    /// }
    /// ```
    #[must_use]
    pub fn map_ext<U, F>(self, f: F) -> EventKind<U, C>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Note {
                side,
                lane,
                kind,
                role,
                audio,
                ext,
            } => EventKind::Note {
                side,
                lane,
                kind,
                role,
                audio,
                ext: f(ext),
            },
            Self::Bgm { audio_index } => EventKind::Bgm { audio_index },
            Self::Bpm { bpm } => EventKind::Bpm { bpm },
            Self::Stop { duration } => EventKind::Stop { duration },
            Self::Scroll { rate } => EventKind::Scroll { rate },
            Self::Speed { rate } => EventKind::Speed { rate },
            Self::Bga { layer, resource_id } => EventKind::Bga { layer, resource_id },
            Self::Bar => EventKind::Bar,
            Self::Custom(c) => EventKind::Custom(c),
        }
    }
}

// Manual Eq: all f64 fields (Bpm.bpm, Scroll.rate, Speed.rate) are guaranteed to
// contain no NaN, so PartialEq satisfies Eq's reflexivity requirement.
impl<T: Eq, C: Eq + CustomEvent> Eq for EventKind<T, C> {}

// Event struct

/// A single timed event in a chart.
///
/// Separates the tick position from the event kind. The tick of every event
/// variant is stored here instead of being repeated per variant.
///
/// Processors must insert events in the desired same-tick order before sorting,
/// because the stable sort preserves insertion order.
///
/// # Construction
///
/// ```rust
/// use bmsrs_chart::{Event, EventKind};
///
/// let ev: Event = Event::bar(480);
/// assert_eq!(ev.tick(), 480);
/// assert_eq!(ev.kind.priority(), 0);
/// ```
///
/// # `Eq` guarantee
///
/// `Event` implements [`Eq`] manually. Variants with `f64` fields (`Bpm`,
/// `Scroll`, `Speed`) guarantee that their float fields contain no `NaN`, so
/// `PartialEq` satisfies `Eq`'s reflexivity requirement.
#[derive(Clone, Debug, PartialEq)]
pub struct Event<T: NoteExt = (), C: CustomEvent = NoCustomEvent> {
    /// Tick position.
    tick: u64,
    /// Event kind.
    pub kind: EventKind<T, C>,
}

impl<T: NoteExt, C: CustomEvent> Event<T, C> {
    /// Create a new event.
    #[must_use]
    pub const fn new(tick: u64, kind: EventKind<T, C>) -> Self {
        Self { tick, kind }
    }

    /// Return this event's tick position.
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Return the sort priority (delegates to [`EventKind::priority`]).
    #[must_use]
    pub const fn priority(&self) -> u8 {
        self.kind.priority()
    }

    /// Composite key for stable sorting: `(tick, priority)`.
    ///
    /// Centralizes the "same-tick event ordering" convention (see
    /// [`EventKind::priority`]). Processors should use this method as the only
    /// sorting entry point so the rule does not scatter.
    #[must_use]
    pub const fn sort_key(&self) -> (u64, u8) {
        (self.tick, self.kind.priority())
    }

    // Convenience constructors

    /// Create a barline event.
    #[must_use]
    pub const fn bar(tick: u64) -> Self {
        Self::new(tick, EventKind::Bar)
    }

    /// Create a BGM event.
    #[must_use]
    pub const fn bgm(tick: u64, audio_index: AudioIndex) -> Self {
        Self::new(tick, EventKind::Bgm { audio_index })
    }

    /// Create a BPM change event.
    #[must_use]
    pub const fn bpm(tick: u64, bpm: f64) -> Self {
        Self::new(tick, EventKind::Bpm { bpm })
    }

    /// Create a stop event.
    #[must_use]
    pub const fn stop(tick: u64, duration: u64) -> Self {
        Self::new(tick, EventKind::Stop { duration })
    }

    /// Create a scroll speed change event.
    #[must_use]
    pub const fn scroll(tick: u64, rate: f64) -> Self {
        Self::new(tick, EventKind::Scroll { rate })
    }

    /// Create a SPEED keyframe event.
    #[must_use]
    pub const fn speed(tick: u64, rate: f64) -> Self {
        Self::new(tick, EventKind::Speed { rate })
    }
}

// Manual Eq implementation.
impl<T: NoteExt + Eq, C: CustomEvent + Eq> Eq for Event<T, C> {}

// NoteExt trait

/// Trait for format-specific per-note extension data.
///
/// The built-in `()` implements this trait at zero cost.
pub trait NoteExt: Clone + Debug + PartialEq + Eq + Default {}

impl NoteExt for () {}

// CustomEvent trait

/// Trait for format-specific custom event types (a marker trait).
pub trait CustomEvent: Clone + Debug + PartialEq + Eq {}

/// Sentinel type: no custom events.
///
/// When `C = NoCustomEvent`, the Custom variant carries this type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoCustomEvent;

impl CustomEvent for NoCustomEvent {}
