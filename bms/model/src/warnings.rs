//! Channel value parse warnings.

use bmsrs_bms_values::BmsChannel;

/// A channel value cannot be parsed as the type required by that channel.
///
/// Collected by [`Messages::finalize`](crate::Messages::finalize) while parsing
/// channel events: the value shape (a 2-character Base62 chunk) is valid but
/// does not satisfy that channel's parse rule (for example, ch03 requires
/// hexadecimal and the value contains a non-hexadecimal character). The event
/// is skipped, and the warning carries location information for the charter.
///
/// Missing-reference warnings (undefined WAV/BPM/STOP keys) are carried by the
/// processor layer's `ProcessWarning`; corrupted-value warnings are carried by
/// this type and forwarded by the processor as
/// `ProcessWarning::UnparsableValue`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unparsable channel value \"{value}\" (channel {channel:?}) at measure {measure}")]
pub struct ChannelValueWarning {
    /// Measure number.
    pub measure: u16,
    /// Channel type.
    pub channel: BmsChannel,
    /// Original 2-character value that could not be parsed.
    pub value: String,
}
