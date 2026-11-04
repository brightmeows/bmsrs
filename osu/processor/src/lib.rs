//! osu!mania to `Chart` conversion processor.
//!
//! [`OsuProcessor`] parses `.osu` text through [`rosu_map`] and converts
//! osu!mania beatmaps into the format-agnostic [`Chart`]:
//!
//! ```text
//! .osu bytes -> rosu_map::Beatmap -> OsuProcessor::process -> Chart<OsuNoteExt, OsuCustomEvent>
//! ```
//!
//! # Supported scope
//!
//! - Only `Mode: 3` (osu!mania) is accepted.
//! - Circles map to [`NoteKind::Normal`](bmsrs_chart::NoteKind::Normal) and
//!   holds map to [`NoteKind::Long`](bmsrs_chart::NoteKind::Long).
//! - Column positions are computed from the `x` coordinate and the
//!   `CircleSize` key count, using [`NoteSide::P1`](bmsrs_chart::NoteSide::P1).
//! - Hit samples go into [`OsuNoteExt`]; chart-level metadata, difficulty
//!   settings, Kiai, and breaks go into [`OsuCustomEvent`].
//! - Sliders, spinners, standard/taiko/catch, storyboards, and videos are out
//!   of scope for this iteration.
//!
//! # Example
//!
//! ```
//! use bmsrs_osu_processor::OsuProcessor;
//!
//! let chart = OsuProcessor::from_str(
//!     "osu file format v14\n\
//!      [General]\n\
//!      AudioFilename: audio.ogg\n\
//!      Mode: 3\n\
//!      [Difficulty]\n\
//!      CircleSize: 4\n\
//!      [TimingPoints]\n\
//!      0,500,4,2,0,100,1,0\n\
//!      [HitObjects]\n\
//!      64,192,1000,1,0,0:0:0:0:\n",
//! )
//! .unwrap();
//! assert_eq!(chart.data().audio_assets().len(), 1);
//! ```

mod convert;
mod custom_event;
mod error;
mod note_ext;

use std::path::Path;

use bmsrs_chart::Chart;

pub use crate::custom_event::{OsuChartMeta, OsuCustomEvent, OsuMode};
pub use crate::error::{OsuError, OsuProcessError};
pub use crate::note_ext::{OsuHitSample, OsuHitSampleName, OsuNoteExt, OsuSampleBank};

/// osu!mania beatmap processor.
pub struct OsuProcessor;

impl OsuProcessor {
    /// Convert an already parsed osu! beatmap into a [`Chart`].
    ///
    /// # Errors
    ///
    /// Returns [`OsuProcessError`] for non-mania modes, invalid key counts,
    /// missing timing points or audio, sliders/spinners, and time overflow.
    pub fn process(
        map: &rosu_map::Beatmap,
    ) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuProcessError> {
        convert::process(map)
    }

    /// Parse and convert from `.osu` byte content.
    ///
    /// # Errors
    ///
    /// Returns [`OsuError`] when reading or decoding the `.osu` content fails
    /// or the conversion fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuError> {
        let map: rosu_map::Beatmap = rosu_map::from_bytes(bytes)?;
        Self::process(&map).map_err(OsuError::from)
    }

    /// Parse and convert from `.osu` text.
    ///
    /// # Errors
    ///
    /// Returns [`OsuError`] when decoding the `.osu` text or the conversion
    /// fails.
    #[expect(
        clippy::should_implement_trait,
        reason = "returns Chart, not OsuProcessor; FromStr cannot express the conversion"
    )]
    pub fn from_str(content: &str) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuError> {
        let map: rosu_map::Beatmap = rosu_map::from_str(content)?;
        Self::process(&map).map_err(OsuError::from)
    }

    /// Parse and convert from an `.osu` file path.
    ///
    /// # Errors
    ///
    /// Returns [`OsuError`] when reading the file, decoding the `.osu`
    /// content, or converting fails.
    pub fn from_path(
        path: impl AsRef<Path>,
    ) -> Result<Chart<OsuNoteExt, OsuCustomEvent>, OsuError> {
        let map: rosu_map::Beatmap = rosu_map::from_path(path)?;
        Self::process(&map).map_err(OsuError::from)
    }
}
