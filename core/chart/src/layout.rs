//! Layout trait and default game-mode layouts.
//!
//! A [`Layout`] defines the lane structure for a specific game mode
//! (beat-7k, popn-9k, etc.). Each processor crate defines its own
//! mapping trait ([`BmsonMapping`][crate::layout] in `bmson-processor`,
//! `BmsMapping` in `bms-processor`) that extends `Layout` with
//! format-specific channel-to-lane resolution.

use crate::note::DefaultNoteData;

/// Defines the lane structure for a game mode.
///
/// Each implementing type represents a specific game mode (BME 7K,
/// PMS 9K, etc.). The associated type [`Layout::NoteData`] determines
/// what data each note carries.
///
/// The processors (`bmson-processor`, `bms-processor`) define their own
/// traits that extend `Layout` with format-specific mapping methods.
///
/// # Example
///
/// ```
/// use bmsrs_chart::layout::{Beat7k, Layout};
///
/// let layout = Beat7k;
/// assert_eq!(layout.lane_count(), 8);
/// ```
pub trait Layout: Clone {
    /// The note data type produced when processing charts with this layout.
    type NoteData: crate::note::NoteData;

    /// Total number of lanes (keys + scratch + second player).
    fn lane_count(&self) -> u16;
}

/// Beat-7K layout: 7 keys + 1 scratch = 8 lanes (1 player).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Beat7k;

impl Layout for Beat7k {
    type NoteData = DefaultNoteData;
    fn lane_count(&self) -> u16 {
        8
    }
}

/// Beat-5K layout: 5 keys + 1 scratch = 6 lanes (1 player).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Beat5k;

impl Layout for Beat5k {
    type NoteData = DefaultNoteData;
    fn lane_count(&self) -> u16 {
        6
    }
}

/// Beat-14K layout: (7 keys + 1 scratch) × 2 players = 16 lanes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Beat14k;

impl Layout for Beat14k {
    type NoteData = DefaultNoteData;
    fn lane_count(&self) -> u16 {
        16
    }
}

/// Beat-10K layout: (5 keys + 1 scratch) × 2 players = 12 lanes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Beat10k;

impl Layout for Beat10k {
    type NoteData = DefaultNoteData;
    fn lane_count(&self) -> u16 {
        12
    }
}

/// Pop'n 9K layout: 9 keys = 9 lanes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Popn9k;

impl Layout for Popn9k {
    type NoteData = DefaultNoteData;
    fn lane_count(&self) -> u16 {
        9
    }
}

/// Pop'n 5K layout: 5 keys = 5 lanes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Popn5k;

impl Layout for Popn5k {
    type NoteData = DefaultNoteData;
    fn lane_count(&self) -> u16 {
        5
    }
}

/// Generic n-keys layout: `keys` lanes, left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericLayout {
    /// Number of keys (= lane count).
    pub keys: u16,
}

impl Layout for GenericLayout {
    type NoteData = DefaultNoteData;
    fn lane_count(&self) -> u16 {
        self.keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beat7k_lane_count() {
        assert_eq!(Beat7k.lane_count(), 8);
    }

    #[test]
    fn beat5k_lane_count() {
        assert_eq!(Beat5k.lane_count(), 6);
    }

    #[test]
    fn beat14k_lane_count() {
        assert_eq!(Beat14k.lane_count(), 16);
    }

    #[test]
    fn beat10k_lane_count() {
        assert_eq!(Beat10k.lane_count(), 12);
    }

    #[test]
    fn popn9k_lane_count() {
        assert_eq!(Popn9k.lane_count(), 9);
    }

    #[test]
    fn popn5k_lane_count() {
        assert_eq!(Popn5k.lane_count(), 5);
    }

    #[test]
    fn generic_layout_lane_count() {
        let layout = GenericLayout { keys: 4 };
        assert_eq!(layout.lane_count(), 4);
    }

    #[test]
    fn layout_clone_and_eq() {
        let a = Beat7k;
        let b = a;
        assert_eq!(a, b);
    }
}
