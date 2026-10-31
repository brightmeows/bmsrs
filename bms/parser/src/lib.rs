//! Parser for the BMS (Be-Music Script) format.
//!
//! This crate provides the second stage of the BMS parsing pipeline: it turns a
//! flat token stream (without control-flow commands) into a structured [`Bms`]
//! object containing typed metadata, resource definitions, timing, and channel
//! messages.

mod audio;
mod cst;
mod display;
mod gameplay;
mod messages;
mod metadata;
mod timing;
mod visual;
mod warnings;

// Re-export all public types from the submodules.
pub use audio::{Audio, OwnedExWavParams};
pub use bmsrs_bms_values::*;
pub use cst::{
    CstParse, CstWarning, CstWarningKind, ErrorStrategy, LineToken, tokenize_lines,
    tokenize_lines_with,
};
pub use display::Display;
pub use gameplay::Gameplay;
pub use messages::{
    BgaEvent, BgmEvent, BmsBgaLayer, BpmChange, BpmValue, KeyType, LongNoteEvent, MeasureLength,
    Messages, MineEvent, NonEventData, NoteEvent, Position, ScrollEvent, SpeedEvent, StopEvent,
    StpEvent,
};
pub use metadata::Metadata;
pub use timing::Timing;
pub use visual::{OwnedExBmpParams, OwnedSwBgaParams, Visual};
pub use warnings::ChannelValueWarning;

// Bms: root document model

/// Structured representation of a BMS file built from a flat token stream.
///
/// Each sub-structure corresponds to exactly one tokenizer header group, so the
/// `process_header` method can route directly to a submodule's `apply` method.
/// The only exception is `#STP` (Timing to Messages), which crosses
/// sub-structure boundaries because it uses the same position model as channel
/// events.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bms {
    /// Song and chart identity metadata (from `BmsHeaderMetadata`).
    pub metadata: metadata::Metadata,
    /// Gameplay behavior settings (from `BmsHeaderGameplay`).
    pub gameplay: gameplay::Gameplay,
    /// Timing definitions (from `BmsHeaderTiming`).
    pub timing: timing::Timing,
    /// Display resources and difficulty markers (from `BmsHeaderDisplay`).
    pub display: display::Display,
    /// Audio resource definitions (from `BmsHeaderResDefAudio`).
    pub audio: audio::Audio,
    /// Visual and BGA resource definitions (from `BmsHeaderResDefVisual`).
    pub visual: visual::Visual,
    /// Channel message data (raw data plus parsed events).
    pub messages: messages::Messages,
    /// Unrecognized or engine-specific header commands.
    pub fallback_headers: Vec<(String, String)>,
}

impl Bms {
    /// Build `Bms` from a flat token stream (without control-flow commands).
    ///
    /// Two-phase processing:
    /// 1. Pre-scan for `#BASE` to determine the radix base.
    /// 2. Process every token with the correct base, normalizing index keys
    ///    (`WavIndex`, `BmpIndex`, and so on) for case-insensitive comparison
    ///    in standard (Base36) BMS files.
    pub fn from_flat_tokens(tokens: impl IntoIterator<Item = BmsToken>) -> Self {
        let mut bms = Self::default();

        // Pre-scan #BASE. Collect the token iterator first because it is
        // iterated twice (once for BASE, once for processing).
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

    // Header dispatch: pure routing to the submodule apply() methods

    /// Route a header command to the corresponding submodule's `apply` method.
    ///
    /// `base` is the radix base determined by `#BASE` (defaults to
    /// [`BmsBase::Base36`]).
    fn process_header(&mut self, header: &BmsHeader, base: BmsBase) {
        match header {
            BmsHeader::Metadata(m) => self.metadata.apply(m),
            BmsHeader::Gameplay(g) => self.gameplay.apply(g, base),
            BmsHeader::Timing(t) => {
                self.timing.apply(t, base);
                // #STP crosses sub-structure boundaries.
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
                .push((f.command.clone(), f.value.clone())),
        }
    }

    // Messages

    /// Insert a channel message into the message container.
    fn process_message(&mut self, m: &BmsMessage) {
        self.messages.concat_raw(m);
    }
}

/// Pre-scan tokens for `#BASE` to determine the radix base.
///
/// Defaults to [`BmsBase::Base36`] when no `#BASE` header command is found.
fn detect_base(tokens: &[BmsToken]) -> BmsBase {
    tokens
        .iter()
        .find_map(|token| {
            if let BmsToken::Header(BmsHeader::Gameplay(BmsHeaderGameplay::Base(b))) = token {
                Some(*b)
            } else {
                None
            }
        })
        .unwrap_or(BmsBase::Base36)
}
