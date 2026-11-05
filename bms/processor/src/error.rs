//! Processing warning type definitions.

use std::error::Error;
use std::fmt;

use bmsrs_bms_model::ChannelValueWarning;
use bmsrs_pipeline::SourceSpan;

/// Recoverable warning produced while processing BMS.
///
/// The warning carries an optional source span and a typed
/// [`ProcessWarningKind`]. Missing-reference warnings and corrupted-value
/// warnings together form the complete observable surface of the processor
/// layer; control-flow structure problems belong to `bmsrs-bms-syntax`
/// diagnostics.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProcessWarning {
    /// Whole-line source span, when the warning can be located in the source.
    pub span: Option<SourceSpan>,
    /// Warning payload.
    pub kind: ProcessWarningKind,
}

impl ProcessWarning {
    /// Construct a warning.
    #[must_use]
    pub const fn new(span: Option<SourceSpan>, kind: ProcessWarningKind) -> Self {
        Self { span, kind }
    }
}

impl fmt::Display for ProcessWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl Error for ProcessWarning {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl From<ChannelValueWarning> for ProcessWarning {
    fn from(warning: ChannelValueWarning) -> Self {
        Self::new(warning.span, ProcessWarningKind::UnparsableValue(warning))
    }
}

impl From<ProcessWarning> for bmsrs_pipeline::Diagnostic {
    fn from(warning: ProcessWarning) -> Self {
        Self::new(
            bmsrs_pipeline::Severity::Warning,
            warning.span,
            warning.to_string(),
            Some(bmsrs_pipeline::DiagnosticCode::BmsProcess),
        )
    }
}

/// Warning payload.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProcessWarningKind {
    /// A note references a key that is not defined in `#WAV`.
    #[error(
        "note at tick {tick} (player {player}, lane {lane}) references undefined WAV key {key}"
    )]
    MissingWavDefinition {
        /// Undefined WAV key (numeric value in the chart's base).
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

    /// A BGA event references an undefined BMP index; the layer is cleared.
    #[error("BGA event at tick {tick} references undefined BMP key {key}; the layer is cleared")]
    MissingBmpDefinition {
        /// Undefined BMP key (numeric value in the chart's base).
        key: u16,
        /// Tick where the event occurs.
        tick: u64,
    },

    /// A channel value cannot be parsed as the type required by that channel;
    /// the event was skipped.
    #[error(transparent)]
    UnparsableValue(#[from] ChannelValueWarning),
}
