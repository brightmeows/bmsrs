//! BMS header command parsing, categorized by semantic domain.

mod control_flow;
mod display;
mod gameplay;
mod metadata;
mod res_def_audio;
mod res_def_visual;
mod timing;

pub use control_flow::BmsHeaderControlFlow;
pub use display::{BmsHeaderDisplay, DifficultyLevel, ParseDifficultyError};
pub use gameplay::{
    BmsHeaderGameplay, LnMode, LnType, ParseLnModeError, ParseLnTypeError, ParsePlayerModeError,
    PlayerMode,
};
pub use metadata::BmsHeaderMetadata;
pub use res_def_audio::BmsHeaderResDefAudio;
pub use res_def_visual::BmsHeaderResDefVisual;
pub use timing::BmsHeaderTiming;

use crate::error::BmsTokenizeError;
use crate::id::{
    BmpTag, BmsChannelId, BpmTag, ExRankTag, LnObjTag, ScrollTag, SeekTag, SpeedTag, StopTag,
    WavTag,
};

/// A header command from a BMS file, categorized by semantic domain.
#[derive(Debug, Clone, PartialEq)]
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
    Ext(BmsHeaderExt<'a>),
}

/// Catch-all for unrecognised header commands.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsHeaderExt<'a> {
    /// The raw command name as it appears in the file (e.g., `"MYEXT"`).
    pub command: &'a str,
    /// The value after the space separator.
    pub value: &'a str,
}

/// All non-indexed header commands (exact match).
#[expect(
    clippy::too_many_lines,
    reason = "one arm per header variant, ~30 variants"
)]
fn match_non_indexed<'a>(
    command: &str,
    value: &'a str,
) -> Result<Option<BmsHeader<'a>>, BmsTokenizeError<'a>> {
    match command {
        // -- Metadata (free-text, no parsing needed) --
        "TITLE" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Title(value)))),
        "SUBTITLE" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Subtitle(
            value,
        )))),
        "ARTIST" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Artist(value)))),
        "SUBARTIST" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::SubArtist(
            value,
        )))),
        "GENRE" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Genre(value)))),
        "MAKER" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Maker(value)))),
        "COMMENT" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Comment(value)))),
        "TEXT" | "SONG" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Text(value)))),
        "CHARSET" => Ok(Some(BmsHeader::Metadata(BmsHeaderMetadata::Charset(value)))),
        // -- Gameplay (typed) --
        "PLAYER" => {
            let v: PlayerMode = value.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                context: "#PLAYER",
                value,
            })?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::Player(v))))
        }
        "RANK" => {
            let v: u8 = value.parse()?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::Rank(v))))
        }
        "DEFEXRANK" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::DefExRank(v))))
        }
        "TOTAL" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::Total(v))))
        }
        "VOLWAV" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::VolWav(v))))
        }
        "LNTYPE" => {
            let v: LnType = value.parse().map_err(|_| BmsTokenizeError::OutOfRange {
                context: "#LNTYPE",
                value,
                expected: "1 or 2",
            })?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::LnType(v))))
        }
        "LNOBJ" => {
            let id: BmsChannelId<LnObjTag> =
                value.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                    context: "#LNOBJ",
                    value,
                })?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::LnObj(id))))
        }
        "LNMODE" => {
            let v: LnMode = value.parse().map_err(|_| BmsTokenizeError::OutOfRange {
                context: "#LNMODE",
                value,
                expected: "1, 2, or 3",
            })?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::LnMode(v))))
        }
        "OCT" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::Oct(v))))
        }
        "FP" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::Fp(v))))
        }
        "OPTION" => Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::Option(value)))),
        "CHANGEOPTION" => Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::ChangeOption(
            value,
        )))),
        // -- Display (some typed) --
        "STAGEFILE" => Ok(Some(BmsHeader::Display(BmsHeaderDisplay::StageFile(value)))),
        "BANNER" => Ok(Some(BmsHeader::Display(BmsHeaderDisplay::Banner(value)))),
        "BACKBMP" => Ok(Some(BmsHeader::Display(BmsHeaderDisplay::BackBmp(value)))),
        "CHARFILE" => Ok(Some(BmsHeader::Display(BmsHeaderDisplay::CharFile(value)))),
        "PLAYLEVEL" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Display(BmsHeaderDisplay::PlayLevel(v))))
        }
        "DIFFICULTY" => {
            let v: DifficultyLevel = value.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                context: "#DIFFICULTY",
                value,
            })?;
            Ok(Some(BmsHeader::Display(BmsHeaderDisplay::Difficulty(v))))
        }
        "PREVIEW" => Ok(Some(BmsHeader::Display(BmsHeaderDisplay::Preview(value)))),
        // -- Timing (typed) --
        "BPM" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Timing(BmsHeaderTiming::Bpm(v))))
        }
        "BASEBPM" => {
            let v: f64 = value.parse()?;
            Ok(Some(BmsHeader::Timing(BmsHeaderTiming::BaseBpm(v))))
        }
        // -- Audio resources (free-text) --
        "WAVCMD" => Ok(Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd(
            value,
        )))),
        "CDDA" => Ok(Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Cdda(
            value,
        )))),
        "MIDIFILE" => Ok(Some(BmsHeader::ResDefAudio(
            BmsHeaderResDefAudio::Midifile(value),
        ))),
        "PATH_WAV" => Ok(Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::PathWav(
            value,
        )))),
        // -- Visual resources (free-text) --
        "POORBGA" => Ok(Some(BmsHeader::ResDefVisual(
            BmsHeaderResDefVisual::PoorBga(value),
        ))),
        "VIDEOFILE" => Ok(Some(BmsHeader::ResDefVisual(
            BmsHeaderResDefVisual::VideoFile(value),
        ))),
        "MOVIE" => Ok(Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Movie(
            value,
        )))),
        "EXTCHR" => Ok(Some(BmsHeader::ResDefVisual(
            BmsHeaderResDefVisual::ExtChr(value),
        ))),
        // -- Control flow (value-less) --
        "ENDRANDOM" => Ok(Some(BmsHeader::ControlFlow(
            BmsHeaderControlFlow::EndRandom,
        ))),
        "ELSE" => Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Else))),
        "ENDIF" => Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::EndIf))),
        "ENDSW" | "ENDSWITCH" => Ok(Some(BmsHeader::ControlFlow(
            BmsHeaderControlFlow::EndSwitch,
        ))),
        "DEF" => Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Def))),
        // -- Control flow (typed) --
        "RANDOM" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(
                v,
            ))))
        }
        "SETRANDOM" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(
                BmsHeaderControlFlow::SetRandom(v),
            )))
        }
        "IF" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::If(v))))
        }
        "ELSEIF" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::ElseIf(
                v,
            ))))
        }
        "SWITCH" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Switch(
                v,
            ))))
        }
        "SETSWITCH" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(
                BmsHeaderControlFlow::SetSwitch(v),
            )))
        }
        "CASE" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Case(v))))
        }
        "SKIP" => {
            let v: u64 = value.parse()?;
            Ok(Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Skip(v))))
        }
        // -- Not matched --
        _ => Ok(None),
    }
}

/// Known indexed command bases (longest first to avoid prefix collisions).
const INDEXED_BASES: &[&str] = &[
    "EXBMP", "EXWAV", "EXRANK", "SCROLL", "SPEED", "SWBGA", "@BGA", "ARGB", "BGA", "BMP", "BPM",
    "STOP", "SEEK", "WAV",
];

/// All bases that have a match arm in `match_indexed`.
/// Must be kept in the same order as `INDEXED_BASES`.
#[cfg(test)]
const INDEXED_BASES_HANDLED: &[&str] = &[
    "EXBMP", "EXWAV", "EXRANK", "SCROLL", "SPEED", "SWBGA", "@BGA", "ARGB", "BGA", "BMP", "BPM",
    "STOP", "SEEK", "WAV",
];

/// Try to match an indexed command (base + 2-char index suffix).
///
/// `command_raw` provides the index slice (borrowed from input, lifetime `'a`),
/// while `command_upper` is the uppercased version used for base matching.
#[expect(clippy::too_many_lines, reason = "one arm per indexed base, ~14 bases")]
fn match_indexed<'a>(
    command_upper: &str,
    command_raw: &'a str,
    value: &'a str,
) -> Result<Option<BmsHeader<'a>>, BmsTokenizeError<'a>> {
    let cmd_len = command_raw.len();

    if cmd_len < 5 {
        return Ok(None);
    }

    for &base in INDEXED_BASES {
        if cmd_len == base.len() + 2 && command_upper.starts_with(base) {
            let idx_start = base.len();
            let idx = &command_raw[idx_start..];

            return match base {
                "WAV" => {
                    let id: BmsChannelId<WavTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#WAV",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                        index: id,
                        filename: value,
                    })))
                }
                "EXWAV" => {
                    let id: BmsChannelId<WavTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#WAV",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav {
                        index: id,
                        filename: value,
                    })))
                }
                "BMP" => {
                    let id: BmsChannelId<BmpTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#BMP",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bmp {
                        index: id,
                        filename: value,
                    })))
                }
                "EXBMP" => {
                    let id: BmsChannelId<BmpTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#BMP",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefVisual(
                        BmsHeaderResDefVisual::ExBmp {
                            index: id,
                            filename: value,
                        },
                    )))
                }
                "BGA" => {
                    let id: BmsChannelId<BmpTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#BMP",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga {
                        index: id,
                        filename: value,
                    })))
                }
                "@BGA" => {
                    let id: BmsChannelId<BmpTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "@BGA",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefVisual(
                        BmsHeaderResDefVisual::AtBga {
                            index: id,
                            filename: value,
                        },
                    )))
                }
                "SWBGA" => {
                    let id: BmsChannelId<BmpTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#SWBGA",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefVisual(
                        BmsHeaderResDefVisual::SwBga {
                            index: id,
                            filename: value,
                        },
                    )))
                }
                "ARGB" => {
                    let id: BmsChannelId<BmpTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#ARGB",
                            value: idx,
                        })?;
                    Ok(Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Argb {
                        index: id,
                        filename: value,
                    })))
                }
                "SEEK" => {
                    let id: BmsChannelId<SeekTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#SEEK",
                            value: idx,
                        })?;
                    let v: f64 = value.parse()?;
                    Ok(Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Seek {
                        index: id,
                        value: v,
                    })))
                }
                "BPM" => {
                    let id: BmsChannelId<BpmTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#BPM",
                            value: idx,
                        })?;
                    let v: f64 = value.parse()?;
                    Ok(Some(BmsHeader::Timing(BmsHeaderTiming::BpmDef {
                        index: id,
                        value: v,
                    })))
                }
                "STOP" => {
                    let id: BmsChannelId<StopTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#STOP",
                            value: idx,
                        })?;
                    let v: f64 = value.parse()?;
                    Ok(Some(BmsHeader::Timing(BmsHeaderTiming::StopDef {
                        index: id,
                        value: v,
                    })))
                }
                "SCROLL" => {
                    let id: BmsChannelId<ScrollTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#SCROLL",
                            value: idx,
                        })?;
                    let v: f64 = value.parse()?;
                    Ok(Some(BmsHeader::Timing(BmsHeaderTiming::ScrollDef {
                        index: id,
                        value: v,
                    })))
                }
                "SPEED" => {
                    let id: BmsChannelId<SpeedTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#SPEED",
                            value: idx,
                        })?;
                    let v: f64 = value.parse()?;
                    Ok(Some(BmsHeader::Timing(BmsHeaderTiming::SpeedDef {
                        index: id,
                        value: v,
                    })))
                }
                "EXRANK" => {
                    let id: BmsChannelId<ExRankTag> =
                        idx.parse().map_err(|_| BmsTokenizeError::InvalidValue {
                            context: "#EXRANK",
                            value: idx,
                        })?;
                    let v: f64 = value.parse()?;
                    Ok(Some(BmsHeader::Gameplay(BmsHeaderGameplay::ExRank {
                        index: id,
                        value: v,
                    })))
                }
                _ => unreachable!("all indexed bases must be handled"),
            };
        }
    }

    Ok(None)
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

    // -- % commands: URL / EMAIL are metadata; everything else is Extension. --
    if prefix == '%' {
        let command_upper = command_raw.to_uppercase();
        return Ok(Some(match command_upper.as_str() {
            "URL" => BmsHeader::Metadata(BmsHeaderMetadata::Url(value)),
            "EMAIL" => BmsHeader::Metadata(BmsHeaderMetadata::Email(value)),
            _ => BmsHeader::Ext(BmsHeaderExt {
                command: command_raw,
                value,
            }),
        }));
    }

    let command_upper = command_raw.to_uppercase();

    // Try exact match first (non-indexed).
    if let Some(header) = match_non_indexed(&command_upper, value)? {
        return Ok(Some(header));
    }

    // Try indexed match (base + 2-char suffix).
    if let Some(header) = match_indexed(&command_upper, command_raw, value)? {
        return Ok(Some(header));
    }

    // Nothing matched → Extension.
    Ok(Some(BmsHeader::Ext(BmsHeaderExt {
        command: command_raw,
        value,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_indexed_bases_have_match_arms() {
        assert_eq!(
            INDEXED_BASES, INDEXED_BASES_HANDLED,
            "INDEXED_BASES and INDEXED_BASES_HANDLED must match exactly"
        );
    }

    // ---- Metadata ----

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

    // ---- Gameplay ----

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
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Oct(1.0)));
    }

    #[test]
    fn parse_fp() {
        let result = parse_header_line("#FP 1").unwrap().unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Fp(1.0)));
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

    // ---- Display ----

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

    // ---- Timing ----

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

    // ---- ResDefAudio ----

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

    // ---- ResDefVisual ----

    #[test]
    fn parse_poorbga() {
        let result = parse_header_line("#POORBGA fallback.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga("fallback.bmp"))
        );
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

    // ---- ControlFlow ----

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

    // ---- Indexed commands ----

    #[test]
    fn parse_wav_indexed() {
        let result = parse_header_line("#WAV01 kick.wav").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                index: BmsChannelId::<WavTag>::try_from("01").unwrap(),
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
                index: BmsChannelId::<WavTag>::try_from("2A").unwrap(),
                filename: "snare.wav"
            })
        );
    }

    #[test]
    fn parse_exwav_indexed() {
        let result = parse_header_line("#EXWAV01 extra.ogg").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav {
                index: BmsChannelId::<WavTag>::try_from("01").unwrap(),
                filename: "extra.ogg"
            })
        );
    }

    #[test]
    fn parse_bmp_indexed() {
        let result = parse_header_line("#BMP01 bg.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bmp {
                index: BmsChannelId::<BmpTag>::try_from("01").unwrap(),
                filename: "bg.bmp"
            })
        );
    }

    #[test]
    fn parse_exbmp_indexed() {
        let result = parse_header_line("#EXBMP01 extra.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExBmp {
                index: BmsChannelId::<BmpTag>::try_from("01").unwrap(),
                filename: "extra.bmp"
            })
        );
    }

    #[test]
    fn parse_bga_indexed() {
        let result = parse_header_line("#BGA01 layer.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga {
                index: BmsChannelId::<BmpTag>::try_from("01").unwrap(),
                filename: "layer.bmp"
            })
        );
    }

    #[test]
    fn parse_at_bga_indexed() {
        let result = parse_header_line("#@BGA01 layer.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::AtBga {
                index: BmsChannelId::<BmpTag>::try_from("01").unwrap(),
                filename: "layer.bmp"
            })
        );
    }

    #[test]
    fn parse_swbga_indexed() {
        let result = parse_header_line("#SWBGA01 switch.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::SwBga {
                index: BmsChannelId::<BmpTag>::try_from("01").unwrap(),
                filename: "switch.bmp"
            })
        );
    }

    #[test]
    fn parse_argb_indexed() {
        let result = parse_header_line("#ARGB01 rgba.bmp").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Argb {
                index: BmsChannelId::<BmpTag>::try_from("01").unwrap(),
                filename: "rgba.bmp"
            })
        );
    }

    #[test]
    fn parse_seek_indexed() {
        let result = parse_header_line("#SEEK01 1.5").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Seek {
                index: BmsChannelId::<SeekTag>::try_from("01").unwrap(),
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
                index: BmsChannelId::<BpmTag>::try_from("01").unwrap(),
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
                index: BmsChannelId::<StopTag>::try_from("01").unwrap(),
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
                index: BmsChannelId::<ScrollTag>::try_from("01").unwrap(),
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
                index: BmsChannelId::<SpeedTag>::try_from("01").unwrap(),
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
                index: BmsChannelId::<ExRankTag>::try_from("01").unwrap(),
                value: 5.0
            })
        );
    }

    // ---- WAVCMD is not confused with WAV + index ----

    #[test]
    fn wavcmd_not_confused_as_wav_indexed() {
        let result = parse_header_line("#WAVCMD test").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd("test"))
        );
    }

    // ---- Extension ----

    #[test]
    fn parse_unknown_header() {
        let result = parse_header_line("#MYEXT abc123").unwrap().unwrap();
        assert_eq!(
            result,
            BmsHeader::Ext(BmsHeaderExt {
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
            BmsHeader::Ext(BmsHeaderExt {
                command: "CUSTOM",
                value: "x"
            })
        );
    }

    // ---- Edge cases ----

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
                index: BmsChannelId::<WavTag>::try_from("01").unwrap(),
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
}
