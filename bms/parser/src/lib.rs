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

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use bms_tokenizer::{BmpIndex, BmsTokenizer, BpmIndex, WavIndex};

    fn parse_tokens(input: &str) -> Vec<BmsToken<&str>> {
        BmsTokenizer::new()
            .tokenize::<Vec<_>, &str>(input)
            .into_iter()
            .filter_map(|(_, res)| res.ok())
            .collect()
    }

    #[test]
    fn header_override_last_wins() {
        let tokens = parse_tokens("#TITLE First\n#TITLE Second");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.metadata.title.as_deref(), Some("Second"));
    }

    #[test]
    fn wav_definitions_stored() {
        let tokens = parse_tokens("#WAV01 a.wav\n#WAV02 b.wav");
        let bms = Bms::from_flat_tokens(tokens);
        let id1: WavIndex = "01".try_into().unwrap();
        let id2: WavIndex = "02".try_into().unwrap();
        assert_eq!(
            bms.audio.wav_files.get(&id1).map(String::as_str),
            Some("a.wav")
        );
        assert_eq!(
            bms.audio.wav_files.get(&id2).map(String::as_str),
            Some("b.wav")
        );
    }

    #[test]
    fn message_storage() {
        let tokens = parse_tokens("#00101:1122");
        let bms = Bms::from_flat_tokens(tokens);
        let ch = bms_tokenizer::BmsChannel::from_raw("01").unwrap();
        let measure_map = bms.messages.raw.get(&1);
        assert!(measure_map.is_some());
        assert_eq!(
            measure_map
                .and_then(|m| m.get(&ch))
                .and_then(|v| v.first().map(String::as_str)),
            Some("1122")
        );
        // Single-line: exactly one entry in the Vec
        assert_eq!(measure_map.and_then(|m| m.get(&ch).map(Vec::len)), Some(1));
    }

    #[test]
    fn message_concat_same_channel() {
        let tokens = parse_tokens("#00101:1122\n#00101:3344");
        let bms = Bms::from_flat_tokens(tokens);
        let ch = bms_tokenizer::BmsChannel::from_raw("01").unwrap();
        let measure_map = bms.messages.raw.get(&1);
        // BGM channels store each line separately (polyphony support).
        let lines = measure_map.and_then(|m| m.get(&ch));
        assert_eq!(lines, Some(&vec!["1122".to_owned(), "3344".to_owned()]));
        // Two lines produce 2 BGM events per line (total 4), each with
        // its own per-line denominator.
        assert_eq!(bms.messages.bgm_events.len(), 4);
        assert_eq!(bms.messages.bgm_events[0].position.denom, 2);
        assert_eq!(bms.messages.bgm_events[2].position.denom, 2);
    }

    #[test]
    fn fallback_headers_stored() {
        let tokens = parse_tokens("#MYEXT abc123");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.fallback_headers.len(), 1);
        assert_eq!(
            bms.fallback_headers[0],
            ("MYEXT".to_owned(), "abc123".to_owned())
        );
    }

    #[test]
    fn mixed_headers_and_messages() {
        let tokens = parse_tokens(
            "#TITLE My Song\n#ARTIST composer\n#BPM 180\n#WAV01 kick.wav\n#00111:11223344",
        );
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.metadata.title.as_deref(), Some("My Song"));
        assert_eq!(bms.metadata.artist.as_deref(), Some("composer"));
        assert_eq!(bms.timing.bpm, Some(180.0));
        let wav_id: WavIndex = "01".try_into().unwrap();
        assert_eq!(
            bms.audio.wav_files.get(&wav_id).map(String::as_str),
            Some("kick.wav")
        );
        let ch = bms_tokenizer::BmsChannel::from_raw("11").unwrap();
        assert_eq!(
            bms.messages
                .raw
                .get(&1)
                .and_then(|m| m.get(&ch))
                .and_then(|v| v.first().map(String::as_str)),
            Some("11223344")
        );
    }

    #[test]
    fn bpm_and_bpm_def_separate_fields() {
        let tokens = parse_tokens("#BPM 120\n#BPM01 180.0\n#EXBPM02 200.0");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.timing.bpm, Some(120.0));
        let id1: BpmIndex = "01".try_into().unwrap();
        let id2: BpmIndex = "02".try_into().unwrap();
        assert_eq!(bms.timing.bpm_defs.get(&id1), Some(&180.0));
        assert_eq!(bms.timing.bpm_defs.get(&id2), Some(&200.0));
    }

    #[test]
    fn default_is_all_none_empty() {
        let bms = Bms::default();
        assert!(bms.metadata.title.is_none());
        assert!(bms.metadata.artist.is_none());
        assert!(bms.timing.bpm.is_none());
        assert!(bms.audio.wav_files.is_empty());
        assert!(bms.messages.raw.is_empty());
        assert!(bms.fallback_headers.is_empty());
    }

    #[test]
    fn merge_channel_basic_example() {
        use crate::messages::merge_channel;
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
        use crate::messages::merge_channel;
        let lines = vec!["AABBCC".to_owned()];
        assert_eq!(merge_channel(&lines), "AABBCC");
    }

    #[test]
    fn merge_channel_00_preserves_earlier() {
        use crate::messages::merge_channel;
        // Non-00 from line 1 is preserved when line 2 has "00" at that pos.
        let lines = vec!["11".to_owned(), "00".to_owned()];
        assert_eq!(merge_channel(&lines), "11");
    }

    #[test]
    fn merge_channel_later_overwrites_non_00() {
        use crate::messages::merge_channel;
        // Later line's non-00 overwrites earlier non-00.
        let lines = vec!["11".to_owned(), "22".to_owned()];
        assert_eq!(merge_channel(&lines), "22");
    }

    // New: verify headers that were previously silently dropped

    #[test]
    fn oct_fp_stored() {
        let tokens = parse_tokens("#OCT 1");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.gameplay.oct_fp, Some(true));
    }

    #[test]
    fn option_stored() {
        let tokens = parse_tokens("#OPTION -R");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.gameplay.option.as_deref(), Some("-R"));
    }

    #[test]
    fn wavcmd_stored() {
        let tokens = parse_tokens("#WAVCMD some-command");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.audio.wav_cmd.as_deref(), Some("some-command"));
    }

    #[test]
    fn cdda_stored() {
        let tokens = parse_tokens("#CDDA track01.bin");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.audio.cdda.as_deref(), Some("track01.bin"));
    }

    #[test]
    fn midifile_stored() {
        let tokens = parse_tokens("#MIDIFILE song.mid");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.audio.midifile.as_deref(), Some("song.mid"));
    }

    #[test]
    fn ext_chr_stored() {
        let tokens = parse_tokens("#ExtChr extra");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.visual.ext_chr.as_deref(), Some("extra"));
    }

    #[test]
    fn poor_bga_stored() {
        use bms_tokenizer::PoorBgaMode;

        let tokens = parse_tokens("#POORBGA 0");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.visual.poor_bga_mode, Some(PoorBgaMode::Default));
    }

    #[test]
    fn ex_bmp_stored() {
        let tokens = parse_tokens("#EXBMP01 255,0,128,64 overlay.png");
        let bms = Bms::from_flat_tokens(tokens);
        let id: BmpIndex = "01".try_into().unwrap();
        let entry = bms.visual.ex_bmp_defs.get(&id);
        assert!(entry.is_some());
        let params = entry.unwrap();
        assert_eq!(params.a, 255);
        assert_eq!(params.r, 0);
        assert_eq!(params.filename, "overlay.png");
    }

    #[test]
    fn bga_def_stored() {
        let tokens = parse_tokens("#BGA01 02 0 0 100 100 10 20");
        let bms = Bms::from_flat_tokens(tokens);
        let id: BmpIndex = "01".try_into().unwrap();
        assert!(bms.visual.crop_defs.contains_key(&id));
    }

    #[test]
    fn at_bga_stored() {
        let tokens = parse_tokens("#@BGA01 03 5 10 200 150 0 0");
        let bms = Bms::from_flat_tokens(tokens);
        let id: BmpIndex = "01".try_into().unwrap();
        assert!(bms.visual.alt_crop_defs.contains_key(&id));
    }

    #[test]
    fn sw_bga_stored() {
        let tokens = parse_tokens("#SWBGA01 30:60:1:0:255,0,0,128 pattern.bmp");
        let bms = Bms::from_flat_tokens(tokens);
        let id: BmpIndex = "01".try_into().unwrap();
        assert!(bms.visual.sw_bga_defs.contains_key(&id));
    }

    #[test]
    fn argb_stored() {
        let tokens = parse_tokens("#ARGB01 128,255,0,64");
        let bms = Bms::from_flat_tokens(tokens);
        let id: BmpIndex = "01".try_into().unwrap();
        assert!(bms.visual.argb_defs.contains_key(&id));
    }

    #[test]
    fn stp_stored() {
        let tokens = parse_tokens("#STP 001.128 500");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.messages.stp_events.len(), 1);
        let ev = &bms.messages.stp_events[0];
        assert_eq!(ev.position.measure, 1);
        assert_eq!(ev.position.numer, 128);
        assert_eq!(ev.position.denom, 1000);
        assert!((ev.duration_ms - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn video_fps_stored() {
        let tokens = parse_tokens("#VIDEOf/s 30");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.visual.video_fps, Some(30.0));
    }

    #[test]
    fn video_colors_stored() {
        let tokens = parse_tokens("#VIDEOCOLORS 16");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.visual.video_colors, Some(16.0));
    }

    #[test]
    fn video_dly_stored() {
        let tokens = parse_tokens("#VIDEODLY 1.5");
        let bms = Bms::from_flat_tokens(tokens);
        assert_eq!(bms.visual.video_dly, Some(1.5));
    }
}
