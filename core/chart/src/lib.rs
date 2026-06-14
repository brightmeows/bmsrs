//! Format-agnostic rhythm game chart data model.
//!
//! [`Chart`] is the central type: an intermediate representation produced
//! by format processors (`bmson-processor`, `bms-processor`) and consumed
//! by the player (`bmsrs-player`).
//!
//! # Time model
//!
//! All event positions are absolute [`u64`] ticks. The global
//! [`resolution`][Chart::resolution] defines ticks per quarter note
//! (default 240). Wall-clock seconds are derived via
//! [`TimingTrack::tick_to_seconds`][timing::TimingTrack::tick_to_seconds].
//!
//! # Generic note data
//!
//! [`Chart<T>`][Chart] is parameterised by a [`NoteData`] type `T`.
//! The default is [`DefaultNoteData`] (lane + kind only).
//! Custom types can carry format-specific extensions (volume, pan, LN mode, etc.).
//!
//! # Example
//!
//! ```
//! use bmsrs_chart::{
//!     Chart, ChartMetadata, DefaultNoteData, Note, NoteKind, TimingTrack,
//!     Bga,
//! };
//!
//! let chart = Chart {
//!     metadata: ChartMetadata {
//!         title: "Test".into(),
//!         ..Default::default()
//!     },
//!     resolution: 240,
//!     lane_count: 8,
//!     timing: TimingTrack {
//!         init_bpm: 120.0,
//!         bpm_changes: vec![],
//!         stops: vec![],
//!     },
//!     judge_multiplier: 1.0,
//!     life_multiplier: 1.0,
//!     notes: vec![Note {
//!         tick: 0,
//!         audio: None,
//!         data: DefaultNoteData {
//!             lane: 0,
//!             kind: NoteKind::Normal,
//!         },
//!     }],
//!     bgm: vec![],
//!     audio_assets: vec![],
//!     bar_lines: vec![],
//!     scroll_events: vec![],
//!     bga: Bga::default(),
//! };
//! assert_eq!(chart.resolution, 240);
//! assert_eq!(chart.notes.len(), 1);
//! ```

pub mod audio;
pub mod layout;
pub mod note;
pub mod timing;
pub mod visual;

pub use audio::{AudioAsset, BgmEvent};
pub use layout::{Beat5k, Beat7k, Beat10k, Beat14k, GenericLayout, Layout, Popn5k, Popn9k};
pub use note::{DefaultNoteData, Note, NoteData, NoteKind};
pub use timing::{BpmChange, StopEvent, TimingTrack};
pub use visual::{BarLine, Bga, BgaResource, BgaTimelineEvent, ScrollChangeEvent};

/// Chart metadata (song and difficulty information).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChartMetadata {
    /// Song title.
    pub title: String,
    /// Song subtitle (may be empty).
    pub subtitle: String,
    /// Primary artist.
    pub artist: String,
    /// Additional contributors (e.g. `["music:composer", "chart:charter"]`).
    pub subartists: Vec<String>,
    /// Song genre.
    pub genre: String,
    /// Chart name / difficulty label (e.g. `"HYPER"`, `"ANOTHER"`).
    pub chart_name: String,
    /// Numeric difficulty level (typically 1–12 for beat modes).
    pub level: u64,
}

/// Format-agnostic rhythm game chart.
///
/// Produced by format processors and consumed by the player.
/// All event vectors should be sorted by tick ascending — processors
/// guarantee this, and the player relies on it for binary-search queries.
///
/// # Fields
///
/// | Field | Source |
/// |-------|--------|
/// | `metadata` | BMSON `SongInfo`/`ChartInfo` or BMS `Metadata` |
/// | `resolution` | BMSON `resolution` or processor-chosen (240 for BMS) |
/// | `lane_count` | Layout used during processing |
/// | `timing` | BMSON `bpm_events`/`stop_events` or BMS `timing`/`messages` |
/// | `notes` | BMSON `sound_channels` or BMS `messages` |
/// | `audio_assets` | BMSON sliced sound channels or BMS WAV table |
#[derive(Clone, Debug, PartialEq)]
pub struct Chart<T: NoteData = DefaultNoteData> {
    /// Song and chart metadata.
    pub metadata: ChartMetadata,

    /// Ticks per quarter note (pulse resolution).
    pub resolution: u64,

    /// Number of playable lanes (from the Layout used during processing).
    pub lane_count: u16,

    /// Timing track for tick ↔ seconds conversion.
    pub timing: TimingTrack,

    /// Judgement window multiplier (`1.0` = normal).
    pub judge_multiplier: f64,

    /// Life gauge multiplier (`1.0` = normal).
    pub life_multiplier: f64,

    /// Playable notes, sorted by tick ascending.
    pub notes: Vec<Note<T>>,

    /// BGM audio events (audio-only, no gameplay), sorted by tick.
    pub bgm: Vec<BgmEvent>,

    /// Audio assets referenced by notes and BGM events.
    pub audio_assets: Vec<AudioAsset>,

    /// Bar line positions for visual display.
    pub bar_lines: Vec<BarLine>,

    /// Scroll-speed change events.
    pub scroll_events: Vec<ScrollChangeEvent>,

    /// Background animation data.
    pub bga: Bga,
}

impl<T: NoteData> Chart<T> {
    /// Returns the last tick position of any event in the chart
    /// (notes, BGM, bar lines, BGA).
    ///
    /// Useful for computing total chart duration.
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        let note_last = self.notes.last().map_or(0, |n| n.tick);
        let bgm_last = self.bgm.last().map_or(0, |e| e.tick);
        let bar_last = self.bar_lines.last().map_or(0, |b| b.tick);
        let bga_final = self
            .bga
            .events
            .iter()
            .chain(self.bga.layer_events.iter())
            .chain(self.bga.poor_events.iter())
            .map(|e| e.tick)
            .max()
            .unwrap_or(0);
        let scroll_last = self.scroll_events.last().map_or(0, |s| s.tick);
        note_last
            .max(bgm_last)
            .max(bar_last)
            .max(bga_final)
            .max(scroll_last)
    }

    /// Returns the total duration of the chart in seconds.
    #[must_use]
    pub fn duration_seconds(&self) -> f64 {
        self.timing
            .tick_to_seconds(self.last_tick(), self.resolution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_chart(notes: Vec<Note>, bgm: Vec<BgmEvent>) -> Chart {
        Chart {
            metadata: ChartMetadata::default(),
            resolution: 240,
            lane_count: 8,
            timing: TimingTrack {
                init_bpm: 120.0,
                bpm_changes: vec![],
                stops: vec![],
            },
            judge_multiplier: 1.0,
            life_multiplier: 1.0,
            notes,
            bgm,
            audio_assets: vec![],
            bar_lines: vec![],
            scroll_events: vec![],
            bga: Bga::default(),
        }
    }

    #[test]
    fn last_tick_empty_chart() {
        let chart = make_test_chart(vec![], vec![]);
        assert_eq!(chart.last_tick(), 0);
    }

    #[test]
    fn last_tick_from_notes() {
        let chart = make_test_chart(
            vec![Note {
                tick: 960,
                audio: None,
                data: DefaultNoteData {
                    lane: 0,
                    kind: NoteKind::Normal,
                },
            }],
            vec![],
        );
        assert_eq!(chart.last_tick(), 960);
    }

    #[test]
    fn last_tick_from_bgm_beyond_notes() {
        let chart = make_test_chart(
            vec![Note {
                tick: 480,
                audio: None,
                data: DefaultNoteData {
                    lane: 0,
                    kind: NoteKind::Normal,
                },
            }],
            vec![BgmEvent {
                tick: 1920,
                audio: 0,
            }],
        );
        assert_eq!(chart.last_tick(), 1920);
    }

    #[test]
    fn duration_seconds_constant_bpm() {
        let chart = make_test_chart(
            vec![Note {
                tick: 480,
                audio: None,
                data: DefaultNoteData {
                    lane: 0,
                    kind: NoteKind::Normal,
                },
            }],
            vec![],
        );
        // 480 ticks at 120 BPM, resolution 240: 480/240 * 0.5 = 1.0s
        assert!((chart.duration_seconds() - 1.0).abs() < 1e-9);
    }
}
