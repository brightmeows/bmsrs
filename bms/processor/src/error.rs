//! Processing warning type definitions.

use bmsrs_bms_parser::ChannelValueWarning;

/// Recoverable warnings produced while processing BMS.
///
/// These warnings indicate anomalies in the chart, but a valid `Chart` can still
/// be produced.
///
/// Missing-reference warnings (`Missing*`) and corrupted-value warnings
/// ([`ProcessWarning::UnparsableValue`]) together form the complete observable
/// surface of the processor layer; control-flow structure problems belong to
/// `bmsrs-bms-syntax` diagnostics.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProcessWarning {
    /// A note references a key that is not defined in `#WAV`.
    #[error(
        "note at tick {tick} (player {player}, lane {lane}) references undefined WAV key {key}"
    )]
    MissingWavDefinition {
        /// Undefined WAV key (Base36 numeric value).
        key: u16,
        /// Player number of the referencing note.
        player: u8,
        /// Lane number of the referencing note.
        lane: u8,
        /// Tick where the event occurs.
        tick: u64,
    },

    /// A long-note start has no matching end.
    #[error("long note start at tick {tick} (player {player}, lane {lane}) has no matching end")]
    UnterminatedLongNote {
        /// Player number.
        player: u8,
        /// Lane number.
        lane: u8,
        /// Tick of the long-note start.
        tick: u64,
    },

    /// A BPM change with an invalid value (zero, NaN, or infinity) was ignored.
    ///
    /// The filter predicate matches [`TimingTrack`](bmsrs_chart::TimingTrack)
    /// construction ([`BpmChange::is_valid`](bmsrs_chart::BpmChange::is_valid));
    /// invalid values enter neither the timing track nor the unified event
    /// stream.
    #[error("invalid BPM change at tick {tick} ignored (value {bpm})")]
    InvalidBpmChangeIgnored {
        /// Tick where the event occurs.
        tick: u64,
        /// Ignored BPM value.
        bpm: f64,
    },

    /// A negative stop duration was clamped to 0.
    #[error("negative stop duration at tick {tick} clipped to 0 (computed {original})")]
    StopDurationClipped {
        /// Tick where the event occurs.
        tick: u64,
        /// Computed negative duration (before clamping).
        original: f64,
    },

    /// A BPM change references a key that is not defined in `#BPMxx`.
    #[error("BPM change at tick {tick} references undefined BPM key {key}")]
    MissingBpmDefinition {
        /// Undefined BPM key (Base36 numeric value).
        key: u16,
        /// Tick where the event occurs.
        tick: u64,
    },

    /// A stop event references a key that is not defined in `#STOPxx`.
    #[error("stop event at tick {tick} references undefined STOP key {key}")]
    MissingStopDefinition {
        /// Undefined STOP key (Base36 numeric value).
        key: u16,
        /// Tick where the event occurs.
        tick: u64,
    },

    /// A channel value cannot be parsed as the type required by that channel;
    /// the event was skipped.
    ///
    /// Forwards the parser layer's [`ChannelValueWarning`] (including
    /// measure/channel/value location information).
    #[error(transparent)]
    UnparsableValue(#[from] ChannelValueWarning),
}
