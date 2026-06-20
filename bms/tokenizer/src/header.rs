//! BMS header command parsing, categorized by semantic domain.

mod control_flow;
mod display;
mod gameplay;
mod metadata;
mod res_def_audio;
mod res_def_visual;
mod timing;

pub use control_flow::BmsHeaderControlFlow;
pub use display::{BmsHeaderDisplay, DifficultyLevel, ParseDifficultyError, PoorBgaMode};
pub use gameplay::{BmsBaseMode, BmsHeaderGameplay, LnMode, LnType, PlayerMode, Rank};
pub use metadata::BmsHeaderMetadata;
pub use res_def_audio::{BmsHeaderResDefAudio, ExWavParams};
pub use res_def_visual::{
    ArgbParams, AtBgaParams, BgaParams, BmsHeaderResDefVisual, ExBmpParams, SwBgaParams,
};
pub use timing::{BmsHeaderTiming, StpParams};

use std::fmt;

use crate::BmsTokenAttr;
use crate::BmsTokenizeError;
use crate::BmsTryFromError;

/// A header command from a BMS file, categorized by semantic domain.
///
/// The dispatch order follows the variant declaration order below.
/// Variants annotated with `#[bms_fallback]` are excluded from dispatch
/// and instead catch anything that didn't match a concrete variant.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr, derive_more::From)]
pub enum BmsHeader<C> {
    /// Audio resource definitions (`#WAV`, `#EXWAV`, `#WAVCMD`, etc.).
    ResDefAudio(BmsHeaderResDefAudio<C>),
    /// Timing definitions (`#BPM`, `#STOP`, `#SCROLL`, `#SPEED`, etc.).
    Timing(BmsHeaderTiming),
    /// Visual resource definitions (`#BMP`, `#BGA`, `#ARGB`, etc.).
    ResDefVisual(BmsHeaderResDefVisual<C>),
    /// Control-flow commands (`#RANDOM`, `#SWITCH`, `#IF`, etc.).
    ControlFlow(BmsHeaderControlFlow),
    /// Gameplay behaviour (`#PLAYER`, `#RANK`, `#TOTAL`, `#LNTYPE`, etc.).
    Gameplay(BmsHeaderGameplay<C>),
    /// Display and difficulty markers (`#STAGEFILE`, `#DIFFICULTY`, etc.).
    Display(BmsHeaderDisplay<C>),
    /// Song/chart identification (`#TITLE`, `#ARTIST`, `#GENRE`, etc.).
    Metadata(BmsHeaderMetadata<C>),
    /// An unrecognised or engine-specific header command.
    #[bms_fallback]
    Fallback(BmsHeaderFallback<C>),
}

/// Catch-all for unrecognised header commands.
///
/// Captures the raw command name and value so that downstream consumers
/// (parsers, tools) can handle engine-specific extensions that the
/// tokenizer doesn't know about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmsHeaderFallback<C> {
    /// The raw command name as it appears in the file (e.g., `"MYEXT"`).
    pub command: C,
    /// The value after the space separator.
    pub value: C,
}

// From / TryFrom conversions

impl<C> TryFrom<BmsHeader<C>> for BmsHeaderFallback<C> {
    type Error = BmsTryFromError<C>;

    #[inline]
    fn try_from(header: BmsHeader<C>) -> Result<Self, Self::Error> {
        match header {
            BmsHeader::Fallback(f) => Ok(f),
            _ => Err(BmsTryFromError::WrongHeaderType),
        }
    }
}

/// Parse a single header line into a `BmsHeader`.
///
/// `prefixes` controls which leading characters are recognised as header
/// markers (default: `#` and `%`).  Lines not starting with one of the
/// configured prefixes are skipped.
///
/// Returns `Ok(None)` if the line is not a header (empty, comment, channel data).
///
/// # Errors
///
/// Returns `Err(BmsTokenizeError)` if a header command is recognised but its
/// value cannot be parsed into the expected type.
/// Parse with explicit supertraits for C (avoid `E0283` with `BmsStr` blanket impl).
/// In tests, use the non-generic `parse_header_line_default` wrapper.
#[expect(
    clippy::string_slice,
    reason = "BMS header lines are ASCII-only; byte indexing at whitespace boundaries is safe"
)]
pub fn parse_header_line<'a, C: AsRef<str> + fmt::Display + Clone + From<&'a str> + 'a>(
    line: &'a str,
    prefixes: &[char],
) -> Result<Option<BmsHeader<C>>, BmsTokenizeError<C>> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    // Lines starting with `##` are comments.
    if trimmed.starts_with("##") {
        return Ok(None);
    }

    // Determine prefix character from the configured list.
    // trimmed is non-empty (checked above).
    let Some(first) = trimmed.chars().next() else {
        return Ok(None);
    };
    if !prefixes.contains(&first) {
        return Ok(None);
    }
    let prefix = first;
    let rest = &trimmed[prefix.len_utf8()..];

    if rest.is_empty() {
        return Ok(None);
    }

    // Split command name and value at first whitespace.
    let split_pos = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let command = &rest[..split_pos];
    let value = rest[split_pos..].trim();

    if command.is_empty() {
        return Ok(None);
    }

    // If the raw command contains a colon it is a channel message, not a header.
    if command.contains(':') {
        return Ok(None);
    }

    // `%` commands: only `%URL` and `%EMAIL` are valid BMS headers —
    // everything else is an engine-specific extension.  Unknown `%` commands
    // must NOT fall through (doing so would let `%TITLE` masquerade as
    // `#TITLE`).
    if prefix == '%'
        && !command.eq_ignore_ascii_case("URL")
        && !command.eq_ignore_ascii_case("EMAIL")
    {
        return Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
            command: C::from(command),
            value: C::from(value),
        })));
    }

    // Dispatch to sub-enum `try_match_header` functions.
    // `command` carries the input lifetime so that index-slice errors
    // can store the correct portion of the command.
    if let Some(header) = BmsHeader::try_match_header(command, value)? {
        return Ok(Some(header));
    }

    // Nothing matched → Fallback.
    Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
        command: C::from(command),
        value: C::from(value),
    })))
}

/// Convenience wrapper for tests — calls [`parse_header_line`] with the
/// default `#` and `%` prefixes.
#[cfg(test)]
pub fn parse_header_line_default(
    line: &str,
) -> Result<Option<BmsHeader<&str>>, BmsTokenizeError<&str>> {
    parse_header_line::<&str>(line, &['#', '%'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::display::PoorBgaMode;
    use crate::header::gameplay::Rank;
    use crate::header::timing::StpParams;
    use crate::index::{
        BmpTag, BmsIndex, BpmTag, ChangeOptionTag, ExRankTag, LnObjTag, ScrollTag, SeekTag,
        SpeedTag, StopTag, TextTag, WavTag,
    };

    #[test]
    fn parse_title() {
        let result = parse_header_line_default("#TITLE My Song")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("My Song"))
        );
    }

    #[test]
    fn parse_title_empty() {
        let result = parse_header_line_default("#TITLE").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Metadata(BmsHeaderMetadata::Title("")));
    }

    #[test]
    fn parse_subtitle() {
        let result = parse_header_line_default("#SUBTITLE (short ver.)")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Subtitle("(short ver.)"))
        );
    }

    #[test]
    fn parse_artist() {
        let result = parse_header_line_default("#ARTIST composer")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Artist("composer"))
        );
    }

    #[test]
    fn parse_subartist() {
        let result = parse_header_line_default("#SUBARTIST co-writer")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::SubArtist("co-writer"))
        );
    }

    #[test]
    fn parse_genre() {
        let result = parse_header_line_default("#GENRE Piano").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Genre("Piano"))
        );
    }

    #[test]
    fn parse_maker() {
        let result = parse_header_line_default("#MAKER chart-creator")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Maker("chart-creator"))
        );
    }

    #[test]
    fn parse_comment() {
        let result = parse_header_line_default("#COMMENT hello")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Comment("hello"))
        );
    }

    #[test]
    fn parse_text() {
        let result = parse_header_line_default("#TEXT01 in-game text")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Text {
                id: BmsIndex::<TextTag>::try_from("01").unwrap(),
                value: "in-game text"
            })
        );
    }

    #[test]
    fn parse_text_with_quotes() {
        let result = parse_header_line_default("#TEXT00 \"MISS!!\"")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Text {
                id: BmsIndex::<TextTag>::try_from("00").unwrap(),
                value: "\"MISS!!\""
            })
        );
    }

    #[test]
    fn parse_song_as_text() {
        let result = parse_header_line_default("#SONG01 some text")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Text {
                id: BmsIndex::<TextTag>::try_from("01").unwrap(),
                value: "some text"
            })
        );
    }

    #[test]
    fn parse_charset() {
        let result = parse_header_line_default("#CHARSET UTF-8")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Charset("UTF-8"))
        );
    }

    #[test]
    fn parse_url() {
        let result = parse_header_line_default("%URL https://example.com")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Url("https://example.com"))
        );
    }

    #[test]
    fn parse_email() {
        let result = parse_header_line_default("%EMAIL user@example.com")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Email("user@example.com"))
        );
    }

    #[test]
    fn parse_player() {
        let result = parse_header_line_default("#PLAYER 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::Player(PlayerMode::Single))
        );
    }

    #[test]
    fn parse_rank() {
        let result = parse_header_line_default("#RANK 2").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::Rank(Rank::Normal))
        );
    }

    #[test]
    fn parse_defexrank() {
        let result = parse_header_line_default("#DEFEXRANK 3").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::DefExRank(3.0))
        );
    }

    #[test]
    fn parse_total() {
        let result = parse_header_line_default("#TOTAL 300").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Total(300.0)));
    }

    #[test]
    fn parse_volwav() {
        let result = parse_header_line_default("#VOLWAV 100").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::VolWav(100.0))
        );
    }

    #[test]
    fn parse_lntype() {
        let result = parse_header_line_default("#LNTYPE 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::LnType(LnType::Type1))
        );
    }

    #[test]
    fn parse_lnobj() {
        let result = parse_header_line_default("#LNOBJ 01").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::LnObj(
                BmsIndex::<LnObjTag>::try_from("01").unwrap()
            ))
        );
    }

    #[test]
    fn parse_lnmode() {
        let result = parse_header_line_default("#LNMODE 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::LnMode(LnMode::Ln))
        );
    }

    #[test]
    fn parse_oct() {
        let result = parse_header_line_default("#OCT 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
    }

    #[test]
    fn parse_fp() {
        let result = parse_header_line_default("#FP 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
    }

    #[test]
    fn parse_octfp() {
        let result = parse_header_line_default("#OCT/FP 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
    }

    #[test]
    fn parse_option() {
        let result = parse_header_line_default("#OPTION -R").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Option("-R")));
    }

    #[test]
    fn parse_changeoption() {
        let result = parse_header_line_default("#CHANGEOPTION01 774:HIDDEN_STEALTH")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::ChangeOption {
                id: BmsIndex::<ChangeOptionTag>::try_from("01").unwrap(),
                value: "774:HIDDEN_STEALTH"
            })
        );
    }

    #[test]
    fn parse_stagefile() {
        let result = parse_header_line_default("#STAGEFILE stage.png")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::StageFile("stage.png"))
        );
    }

    #[test]
    fn parse_banner() {
        let result = parse_header_line_default("#BANNER banner.bmp")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Banner("banner.bmp"))
        );
    }

    #[test]
    fn parse_backbmp() {
        let result = parse_header_line_default("#BACKBMP bg.png")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::BackBmp("bg.png"))
        );
    }

    #[test]
    fn parse_charfile() {
        let result = parse_header_line_default("#CHARFILE char.bmp")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::CharFile("char.bmp"))
        );
    }

    #[test]
    fn parse_playlevel() {
        let result = parse_header_line_default("#PLAYLEVEL 12").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::PlayLevel(12.0))
        );
    }

    #[test]
    fn parse_difficulty() {
        let result = parse_header_line_default("#DIFFICULTY 3").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Difficulty(
                "3".parse::<DifficultyLevel>().unwrap()
            ))
        );
    }

    #[test]
    fn parse_preview() {
        let result = parse_header_line_default("#PREVIEW preview.ogg")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Preview("preview.ogg"))
        );
    }

    #[test]
    fn parse_bpm_global() {
        let result = parse_header_line_default("#BPM 180").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm(180.0)));
    }

    #[test]
    fn parse_bpm_global_float() {
        let result = parse_header_line_default("#BPM 180.0").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm(180.0)));
    }

    #[test]
    fn parse_basebpm() {
        let result = parse_header_line_default("#BASEBPM 180").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::BaseBpm(180.0)));
    }

    #[test]
    fn parse_wavcmd() {
        let result = parse_header_line_default("#WAVCMD some-command")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd("some-command"))
        );
    }

    #[test]
    fn parse_cdda() {
        let result = parse_header_line_default("#CDDA track01.bin")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Cdda("track01.bin"))
        );
    }

    #[test]
    fn parse_midifile() {
        let result = parse_header_line_default("#MIDIFILE song.mid")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Midifile("song.mid"))
        );
    }

    #[test]
    fn parse_path_wav() {
        let result = parse_header_line_default("#PATH_WAV ./sounds/")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::PathWav("./sounds/"))
        );
    }

    #[test]
    fn parse_poorbga() {
        let result = parse_header_line_default("#POORBGA 0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Default))
        );
    }

    #[test]
    fn parse_poorbga_overlay() {
        let result = parse_header_line_default("#POORBGA 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Overlay))
        );
    }

    #[test]
    fn parse_poorbga_hidden() {
        let result = parse_header_line_default("#POORBGA 2").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Hidden))
        );
    }

    #[test]
    fn parse_poorbga_invalid_fallback() {
        let result = parse_header_line_default("#POORBGA 3").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_videofile() {
        let result = parse_header_line_default("#VIDEOFILE bg.avi")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::VideoFile("bg.avi"))
        );
    }

    #[test]
    fn parse_movie() {
        let result = parse_header_line_default("#MOVIE intro.mpg")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Movie("intro.mpg"))
        );
    }

    #[test]
    fn parse_extchr() {
        let result = parse_header_line_default("#ExtChr extra").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExtChr("extra"))
        );
    }

    #[test]
    fn parse_random() {
        let result = parse_header_line_default("#RANDOM 10").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(10))
        );
    }

    #[test]
    fn parse_setrandom() {
        let result = parse_header_line_default("#SETRANDOM 5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::SetRandom(5))
        );
    }

    #[test]
    fn parse_if() {
        let result = parse_header_line_default("#IF 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::If(1)));
    }

    #[test]
    fn parse_elseif() {
        let result = parse_header_line_default("#ELSEIF 0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::ElseIf(0))
        );
    }

    #[test]
    fn parse_else() {
        let result = parse_header_line_default("#ELSE").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Else));
    }

    #[test]
    fn parse_endif() {
        let result = parse_header_line_default("#ENDIF").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::EndIf));
    }

    #[test]
    fn parse_endrandom() {
        let result = parse_header_line_default("#ENDRANDOM").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndRandom)
        );
    }

    #[test]
    fn parse_switch() {
        let result = parse_header_line_default("#SWITCH 3").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Switch(3))
        );
    }

    #[test]
    fn parse_setswitch() {
        let result = parse_header_line_default("#SETSWITCH 2").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::SetSwitch(2))
        );
    }

    #[test]
    fn parse_case() {
        let result = parse_header_line_default("#CASE 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Case(1))
        );
    }

    #[test]
    fn parse_skip() {
        let result = parse_header_line_default("#SKIP 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Skip(1))
        );
    }

    #[test]
    fn parse_def() {
        let result = parse_header_line_default("#DEF").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Def));
    }

    #[test]
    fn parse_endsw() {
        let result = parse_header_line_default("#ENDSW").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
        );
    }

    #[test]
    fn parse_endswitch_alias() {
        let result = parse_header_line_default("#ENDSWITCH").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
        );
    }

    #[test]
    fn parse_wav_indexed() {
        let result = parse_header_line_default("#WAV01 kick.wav")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                id: BmsIndex::<WavTag>::try_from("01").unwrap(),
                filename: "kick.wav"
            })
        );
    }

    #[test]
    fn parse_wav_36ary_index() {
        let result = parse_header_line_default("#WAV2A snare.wav")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                id: BmsIndex::<WavTag>::try_from("2A").unwrap(),
                filename: "snare.wav"
            })
        );
    }

    #[test]
    fn parse_exwav_indexed() {
        let result = parse_header_line_default("#EXWAV01 extra.ogg")
            .unwrap()
            .unwrap();
        if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { id, params }) = result {
            assert_eq!(id.as_str(), "01");
            assert_eq!(params.filename, "extra.ogg");
            assert!(params.flags.is_empty());
        } else {
            panic!("expected ExWav variant");
        }
    }

    #[test]
    fn parse_exwav_with_flags() {
        let result = parse_header_line_default("#EXWAV01 pvf -100 50 440 sound.wav")
            .unwrap()
            .unwrap();
        if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { id, params }) = result {
            assert_eq!(id.as_str(), "01");
            assert_eq!(params.flags, "pvf");
            assert_eq!(params.values, vec![-100.0, 50.0, 440.0]);
            assert_eq!(params.filename, "sound.wav");
        } else {
            panic!("expected ExWav variant");
        }
    }

    #[test]
    fn parse_bmp_indexed() {
        let result = parse_header_line_default("#BMP01 bg.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bmp {
                id: BmsIndex::<BmpTag>::try_from("01").unwrap(),
                filename: "bg.bmp"
            })
        );
    }

    #[test]
    fn parse_exbmp_indexed() {
        let result = parse_header_line_default("#EXBMP01 255,0,128,64 overlay.png")
            .unwrap()
            .unwrap();
        if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExBmp { id, params }) = result {
            assert_eq!(id.as_str(), "01");
            assert_eq!(params.a, 255);
            assert_eq!(params.r, 0);
            assert_eq!(params.g, 128);
            assert_eq!(params.b, 64);
            assert_eq!(params.filename, "overlay.png");
        } else {
            panic!("expected ExBmp variant");
        }
    }

    #[test]
    fn parse_bga_indexed() {
        let result = parse_header_line_default("#BGA01 02 0 0 100 100 10 20")
            .unwrap()
            .unwrap();
        if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga { id, params }) = result {
            assert_eq!(id.as_str(), "01");
            assert_eq!(params.bmp_index, 2);
            assert_eq!(params.x1, 0);
            assert_eq!(params.y1, 0);
            assert_eq!(params.x2, 100);
            assert_eq!(params.y2, 100);
            assert_eq!(params.dx, 10);
            assert_eq!(params.dy, 20);
        } else {
            panic!("expected Bga variant");
        }
    }

    #[test]
    fn parse_at_bga_indexed() {
        let result = parse_header_line_default("#@BGA01 03 5 10 200 150 0 0")
            .unwrap()
            .unwrap();
        if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::AtBga { id, params }) = result {
            assert_eq!(id.as_str(), "01");
            assert_eq!(params.bmp_index, 3);
            assert_eq!(params.sx, 5);
            assert_eq!(params.sy, 10);
            assert_eq!(params.w, 200);
            assert_eq!(params.h, 150);
        } else {
            panic!("expected AtBga variant");
        }
    }

    #[test]
    fn parse_swbga_indexed() {
        let result = parse_header_line_default("#SWBGA01 30:60:1:0:255,0,0,128 pattern.bmp")
            .unwrap()
            .unwrap();
        if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::SwBga { id, params }) = result {
            assert_eq!(id.as_str(), "01");
            assert_eq!(params.fr, 30);
            assert_eq!(params.time, 60);
            assert_eq!(params.line, 1);
            assert!(!params.r#loop);
            assert_eq!(params.a, 255);
            assert_eq!(params.r, 0);
            assert_eq!(params.g, 0);
            assert_eq!(params.b, 128);
            assert_eq!(params.pattern, "pattern.bmp");
        } else {
            panic!("expected SwBga variant");
        }
    }

    #[test]
    fn parse_argb_indexed() {
        let result = parse_header_line_default("#ARGB01 128,255,0,64")
            .unwrap()
            .unwrap();
        if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Argb { id, params }) = result {
            assert_eq!(id.as_str(), "01");
            assert_eq!(params.a, 128);
            assert_eq!(params.r, 255);
            assert_eq!(params.g, 0);
            assert_eq!(params.b, 64);
        } else {
            panic!("expected Argb variant");
        }
    }

    #[test]
    fn parse_bga_invalid_fallback() {
        let result = parse_header_line_default("#BGA01 bad").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_argb_invalid_fallback() {
        let result = parse_header_line_default("#ARGB01 bad").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_seek_indexed() {
        let result = parse_header_line_default("#SEEK01 1.5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Seek {
                id: BmsIndex::<SeekTag>::try_from("01").unwrap(),
                value: 1.5
            })
        );
    }

    #[test]
    fn parse_bpm_def_indexed() {
        let result = parse_header_line_default("#BPM01 180.0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::BpmDef {
                id: BmsIndex::<BpmTag>::try_from("01").unwrap(),
                value: 180.0
            })
        );
    }

    #[test]
    fn parse_stop_indexed() {
        let result = parse_header_line_default("#STOP01 192").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::StopDef {
                id: BmsIndex::<StopTag>::try_from("01").unwrap(),
                value: 192.0
            })
        );
    }

    #[test]
    fn parse_scroll_indexed() {
        let result = parse_header_line_default("#SCROLL01 1.5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::ScrollDef {
                id: BmsIndex::<ScrollTag>::try_from("01").unwrap(),
                value: 1.5
            })
        );
    }

    #[test]
    fn parse_speed_indexed() {
        let result = parse_header_line_default("#SPEED01 2.0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::SpeedDef {
                id: BmsIndex::<SpeedTag>::try_from("01").unwrap(),
                value: 2.0
            })
        );
    }

    #[test]
    fn parse_exrank_indexed() {
        let result = parse_header_line_default("#EXRANK01 5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::ExRank {
                id: BmsIndex::<ExRankTag>::try_from("01").unwrap(),
                value: 5.0
            })
        );
    }

    #[test]
    fn wavcmd_not_confused_as_wav_indexed() {
        let result = parse_header_line_default("#WAVCMD test").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd("test"))
        );
    }

    #[test]
    fn parse_unknown_header() {
        let result = parse_header_line_default("#MYEXT abc123").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Fallback(BmsHeaderFallback {
                command: "MYEXT",
                value: "abc123",
            })
        );
    }

    #[test]
    fn parse_unknown_percent_header() {
        let result = parse_header_line_default("%CUSTOM x").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Fallback(BmsHeaderFallback {
                command: "CUSTOM",
                value: "x",
            })
        );
    }

    #[test]
    fn empty_line_returns_none() {
        assert_eq!(parse_header_line_default("").unwrap(), None);
    }

    #[test]
    fn whitespace_only_returns_none() {
        assert_eq!(parse_header_line_default("   ").unwrap(), None);
    }

    #[test]
    fn comment_line_returns_none() {
        assert_eq!(parse_header_line_default("// comment").unwrap(), None);
    }

    #[test]
    fn just_hash_returns_none() {
        assert_eq!(parse_header_line_default("#").unwrap(), None);
    }

    #[test]
    fn just_percent_returns_none() {
        assert_eq!(parse_header_line_default("%").unwrap(), None);
    }

    #[test]
    fn case_insensitive_title() {
        let result = parse_header_line_default("#title lowercase")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("lowercase"))
        );
    }

    #[test]
    fn case_insensitive_wav() {
        let result = parse_header_line_default("#wav01 sound.wav")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                id: BmsIndex::<WavTag>::try_from("01").unwrap(),
                filename: "sound.wav"
            })
        );
    }

    #[test]
    fn value_with_multiple_spaces() {
        let result = parse_header_line_default("#TITLE   My   Song")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("My   Song"))
        );
    }

    #[test]
    fn non_header_line() {
        assert_eq!(parse_header_line_default("just some text").unwrap(), None);
    }

    #[test]
    fn channel_line_not_confused() {
        assert_eq!(parse_header_line_default("#00111:1122").unwrap(), None);
    }

    #[test]
    fn hash_comment_line() {
        assert_eq!(
            parse_header_line_default("## just a comment").unwrap(),
            None
        );
    }

    #[test]
    fn invalid_player_returns_error() {
        assert!(parse_header_line_default("#PLAYER xyz").is_err());
    }

    #[test]
    fn invalid_difficulty_returns_error() {
        assert!(parse_header_line_default("#DIFFICULTY 0").is_err());
    }

    #[test]
    fn invalid_rank_returns_error() {
        assert!(parse_header_line_default("#RANK abc").is_err());
    }

    #[test]
    fn invalid_wav_index_returns_error() {
        assert!(parse_header_line_default("#WAV!! file.wav").is_err());
    }

    #[test]
    fn invalid_bpm_returns_error() {
        assert!(parse_header_line_default("#BPM notanumber").is_err());
    }

    #[test]
    fn parse_exbpm_indexed() {
        let result = parse_header_line_default("#EXBPM01 180.0")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::ExBpm {
                id: BmsIndex::<BpmTag>::try_from("01").unwrap(),
                value: 180.0
            })
        );
    }

    #[test]
    fn parse_stp_with_position() {
        let result = parse_header_line_default("#STP 001.128 500")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::Stp {
                params: StpParams {
                    measure: 1,
                    position: 128,
                    duration_ms: 500.0
                }
            })
        );
    }

    #[test]
    fn parse_stp_without_position() {
        let result = parse_header_line_default("#STP 001 500.5")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::Stp {
                params: StpParams {
                    measure: 1,
                    position: 0,
                    duration_ms: 500.5
                }
            })
        );
    }

    #[test]
    fn parse_stp_invalid_fallback() {
        let result = parse_header_line_default("#STP invalid").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_genre_alias() {
        let result = parse_header_line_default("#GENLE Pop").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Metadata(BmsHeaderMetadata::Genre("Pop")));
    }

    #[test]
    fn parse_random_alias() {
        let result = parse_header_line_default("#RONDAM 5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(5))
        );
    }

    #[test]
    fn parse_base_16() {
        let result = parse_header_line_default("#BASE 16").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::Base(BmsBaseMode::Base16))
        );
    }

    #[test]
    fn parse_base_36() {
        let result = parse_header_line_default("#BASE 36").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::Base(BmsBaseMode::Base36))
        );
    }

    #[test]
    fn parse_base_62() {
        let result = parse_header_line_default("#BASE 62").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::Base(BmsBaseMode::Base62))
        );
    }

    #[test]
    fn parse_base_unknown_fallback() {
        let result = parse_header_line_default("#BASE 99").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }
}
