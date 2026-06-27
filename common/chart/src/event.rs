//! Unified event enumeration, format-extension traits.
//!
//! All timed events live in a single `Vec<Event<T, C>>` sorted by tick,
//! enabling unified iteration and binary-search queries.
//!
//! Generic parameters:
//! - `T` — per-note extension data (carried on [`Event::Note`]).
//! - `C` — format-specific custom event type (carried on [`Event::Custom`]).
//!
//! # Sorting
//!
//! Processors sort events by tick using a stable sort, so the relative
//! order of events at the same tick is determined by insertion order.
//! The convention is:
//!
//! `Bar → Note/BGA/BGM → BPM → Stop → Scroll → Custom`

use std::fmt::Debug;

use crate::mode::{Lane, NoteSide};
use crate::note::NoteKind;
use crate::visual::BgaLayer;

// Event enum

/// A single timed event in a chart.
///
/// Every variant has a `tick` field (or method) giving its absolute position
/// in the chart timeline.  Use [`Event::tick`] for uniform access.
///
/// Processors must insert events in the desired same-tick order before
/// sorting, because stable sort preserves insertion order.
#[derive(Clone, Debug, PartialEq)]
pub enum Event<T, C: CustomEvent = NoCustomEvent> {
    /// A playable note.
    Note {
        /// Tick position.
        tick: u64,
        /// Which player side.
        side: NoteSide,
        /// Which key/scratch/pedal.
        lane: Lane,
        /// Note kind (normal, long, mine, invisible).
        kind: NoteKind,
        /// Audio asset index into [`crate::ChartData::audio_assets`], or `None`.
        audio_index: Option<u32>,
        /// Format-specific extension data (use `()` for no extensions).
        ext: T,
    },
    /// A BGM (background music) event — audio trigger, no gameplay interaction.
    Bgm {
        /// Tick position.
        tick: u64,
        /// Index into [`crate::ChartData::audio_assets`].
        audio_index: u32,
    },
    /// A BPM change.
    Bpm {
        /// Tick position.
        tick: u64,
        /// New BPM value.
        bpm: f64,
    },
    /// A stop / pause event.
    Stop {
        /// Tick position.
        tick: u64,
        /// Stop duration in ticks.
        duration: u64,
    },
    /// A scroll-speed multiplier change.
    Scroll {
        /// Tick position.
        tick: u64,
        /// Scroll speed multiplier (`1.0` = normal, negative = reverse).
        rate: f64,
    },
    /// A BGA (background animation) display event.
    Bga {
        /// Tick position.
        tick: u64,
        /// Which BGA layer this event targets.
        layer: BgaLayer,
        /// Index into [`crate::ChartInfo::bga_resources`].
        resource_id: u32,
    },
    /// A bar line for visual display.
    Bar {
        /// Tick position.
        tick: u64,
    },
    /// Format-specific custom event.
    Custom(C),
}

impl<T, C: CustomEvent> Event<T, C> {
    /// Returns the tick position of this event uniformly, regardless of variant.
    #[must_use]
    pub fn tick(&self) -> u64 {
        match self {
            Self::Custom(c) => c.tick(),
            Self::Note { tick, .. }
            | Self::Bgm { tick, .. }
            | Self::Bpm { tick, .. }
            | Self::Stop { tick, .. }
            | Self::Scroll { tick, .. }
            | Self::Bga { tick, .. }
            | Self::Bar { tick } => *tick,
        }
    }
}

// NoteExt trait

/// Trait for format-specific per-note extension data.
///
/// The built-in `()` implements this trait with no overhead.
pub trait NoteExt: Clone + Debug + PartialEq + Default {}

impl NoteExt for () {}

// CustomEvent trait

/// Trait for format-specific custom event types.
///
/// Custom events participate in the unified sorted timeline.
/// The processor must insert them in the desired same-tick order before
/// the stable sort.
pub trait CustomEvent: Clone + Debug + PartialEq {
    /// Tick position of this custom event.
    fn tick(&self) -> u64;
}

/// Sentinel type: no custom events.
///
/// The `Custom` variant is simply never constructed when
/// `C = NoCustomEvent`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoCustomEvent;

impl CustomEvent for NoCustomEvent {
    fn tick(&self) -> u64 {
        0
    }
}
