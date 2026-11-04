//! Error types.

use bmsrs_chart::{ChartDataError, TimingTrackError};

use thiserror::Error;

use crate::custom_event::OsuMode;

/// Errors that can occur while parsing and converting an osu! beatmap.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OsuError {
    /// An I/O error occurred while reading or parsing the `.osu` content.
    ///
    /// The `rosu-map` convenience entry points unify read and decode errors
    /// into [`std::io::Error`]; single-line semantic errors are silently
    /// ignored in the default configuration.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Parsing succeeded, but the beatmap cannot be converted into a `Chart`.
    #[error(transparent)]
    Process(#[from] OsuProcessError),
}

/// Errors that can occur while converting osu!mania into a `Chart`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OsuProcessError {
    /// The beatmap mode is not osu!mania.
    #[error("unsupported osu! mode {mode:?}; only mania is supported")]
    UnsupportedMode {
        /// The actual mode.
        mode: OsuMode,
    },
    /// `CircleSize` is not an integer from 1 to 18.
    #[error(
        "invalid mania key count derived from CircleSize {circle_size}; expected an integer in 1..=18"
    )]
    InvalidKeyCount {
        /// The raw `CircleSize` value.
        circle_size: f32,
    },
    /// There is no uninherited timing point, so the timing data cannot be built.
    #[error("beatmap has no uninherited timing point")]
    MissingTimingPoint,
    /// `AudioFilename` is empty.
    #[error("beatmap has no audio filename")]
    MissingAudioFilename,
    /// A hit object time is negative.
    #[error("hit object {index} has negative time {time} ms")]
    NegativeHitObjectTime {
        /// The hit object index in `[HitObjects]`.
        index: usize,
        /// The negative time value in milliseconds.
        time: f64,
    },
    /// A hold ends no later than it starts.
    #[error("hit object {index} is an invalid hold: start {start} ms, end {end} ms")]
    InvalidHold {
        /// The hit object index in `[HitObjects]`.
        index: usize,
        /// The hold start time in milliseconds.
        start: f64,
        /// The hold end time in milliseconds.
        end: f64,
    },
    /// A slider or spinner appears in a mania beatmap.
    #[error("hit object {index} is a {kind}, which is not valid for osu!mania")]
    UnsupportedObject {
        /// The hit object index in `[HitObjects]`.
        index: usize,
        /// The object kind name.
        kind: &'static str,
    },
    /// A millisecond time cannot be represented as a tick.
    #[error("time {time} ms cannot be represented as a tick")]
    TimeOverflow {
        /// The time value in milliseconds that triggered the overflow.
        time: f64,
    },
    /// Timing validation failed.
    #[error(transparent)]
    Timing(#[from] TimingTrackError),
    /// Chart construction or validation failed.
    #[error(transparent)]
    Chart(#[from] ChartDataError),
}
