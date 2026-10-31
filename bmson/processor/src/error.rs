//! Processing warning type definitions.

/// Recoverable warnings produced while processing BMSON.
///
/// Corresponds to `ProcessWarning` on the BMS side. BMSON data is parsed through
/// typed serde structures, so there are no corrupted-value warnings; parse
/// failures are reported during deserialization.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BmsonProcessWarning {
    /// A BPM change with value 0 was ignored.
    #[error("zero BPM change at tick {tick} ignored")]
    ZeroBpmChangeIgnored {
        /// Tick where the event occurs.
        tick: u64,
    },

    /// A negative stop duration was clamped to 0.
    #[error("negative stop duration at tick {tick} clipped to 0 (original {original})")]
    StopDurationClipped {
        /// Tick where the event occurs.
        tick: u64,
        /// Original negative duration.
        original: f64,
    },

    /// A non-standard extension `mode_hint` fell back to the beat layout.
    #[error("unknown mode hint \"{hint}\" fell back to beat layout")]
    UnknownModeHint {
        /// Original extension mode hint string.
        hint: String,
    },

    /// Layered long notes at the same position have mismatched lengths.
    #[error("layered long note at tick {tick} has mismatched lengths {first} and {other}")]
    LayeredNoteLengthMismatch {
        /// Tick where the event occurs.
        tick: u64,
        /// Length of the first note.
        first: u64,
        /// Length of the conflicting note.
        other: u64,
    },
}
