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
pub use gameplay::{BmsHeaderGameplay, LnMode, LnType, PlayerMode};
pub use metadata::BmsHeaderMetadata;
pub use res_def_audio::{BmsHeaderResDefAudio, ExWavParams};
pub use res_def_visual::{
    ArgbParams, AtBgaParams, BgaParams, BmsHeaderResDefVisual, ExBmpParams, SwBgaParams,
};
pub use timing::BmsHeaderTiming;

use crate::BmsTokenAttr;
use crate::BmsTokenizeError;

/// A header command from a BMS file, categorized by semantic domain.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeader<'a> {
    /// Song/chart identification (title, artist, genre, ...).
    Metadata(BmsHeaderMetadata<'a>),
    /// Gameplay behaviour (player count, rank, total, LN settings, ...).
    Gameplay(BmsHeaderGameplay<'a>),
    /// Display and difficulty markers.
    Display(BmsHeaderDisplay<'a>),
    /// Timing definitions (BPM, stops, scroll, speed).
    Timing(BmsHeaderTiming),
    /// Audio resource definitions (WAV files, CDDA, ...).
    ResDefAudio(BmsHeaderResDefAudio<'a>),
    /// Visual resource definitions (BMP, BGA, video, ...).
    ResDefVisual(BmsHeaderResDefVisual<'a>),
    /// Control-flow commands (random, if, switch, ...).
    ControlFlow(BmsHeaderControlFlow),
    /// An unrecognised or engine-specific header command.
    #[bms_fallback]
    Fallback(BmsHeaderFallback<'a>),
}

/// Catch-all for unrecognised header commands.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsHeaderFallback<'a> {
    /// The raw command name as it appears in the file (e.g., `"MYEXT"`).
    pub command: &'a str,
    /// The value after the space separator.
    pub value: &'a str,
}

/// Parse a single header line into a `BmsHeader`.
///
/// Returns `Ok(None)` if the line is not a header (empty, comment, channel data).
///
/// # Errors
///
/// Returns `Err(BmsTokenizeError)` if a header command is recognised but its
/// value cannot be parsed into the expected type.
pub(crate) fn parse_header_line(line: &str) -> Result<Option<BmsHeader<'_>>, BmsTokenizeError<'_>> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    // Lines starting with `##` are comments.
    if trimmed.starts_with("##") {
        return Ok(None);
    }

    // Determine prefix character.
    let (prefix, rest) = if let Some(r) = trimmed.strip_prefix('#') {
        ('#', r)
    } else if let Some(r) = trimmed.strip_prefix('%') {
        ('%', r)
    } else {
        return Ok(None);
    };

    if rest.is_empty() {
        return Ok(None);
    }

    // Split command name and value at first whitespace.
    let split_pos = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let command_raw = &rest[..split_pos];
    let value = rest[split_pos..].trim();

    if command_raw.is_empty() {
        return Ok(None);
    }

    // If the raw command contains a colon it is a channel message, not a header.
    if command_raw.contains(':') {
        return Ok(None);
    }

    let command_upper = command_raw.to_uppercase();

    // `%` commands: only `%URL` and `%EMAIL` are valid BMS headers —
    // everything else is an engine-specific extension.  The dispatch
    // table below would catch `%URL`/`%EMAIL` via their command name,
    // but unknown `%` commands must NOT fall through to the `#` dispatch
    // (doing so would let `%TITLE` masquerade as `#TITLE`).
    if prefix == '%' && command_upper != "URL" && command_upper != "EMAIL" {
        return Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
            command: command_raw,
            value,
        })));
    }

    // Single dispatch via sub-enum `__bms_dispatch` functions.
    // `command_raw` carries the input lifetime so that index-slice errors
    // can store the correct portion of the command.
    if let Some(header) = BmsHeader::try_match_header(&command_upper, command_raw, value)? {
        return Ok(Some(header));
    }

    // #STP: non-standard format "xxx[.yyy] zzzz" — hand-parsed.
    if command_upper == "STP" {
        if let Some(stp) = parse_stp(value) {
            return Ok(Some(BmsHeader::Timing(BmsHeaderTiming::Stp {
                measure: stp.measure,
                position: stp.position,
                duration_ms: stp.duration_ms,
            })));
        }
        return Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
            command: command_raw,
            value,
        })));
    }

    // Nothing matched → Fallback.
    Ok(Some(BmsHeader::Fallback(BmsHeaderFallback {
        command: command_raw,
        value,
    })))
}

/// Intermediate parsed result for `#STP` value.
struct StpParts {
    /// Measure number.
    measure: u16,
    /// Position within the measure (0–255).
    position: u16,
    /// Duration in milliseconds.
    duration_ms: f64,
}

/// Parse `#STP` value format: `xxx[.yyy] zzzz`
///
/// - `xxx` = measure (decimal, 1–3 digits)
/// - `.yyy` = position within measure (optional, 0–255)
/// - `zzzz` = stop duration in milliseconds (decimal)
fn parse_stp(value: &str) -> Option<StpParts> {
    let (pos_part, dur_part) = value.split_once(' ')?;
    let dur_ms: f64 = dur_part.trim().parse().ok()?;

    let (measure_str, position_str) = if let Some(dot) = pos_part.find('.') {
        let m = &pos_part[..dot];
        let p = &pos_part[dot + 1..];
        (m, p)
    } else {
        (pos_part, "0")
    };

    let measure: u16 = measure_str.parse().ok()?;
    let position: u16 = position_str.parse().ok()?;
    if position > 255 {
        return None;
    }

    Some(StpParts {
        measure,
        position,
        duration_ms: dur_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::display::PoorBgaMode;
    use crate::id::{
        BmpTag, BmsChannelId, BpmTag, ExRankTag, LnObjTag, ScrollTag, SeekTag, SpeedTag, StopTag,
        WavTag,
    };

    #[test]
    fn parse_title() {
        let result = parse_header_line("#TITLE My Song").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("My Song"))
        );
    }

    #[test]
    fn parse_title_empty() {
        let result = parse_header_line("#TITLE").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Metadata(BmsHeaderMetadata::Title("")));
    }

    #[test]
    fn parse_subtitle() {
        let result = parse_header_line("#SUBTITLE (short ver.)")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Subtitle("(short ver.)"))
        );
    }

    #[test]
    fn parse_artist() {
        let result = parse_header_line("#ARTIST composer").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Artist("composer"))
        );
    }

    #[test]
    fn parse_subartist() {
        let result = parse_header_line("#SUBARTIST co-writer").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::SubArtist("co-writer"))
        );
    }

    #[test]
    fn parse_genre() {
        let result = parse_header_line("#GENRE Piano").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Genre("Piano"))
        );
    }

    #[test]
    fn parse_maker() {
        let result = parse_header_line("#MAKER chart-creator").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Maker("chart-creator"))
        );
    }

    #[test]
    fn parse_comment() {
        let result = parse_header_line("#COMMENT hello").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Comment("hello"))
        );
    }

    #[test]
    fn parse_text() {
        let result = parse_header_line("#TEXT in-game text").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Text("in-game text"))
        );
    }

    #[test]
    fn parse_song_as_text() {
        let result = parse_header_line("#SONG some text").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Text("some text"))
        );
    }

    #[test]
    fn parse_charset() {
        let result = parse_header_line("#CHARSET UTF-8").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Charset("UTF-8"))
        );
    }

    #[test]
    fn parse_url() {
        let result = parse_header_line("%URL https://example.com")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Url("https://example.com"))
        );
    }

    #[test]
    fn parse_email() {
        let result = parse_header_line("%EMAIL user@example.com")
            .unwrap()
            .unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Email("user@example.com"))
        );
    }

    #[test]
    fn parse_player() {
        let result = parse_header_line("#PLAYER 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::Player(PlayerMode::Single))
        );
    }

    #[test]
    fn parse_rank() {
        let result = parse_header_line("#RANK 2").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Rank(2u8)));
    }

    #[test]
    fn parse_defexrank() {
        let result = parse_header_line("#DEFEXRANK 3").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::DefExRank(3.0))
        );
    }

    #[test]
    fn parse_total() {
        let result = parse_header_line("#TOTAL 300").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Total(300.0)));
    }

    #[test]
    fn parse_volwav() {
        let result = parse_header_line("#VOLWAV 100").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::VolWav(100.0))
        );
    }

    #[test]
    fn parse_lntype() {
        let result = parse_header_line("#LNTYPE 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::LnType(LnType::Type1))
        );
    }

    #[test]
    fn parse_lnobj() {
        let result = parse_header_line("#LNOBJ 01").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::LnObj(
                BmsChannelId::<LnObjTag>::try_from("01").unwrap()
            ))
        );
    }

    #[test]
    fn parse_lnmode() {
        let result = parse_header_line("#LNMODE 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::LnMode(LnMode::Ln))
        );
    }

    #[test]
    fn parse_oct() {
        let result = parse_header_line("#OCT 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
    }

    #[test]
    fn parse_fp() {
        let result = parse_header_line("#FP 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
    }

    #[test]
    fn parse_octfp() {
        let result = parse_header_line("#OCT/FP 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
    }

    #[test]
    fn parse_option() {
        let result = parse_header_line("#OPTION -R").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Option("-R")));
    }

    #[test]
    fn parse_changeoption() {
        let result = parse_header_line("#CHANGEOPTION RANDOM").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::ChangeOption("RANDOM"))
        );
    }

    #[test]
    fn parse_stagefile() {
        let result = parse_header_line("#STAGEFILE stage.png").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::StageFile("stage.png"))
        );
    }

    #[test]
    fn parse_banner() {
        let result = parse_header_line("#BANNER banner.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Banner("banner.bmp"))
        );
    }

    #[test]
    fn parse_backbmp() {
        let result = parse_header_line("#BACKBMP bg.png").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::BackBmp("bg.png"))
        );
    }

    #[test]
    fn parse_charfile() {
        let result = parse_header_line("#CHARFILE char.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::CharFile("char.bmp"))
        );
    }

    #[test]
    fn parse_playlevel() {
        let result = parse_header_line("#PLAYLEVEL 12").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::PlayLevel(12.0))
        );
    }

    #[test]
    fn parse_difficulty() {
        let result = parse_header_line("#DIFFICULTY 3").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Difficulty(
                "3".parse::<DifficultyLevel>().unwrap()
            ))
        );
    }

    #[test]
    fn parse_preview() {
        let result = parse_header_line("#PREVIEW preview.ogg").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Preview("preview.ogg"))
        );
    }

    #[test]
    fn parse_bpm_global() {
        let result = parse_header_line("#BPM 180").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm(180.0)));
    }

    #[test]
    fn parse_bpm_global_float() {
        let result = parse_header_line("#BPM 180.0").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm(180.0)));
    }

    #[test]
    fn parse_basebpm() {
        let result = parse_header_line("#BASEBPM 180").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::BaseBpm(180.0)));
    }

    #[test]
    fn parse_wavcmd() {
        let result = parse_header_line("#WAVCMD some-command").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd("some-command"))
        );
    }

    #[test]
    fn parse_cdda() {
        let result = parse_header_line("#CDDA track01.bin").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Cdda("track01.bin"))
        );
    }

    #[test]
    fn parse_midifile() {
        let result = parse_header_line("#MIDIFILE song.mid").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Midifile("song.mid"))
        );
    }

    #[test]
    fn parse_path_wav() {
        let result = parse_header_line("#PATH_WAV ./sounds/").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::PathWav("./sounds/"))
        );
    }

    #[test]
    fn parse_poorbga() {
        let result = parse_header_line("#POORBGA 0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Default))
        );
    }

    #[test]
    fn parse_poorbga_overlay() {
        let result = parse_header_line("#POORBGA 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Overlay))
        );
    }

    #[test]
    fn parse_poorbga_hidden() {
        let result = parse_header_line("#POORBGA 2").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Hidden))
        );
    }

    #[test]
    fn parse_poorbga_invalid_fallback() {
        let result = parse_header_line("#POORBGA 3").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_videofile() {
        let result = parse_header_line("#VIDEOFILE bg.avi").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::VideoFile("bg.avi"))
        );
    }

    #[test]
    fn parse_movie() {
        let result = parse_header_line("#MOVIE intro.mpg").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Movie("intro.mpg"))
        );
    }

    #[test]
    fn parse_extchr() {
        let result = parse_header_line("#ExtChr extra").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExtChr("extra"))
        );
    }

    #[test]
    fn parse_random() {
        let result = parse_header_line("#RANDOM 10").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(10))
        );
    }

    #[test]
    fn parse_setrandom() {
        let result = parse_header_line("#SETRANDOM 5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::SetRandom(5))
        );
    }

    #[test]
    fn parse_if() {
        let result = parse_header_line("#IF 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::If(1)));
    }

    #[test]
    fn parse_elseif() {
        let result = parse_header_line("#ELSEIF 0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::ElseIf(0))
        );
    }

    #[test]
    fn parse_else() {
        let result = parse_header_line("#ELSE").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Else));
    }

    #[test]
    fn parse_endif() {
        let result = parse_header_line("#ENDIF").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::EndIf));
    }

    #[test]
    fn parse_endrandom() {
        let result = parse_header_line("#ENDRANDOM").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndRandom)
        );
    }

    #[test]
    fn parse_switch() {
        let result = parse_header_line("#SWITCH 3").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Switch(3))
        );
    }

    #[test]
    fn parse_setswitch() {
        let result = parse_header_line("#SETSWITCH 2").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::SetSwitch(2))
        );
    }

    #[test]
    fn parse_case() {
        let result = parse_header_line("#CASE 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Case(1))
        );
    }

    #[test]
    fn parse_skip() {
        let result = parse_header_line("#SKIP 1").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Skip(1))
        );
    }

    #[test]
    fn parse_def() {
        let result = parse_header_line("#DEF").unwrap().unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Def));
    }

    #[test]
    fn parse_endsw() {
        let result = parse_header_line("#ENDSW").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
        );
    }

    #[test]
    fn parse_endswitch_alias() {
        let result = parse_header_line("#ENDSWITCH").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
        );
    }

    #[test]
    fn parse_wav_indexed() {
        let result = parse_header_line("#WAV01 kick.wav").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                id: BmsChannelId::<WavTag>::try_from("01").unwrap(),
                filename: "kick.wav"
            })
        );
    }

    #[test]
    fn parse_wav_36ary_index() {
        let result = parse_header_line("#WAV2A snare.wav").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                id: BmsChannelId::<WavTag>::try_from("2A").unwrap(),
                filename: "snare.wav"
            })
        );
    }

    #[test]
    fn parse_exwav_indexed() {
        let result = parse_header_line("#EXWAV01 extra.ogg").unwrap().unwrap();
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
        let result = parse_header_line("#EXWAV01 pvf -100 50 440 sound.wav")
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
        let result = parse_header_line("#BMP01 bg.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bmp {
                id: BmsChannelId::<BmpTag>::try_from("01").unwrap(),
                filename: "bg.bmp"
            })
        );
    }

    #[test]
    fn parse_exbmp_indexed() {
        let result = parse_header_line("#EXBMP01 255,0,128,64 overlay.png")
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
        let result = parse_header_line("#BGA01 02 0 0 100 100 10 20")
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
        let result = parse_header_line("#@BGA01 03 5 10 200 150 0 0")
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
        let result = parse_header_line("#SWBGA01 30:60:1:0:255,0,0,128 pattern.bmp")
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
        let result = parse_header_line("#ARGB01 128,255,0,64").unwrap().unwrap();
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
        let result = parse_header_line("#BGA01 bad").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_argb_invalid_fallback() {
        let result = parse_header_line("#ARGB01 bad").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_seek_indexed() {
        let result = parse_header_line("#SEEK01 1.5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Seek {
                id: BmsChannelId::<SeekTag>::try_from("01").unwrap(),
                value: 1.5
            })
        );
    }

    #[test]
    fn parse_bpm_def_indexed() {
        let result = parse_header_line("#BPM01 180.0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::BpmDef {
                id: BmsChannelId::<BpmTag>::try_from("01").unwrap(),
                value: 180.0
            })
        );
    }

    #[test]
    fn parse_stop_indexed() {
        let result = parse_header_line("#STOP01 192").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::StopDef {
                id: BmsChannelId::<StopTag>::try_from("01").unwrap(),
                value: 192.0
            })
        );
    }

    #[test]
    fn parse_scroll_indexed() {
        let result = parse_header_line("#SCROLL01 1.5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::ScrollDef {
                id: BmsChannelId::<ScrollTag>::try_from("01").unwrap(),
                value: 1.5
            })
        );
    }

    #[test]
    fn parse_speed_indexed() {
        let result = parse_header_line("#SPEED01 2.0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::SpeedDef {
                id: BmsChannelId::<SpeedTag>::try_from("01").unwrap(),
                value: 2.0
            })
        );
    }

    #[test]
    fn parse_exrank_indexed() {
        let result = parse_header_line("#EXRANK01 5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::ExRank {
                id: BmsChannelId::<ExRankTag>::try_from("01").unwrap(),
                value: 5.0
            })
        );
    }

    #[test]
    fn wavcmd_not_confused_as_wav_indexed() {
        let result = parse_header_line("#WAVCMD test").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd("test"))
        );
    }

    #[test]
    fn parse_unknown_header() {
        let result = parse_header_line("#MYEXT abc123").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Fallback(BmsHeaderFallback {
                command: "MYEXT",
                value: "abc123"
            })
        );
    }

    #[test]
    fn parse_unknown_percent_header() {
        let result = parse_header_line("%CUSTOM x").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Fallback(BmsHeaderFallback {
                command: "CUSTOM",
                value: "x"
            })
        );
    }

    #[test]
    fn empty_line_returns_none() {
        assert_eq!(parse_header_line("").unwrap(), None);
    }

    #[test]
    fn whitespace_only_returns_none() {
        assert_eq!(parse_header_line("   ").unwrap(), None);
    }

    #[test]
    fn comment_line_returns_none() {
        assert_eq!(parse_header_line("// comment").unwrap(), None);
    }

    #[test]
    fn just_hash_returns_none() {
        assert_eq!(parse_header_line("#").unwrap(), None);
    }

    #[test]
    fn just_percent_returns_none() {
        assert_eq!(parse_header_line("%").unwrap(), None);
    }

    #[test]
    fn case_insensitive_title() {
        let result = parse_header_line("#title lowercase").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("lowercase"))
        );
    }

    #[test]
    fn case_insensitive_wav() {
        let result = parse_header_line("#wav01 sound.wav").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                id: BmsChannelId::<WavTag>::try_from("01").unwrap(),
                filename: "sound.wav"
            })
        );
    }

    #[test]
    fn value_with_multiple_spaces() {
        let result = parse_header_line("#TITLE   My   Song").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("My   Song"))
        );
    }

    #[test]
    fn non_header_line() {
        assert_eq!(parse_header_line("just some text").unwrap(), None);
    }

    #[test]
    fn channel_line_not_confused() {
        assert_eq!(parse_header_line("#00111:1122").unwrap(), None);
    }

    #[test]
    fn hash_comment_line() {
        assert_eq!(parse_header_line("## just a comment").unwrap(), None);
    }

    #[test]
    fn invalid_player_returns_error() {
        assert!(parse_header_line("#PLAYER xyz").is_err());
    }

    #[test]
    fn invalid_difficulty_returns_error() {
        assert!(parse_header_line("#DIFFICULTY 0").is_err());
    }

    #[test]
    fn invalid_rank_returns_error() {
        assert!(parse_header_line("#RANK abc").is_err());
    }

    #[test]
    fn invalid_wav_index_returns_error() {
        assert!(parse_header_line("#WAV!! file.wav").is_err());
    }

    #[test]
    fn invalid_bpm_returns_error() {
        assert!(parse_header_line("#BPM notanumber").is_err());
    }

    #[test]
    fn parse_exbpm_indexed() {
        let result = parse_header_line("#EXBPM01 180.0").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::ExBpm {
                id: BmsChannelId::<BpmTag>::try_from("01").unwrap(),
                value: 180.0
            })
        );
    }

    #[test]
    fn parse_stp_with_position() {
        let result = parse_header_line("#STP 001.128 500").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::Stp {
                measure: 1,
                position: 128,
                duration_ms: 500.0
            })
        );
    }

    #[test]
    fn parse_stp_without_position() {
        let result = parse_header_line("#STP 001 500.5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::Stp {
                measure: 1,
                position: 0,
                duration_ms: 500.5
            })
        );
    }

    #[test]
    fn parse_stp_invalid_fallback() {
        let result = parse_header_line("#STP invalid").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }

    #[test]
    fn parse_genre_alias() {
        let result = parse_header_line("#GENLE Pop").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Metadata(BmsHeaderMetadata::Genre("Pop")));
    }

    #[test]
    fn parse_random_alias() {
        let result = parse_header_line("#RONDAM 5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(5))
        );
    }

    #[test]
    fn parse_base62() {
        let result = parse_header_line("#BASE 62").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Base62));
    }

    #[test]
    fn parse_base_other_fallback() {
        let result = parse_header_line("#BASE 36").unwrap().unwrap();
        assert!(matches!(result, BmsHeader::Fallback(_)));
    }
}
