//! Parser for BMS (Be-Music Script) format.
//!
//! This crate provides the second stage of the BMS parsing pipeline:
//! converting a flat token stream (no control-flow commands) into a
//! structured [`Bms`] object with typed metadata, resource definitions,
//! timing, and channel messages.

use bms_tokenizer::{BmsBase, BmsHeader, BmsHeaderGameplay, BmsHeaderTiming, BmsMessage, BmsToken};

mod audio;
mod display;
mod gameplay;
mod messages;
mod metadata;
mod timing;
mod visual;

// Re-export all public types from sub-modules.
pub use audio::{Audio, ExWavParams};
pub use display::Display;
pub use gameplay::Gameplay;
pub use messages::{
    BgaEvent, BgmEvent, BpmChange, BpmValue, KeyType, LongNoteEvent, MeasureLength, Messages,
    MineEvent, NoteEvent, Position, ScrollEvent, SpeedEvent, StopEvent, StpEvent,
};
pub use metadata::Metadata;
pub use timing::Timing;
pub use visual::{OwnedExBmpParams, OwnedSwBgaParams, Visual};

// Bms — root document model

/// Structured representation of a BMS file built from a flat token stream.
///
/// Each sub-struct corresponds to exactly one tokenizer header group,
/// enabling the `process_header` method to route directly to sub-module
/// `apply` methods.  The only exception is `#STP` (Timing → Messages),
/// which crosses sub-struct boundaries because it uses the same position
/// model as channel events.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bms {
    /// Song / chart identification metadata (← `BmsHeaderMetadata`).
    pub metadata: metadata::Metadata,
    /// Gameplay behaviour settings (← `BmsHeaderGameplay`).
    pub gameplay: gameplay::Gameplay,
    /// Timing definitions (← `BmsHeaderTiming`).
    pub timing: timing::Timing,
    /// Display assets / difficulty markers (← `BmsHeaderDisplay`).
    pub display: display::Display,
    /// Audio resource definitions (← `BmsHeaderResDefAudio`).
    pub audio: audio::Audio,
    /// Visual / BGA resource definitions (← `BmsHeaderResDefVisual`).
    pub visual: visual::Visual,
    /// Channel message data (raw + parsed events).
    pub messages: messages::Messages,
    /// Unrecognised / engine-specific headers.
    pub fallback_headers: Vec<(String, String)>,
}

impl Bms {
    /// Build a `Bms` from a flat token stream (no control-flow commands).
    ///
    /// Two-pass processing:
    /// 1. A pre-scan locates `#BASE` to determine the numbering base.
    /// 2. All tokens are then processed with the correct base, normalising
    ///    indexed keys (`WavIndex`, `BmpIndex`, etc.) for case-insensitive
    ///    comparison in standard (Base36) BMS files.
    pub fn from_flat_tokens<C: AsRef<str>>(tokens: impl IntoIterator<Item = BmsToken<C>>) -> Self {
        let mut bms = Self::default();

        // Pre-scan for #BASE.  Collect the token iterator first since we
        // need to iterate it twice (once for BASE, once for processing).
        let all_tokens: Vec<_> = tokens.into_iter().collect();
        let bms_base = detect_base(&all_tokens);

        for token in &all_tokens {
            match token {
                BmsToken::Header(header) => bms.process_header(header, bms_base),
                BmsToken::Message(msg) => bms.process_message(msg),
            }
        }

        bms.messages.finalize(bms_base);
        bms
    }

    // Header dispatch — pure routing to sub-module apply() methods

    /// Route a header to the appropriate sub-module's `apply` method.
    ///
    /// `base` is the numbering base determined by `#BASE` (defaults to
    /// [`BmsBase::Base36`]).
    fn process_header<C: AsRef<str>>(&mut self, header: &BmsHeader<C>, base: BmsBase) {
        match header {
            BmsHeader::Metadata(m) => self.metadata.apply(m),
            BmsHeader::Gameplay(g) => self.gameplay.apply(g),
            BmsHeader::Timing(t) => {
                self.timing.apply(t, base);
                // #STP crosses sub-struct boundaries
                if let BmsHeaderTiming::Stp { params } = t {
                    self.messages.stp_events.push(messages::StpEvent {
                        position: messages::Position::new(
                            params.measure,
                            u32::from(params.position),
                            1000,
                        ),
                        duration_ms: params.duration_ms,
                    });
                }
            }
            BmsHeader::ResDefAudio(a) => self.audio.apply(a, base),
            BmsHeader::ResDefVisual(v) => self.visual.apply(v, base),
            BmsHeader::Display(d) => self.display.apply(d),
            BmsHeader::ControlFlow(_) => { /* skipped — not stored in Bms */ }
            BmsHeader::Fallback(f) => self
                .fallback_headers
                .push((f.command.as_ref().to_owned(), f.value.as_ref().to_owned())),
        }
    }

    // Messages

    /// Insert a channel message into the messages container.
    fn process_message<C: AsRef<str>>(&mut self, m: &BmsMessage<C>) {
        self.messages.concat_raw(m);
    }
}

/// Pre-scan tokens for `#BASE` to determine the numbering base.
///
/// Defaults to [`BmsBase::Base36`] when no `#BASE` header is found.
fn detect_base<C: AsRef<str>>(tokens: &[BmsToken<C>]) -> BmsBase {
    for token in tokens {
        if let BmsToken::Header(BmsHeader::Gameplay(BmsHeaderGameplay::Base(b))) = token {
            return *b;
        }
    }
    BmsBase::Base36
}

// Tests — only internal (non-public-API) tests remain here.
// All public-API tests live in tests/.

#[cfg(test)]
mod tests {
    use crate::messages::merge_channel;

    #[test]
    fn merge_channel_basic_example() {
        // Spec example from memo/03:
        //   #00113:11111111   (4 values)
        //   #00113:0022332255224400  (8 values)
        //   #00113:0066        (2 values)
        //   Result: 1122332266224400
        let lines = vec![
            "11111111".to_owned(),
            "0022332255224400".to_owned(),
            "0066".to_owned(),
        ];
        assert_eq!(merge_channel(&lines), "1122332266224400");
    }

    #[test]
    fn merge_channel_single_line_passthrough() {
        let lines = vec!["AABBCC".to_owned()];
        assert_eq!(merge_channel(&lines), "AABBCC");
    }

    #[test]
    fn merge_channel_00_preserves_earlier() {
        // Non-00 from line 1 is preserved when line 2 has "00" at that pos.
        let lines = vec!["11".to_owned(), "00".to_owned()];
        assert_eq!(merge_channel(&lines), "11");
    }

    #[test]
    fn merge_channel_later_overwrites_non_00() {
        // Later line's non-00 overwrites earlier non-00.
        let lines = vec!["11".to_owned(), "22".to_owned()];
        assert_eq!(merge_channel(&lines), "22");
    }
}
