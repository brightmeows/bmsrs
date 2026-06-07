//! BMS header command parsing, categorized by semantic domain.

mod control_flow;
mod display;
mod gameplay;
mod metadata;
mod res_def_audio;
mod res_def_visual;
mod timing;

pub use control_flow::BmsHeaderControlFlow;
pub use display::BmsHeaderDisplay;
pub use gameplay::BmsHeaderGameplay;
pub use metadata::BmsHeaderMetadata;
pub use res_def_audio::BmsHeaderResDefAudio;
pub use res_def_visual::BmsHeaderResDefVisual;
pub use timing::BmsHeaderTiming;

/// A header command from a BMS file, categorized by semantic domain.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BmsHeader<'a> {
    /// Song/chart identification (title, artist, genre, ...).
    #[serde(borrow)]
    Metadata(BmsHeaderMetadata<'a>),
    /// Gameplay behaviour (player count, rank, total, LN settings, ...).
    Gameplay(BmsHeaderGameplay<'a>),
    /// Display and difficulty markers.
    Display(BmsHeaderDisplay<'a>),
    /// Timing definitions (BPM, stops, scroll, speed).
    Timing(BmsHeaderTiming<'a>),
    /// Audio resource definitions (WAV files, CDDA, ...).
    ResDefAudio(BmsHeaderResDefAudio<'a>),
    /// Visual resource definitions (BMP, BGA, video, ...).
    ResDefVisual(BmsHeaderResDefVisual<'a>),
    /// Control-flow commands (random, if, switch, ...).
    ControlFlow(BmsHeaderControlFlow<'a>),
    /// An unrecognised or engine-specific header command.
    Ext(BmsHeaderExt<'a>),
}

/// Catch-all for unrecognised header commands.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BmsHeaderExt<'a> {
    /// The raw command name as it appears in the file (e.g., `"MYEXT"`).
    #[serde(borrow)]
    pub command: &'a str,
    /// The value after the space separator.
    #[serde(borrow)]
    pub value: &'a str,
}

/// All non-indexed header commands (exact match).
fn match_non_indexed<'a>(command: &str, value: &'a str) -> Option<BmsHeader<'a>> {
    match command {
        // -- Metadata --
        "TITLE" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Title(value))),
        "SUBTITLE" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Subtitle(value))),
        "ARTIST" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Artist(value))),
        "SUBARTIST" => Some(BmsHeader::Metadata(BmsHeaderMetadata::SubArtist(value))),
        "GENRE" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Genre(value))),
        "MAKER" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Maker(value))),
        "COMMENT" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Comment(value))),
        "TEXT" | "SONG" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Text(value))),
        "CHARSET" => Some(BmsHeader::Metadata(BmsHeaderMetadata::Charset(value))),
        // -- Gameplay --
        "PLAYER" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::Player(value))),
        "RANK" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::Rank(value))),
        "DEFEXRANK" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::DefExRank(value))),
        "TOTAL" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::Total(value))),
        "VOLWAV" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::VolWav(value))),
        "LNTYPE" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::LnType(value))),
        "LNOBJ" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::LnObj(value))),
        "LNMODE" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::LnMode(value))),
        "OCT" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::Oct(value))),
        "FP" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::Fp(value))),
        "OPTION" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::Option(value))),
        "CHANGEOPTION" => Some(BmsHeader::Gameplay(BmsHeaderGameplay::ChangeOption(value))),
        // -- Display --
        "STAGEFILE" => Some(BmsHeader::Display(BmsHeaderDisplay::StageFile(value))),
        "BANNER" => Some(BmsHeader::Display(BmsHeaderDisplay::Banner(value))),
        "BACKBMP" => Some(BmsHeader::Display(BmsHeaderDisplay::BackBmp(value))),
        "CHARFILE" => Some(BmsHeader::Display(BmsHeaderDisplay::CharFile(value))),
        "PLAYLEVEL" => Some(BmsHeader::Display(BmsHeaderDisplay::PlayLevel(value))),
        "DIFFICULTY" => Some(BmsHeader::Display(BmsHeaderDisplay::Difficulty(value))),
        "PREVIEW" => Some(BmsHeader::Display(BmsHeaderDisplay::Preview(value))),
        // -- Timing --
        "BPM" => Some(BmsHeader::Timing(BmsHeaderTiming::Bpm(value))),
        "BASEBPM" => Some(BmsHeader::Timing(BmsHeaderTiming::BaseBpm(value))),
        // -- Audio resources --
        "WAVCMD" => Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd(value))),
        "CDDA" => Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Cdda(value))),
        "MIDIFILE" => Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Midifile(
            value,
        ))),
        "PATH_WAV" => Some(BmsHeader::ResDefAudio(BmsHeaderResDefAudio::PathWav(value))),
        // -- Visual resources --
        "POORBGA" => Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(
            value,
        ))),
        "VIDEOFILE" => Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::VideoFile(
            value,
        ))),
        "MOVIE" => Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Movie(value))),
        "EXTCHR" => Some(BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExtChr(
            value,
        ))),
        // -- Control flow (value-less) --
        "ENDRANDOM" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::EndRandom)),
        "ELSE" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Else)),
        "ENDIF" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::EndIf)),
        "ENDSW" | "ENDSWITCH" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)),
        "DEF" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Def)),
        // -- Control flow (with value) --
        "RANDOM" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(value))),
        "SETRANDOM" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::SetRandom(
            value,
        ))),
        "IF" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::If(value))),
        "ELSEIF" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::ElseIf(value))),
        "SWITCH" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Switch(value))),
        "SETSWITCH" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::SetSwitch(
            value,
        ))),
        "CASE" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Case(value))),
        "SKIP" => Some(BmsHeader::ControlFlow(BmsHeaderControlFlow::Skip(value))),
        // -- Not matched --
        _ => None,
    }
}

/// Known indexed command bases (longest first to avoid prefix collisions).
const INDEXED_BASES: &[&str] = &[
    "EXBMP", "EXWAV", "EXRANK", "SCROLL", "SPEED", "SWBGA", "@BGA", "ARGB", "BGA", "BMP", "BPM",
    "STOP", "SEEK", "WAV",
];

/// Try to match an indexed command (base + 2-char index suffix).
///
/// `command_raw` provides the index slice (borrowed from input, lifetime `'a`),
/// while `command_upper` is the uppercased version used for base matching.
fn match_indexed<'a>(
    command_upper: &str,
    command_raw: &'a str,
    value: &'a str,
) -> Option<BmsHeader<'a>> {
    let cmd_len = command_raw.len();

    if cmd_len < 5 {
        return None;
    }

    for &base in INDEXED_BASES {
        if cmd_len == base.len() + 2 && command_upper.starts_with(base) {
            let idx_start = base.len();
            let idx = &command_raw[idx_start..];

            if !idx.bytes().all(|b| b.is_ascii_alphanumeric()) {
                return None;
            }

            return Some(match base {
                "WAV" => BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                    index: idx,
                    filename: value,
                }),
                "EXWAV" => BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav {
                    index: idx,
                    filename: value,
                }),
                "BMP" => BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bmp {
                    index: idx,
                    filename: value,
                }),
                "EXBMP" => BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExBmp {
                    index: idx,
                    filename: value,
                }),
                "BGA" => BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga {
                    index: idx,
                    filename: value,
                }),
                "@BGA" => BmsHeader::ResDefVisual(BmsHeaderResDefVisual::AtBga {
                    index: idx,
                    filename: value,
                }),
                "SWBGA" => BmsHeader::ResDefVisual(BmsHeaderResDefVisual::SwBga {
                    index: idx,
                    filename: value,
                }),
                "ARGB" => BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Argb {
                    index: idx,
                    filename: value,
                }),
                "SEEK" => {
                    BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Seek { index: idx, value })
                }
                "BPM" => BmsHeader::Timing(BmsHeaderTiming::BpmDef { index: idx, value }),
                "STOP" => BmsHeader::Timing(BmsHeaderTiming::StopDef { index: idx, value }),
                "SCROLL" => BmsHeader::Timing(BmsHeaderTiming::ScrollDef { index: idx, value }),
                "SPEED" => BmsHeader::Timing(BmsHeaderTiming::SpeedDef { index: idx, value }),
                "EXRANK" => BmsHeader::Gameplay(BmsHeaderGameplay::ExRank { index: idx, value }),
                _ => unreachable!("all indexed bases must be handled"),
            });
        }
    }

    None
}

/// Parse a single header line into a `BmsHeader`.
///
/// Returns `None` if the line is not a header (empty, comment, channel data).
pub(crate) fn parse_header_line(line: &str) -> Option<BmsHeader<'_>> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return None;
    }

    // Lines starting with `##` are comments.
    if trimmed.starts_with("##") {
        return None;
    }

    // Determine prefix character.
    let (prefix, rest) = if let Some(r) = trimmed.strip_prefix('#') {
        ('#', r)
    } else if let Some(r) = trimmed.strip_prefix('%') {
        ('%', r)
    } else {
        return None;
    };

    if rest.is_empty() {
        return None;
    }

    // Split command name and value at first whitespace.
    let split_pos = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let command_raw = &rest[..split_pos];
    let value = rest[split_pos..].trim();

    if command_raw.is_empty() {
        return None;
    }

    // If the raw command contains a colon it is a channel message, not a header.
    if command_raw.contains(':') {
        return None;
    }

    // -- % commands: URL / EMAIL are metadata; everything else is Extension. --
    if prefix == '%' {
        let command_upper = command_raw.to_uppercase();
        return Some(match command_upper.as_str() {
            "URL" => BmsHeader::Metadata(BmsHeaderMetadata::Url(value)),
            "EMAIL" => BmsHeader::Metadata(BmsHeaderMetadata::Email(value)),
            _ => BmsHeader::Ext(BmsHeaderExt {
                command: command_raw,
                value,
            }),
        });
    }

    let command_upper = command_raw.to_uppercase();

    // Try exact match first (non-indexed).
    if let Some(header) = match_non_indexed(&command_upper, value) {
        return Some(header);
    }

    // Try indexed match (base + 2-char suffix).
    if let Some(header) = match_indexed(&command_upper, command_raw, value) {
        return Some(header);
    }

    // Nothing matched → Extension.
    Some(BmsHeader::Ext(BmsHeaderExt {
        command: command_raw,
        value,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Metadata ----

    #[test]
    fn parse_title() {
        let result = parse_header_line("#TITLE My Song").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("My Song"))
        );
    }

    #[test]
    fn parse_title_empty() {
        let result = parse_header_line("#TITLE").unwrap();
        assert_eq!(result, BmsHeader::Metadata(BmsHeaderMetadata::Title("")));
    }

    #[test]
    fn parse_subtitle() {
        let result = parse_header_line("#SUBTITLE (short ver.)").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Subtitle("(short ver.)"))
        );
    }

    #[test]
    fn parse_artist() {
        let result = parse_header_line("#ARTIST composer").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Artist("composer"))
        );
    }

    #[test]
    fn parse_subartist() {
        let result = parse_header_line("#SUBARTIST co-writer").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::SubArtist("co-writer"))
        );
    }

    #[test]
    fn parse_genre() {
        let result = parse_header_line("#GENRE Piano").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Genre("Piano"))
        );
    }

    #[test]
    fn parse_maker() {
        let result = parse_header_line("#MAKER chart-creator").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Maker("chart-creator"))
        );
    }

    #[test]
    fn parse_comment() {
        let result = parse_header_line("#COMMENT hello").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Comment("hello"))
        );
    }

    #[test]
    fn parse_text() {
        let result = parse_header_line("#TEXT in-game text").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Text("in-game text"))
        );
    }

    #[test]
    fn parse_song_as_text() {
        let result = parse_header_line("#SONG some text").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Text("some text"))
        );
    }

    #[test]
    fn parse_charset() {
        let result = parse_header_line("#CHARSET UTF-8").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Charset("UTF-8"))
        );
    }

    #[test]
    fn parse_url() {
        let result = parse_header_line("%URL https://example.com").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Url("https://example.com"))
        );
    }

    #[test]
    fn parse_email() {
        let result = parse_header_line("%EMAIL user@example.com").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Email("user@example.com"))
        );
    }

    // ---- Gameplay ----

    #[test]
    fn parse_player() {
        let result = parse_header_line("#PLAYER 1").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Player("1")));
    }

    #[test]
    fn parse_rank() {
        let result = parse_header_line("#RANK 2").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Rank("2")));
    }

    #[test]
    fn parse_defexrank() {
        let result = parse_header_line("#DEFEXRANK 3").unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::DefExRank("3"))
        );
    }

    #[test]
    fn parse_total() {
        let result = parse_header_line("#TOTAL 300").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Total("300")));
    }

    #[test]
    fn parse_volwav() {
        let result = parse_header_line("#VOLWAV 100").unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::VolWav("100"))
        );
    }

    #[test]
    fn parse_lntype() {
        let result = parse_header_line("#LNTYPE 1").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::LnType("1")));
    }

    #[test]
    fn parse_lnobj() {
        let result = parse_header_line("#LNOBJ 01").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::LnObj("01")));
    }

    #[test]
    fn parse_lnmode() {
        let result = parse_header_line("#LNMODE 1").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::LnMode("1")));
    }

    #[test]
    fn parse_oct() {
        let result = parse_header_line("#OCT 1").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Oct("1")));
    }

    #[test]
    fn parse_fp() {
        let result = parse_header_line("#FP 1").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Fp("1")));
    }

    #[test]
    fn parse_option() {
        let result = parse_header_line("#OPTION -R").unwrap();
        assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Option("-R")));
    }

    #[test]
    fn parse_changeoption() {
        let result = parse_header_line("#CHANGEOPTION RANDOM").unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::ChangeOption("RANDOM"))
        );
    }

    // ---- Display ----

    #[test]
    fn parse_stagefile() {
        let result = parse_header_line("#STAGEFILE stage.png").unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::StageFile("stage.png"))
        );
    }

    #[test]
    fn parse_banner() {
        let result = parse_header_line("#BANNER banner.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Banner("banner.bmp"))
        );
    }

    #[test]
    fn parse_backbmp() {
        let result = parse_header_line("#BACKBMP bg.png").unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::BackBmp("bg.png"))
        );
    }

    #[test]
    fn parse_charfile() {
        let result = parse_header_line("#CHARFILE char.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::CharFile("char.bmp"))
        );
    }

    #[test]
    fn parse_playlevel() {
        let result = parse_header_line("#PLAYLEVEL 12").unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::PlayLevel("12"))
        );
    }

    #[test]
    fn parse_difficulty() {
        let result = parse_header_line("#DIFFICULTY 3").unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Difficulty("3"))
        );
    }

    #[test]
    fn parse_preview() {
        let result = parse_header_line("#PREVIEW preview.ogg").unwrap();
        assert_eq!(
            result,
            BmsHeader::Display(BmsHeaderDisplay::Preview("preview.ogg"))
        );
    }

    // ---- Timing ----

    #[test]
    fn parse_bpm_global() {
        let result = parse_header_line("#BPM 180").unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm("180")));
    }

    #[test]
    fn parse_bpm_global_float() {
        let result = parse_header_line("#BPM 180.0").unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm("180.0")));
    }

    #[test]
    fn parse_basebpm() {
        let result = parse_header_line("#BASEBPM 180").unwrap();
        assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::BaseBpm("180")));
    }

    // ---- ResDefAudio ----

    #[test]
    fn parse_wavcmd() {
        let result = parse_header_line("#WAVCMD some-command").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd("some-command"))
        );
    }

    #[test]
    fn parse_cdda() {
        let result = parse_header_line("#CDDA track01.bin").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Cdda("track01.bin"))
        );
    }

    #[test]
    fn parse_midifile() {
        let result = parse_header_line("#MIDIFILE song.mid").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Midifile("song.mid"))
        );
    }

    #[test]
    fn parse_path_wav() {
        let result = parse_header_line("#PATH_WAV ./sounds/").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::PathWav("./sounds/"))
        );
    }

    // ---- ResDefVisual ----

    #[test]
    fn parse_poorbga() {
        let result = parse_header_line("#POORBGA fallback.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga("fallback.bmp"))
        );
    }

    #[test]
    fn parse_videofile() {
        let result = parse_header_line("#VIDEOFILE bg.avi").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::VideoFile("bg.avi"))
        );
    }

    #[test]
    fn parse_movie() {
        let result = parse_header_line("#MOVIE intro.mpg").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Movie("intro.mpg"))
        );
    }

    #[test]
    fn parse_extchr() {
        let result = parse_header_line("#ExtChr extra").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExtChr("extra"))
        );
    }

    // ---- ControlFlow ----

    #[test]
    fn parse_random() {
        let result = parse_header_line("#RANDOM 10").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Random("10"))
        );
    }

    #[test]
    fn parse_setrandom() {
        let result = parse_header_line("#SETRANDOM 5").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::SetRandom("5"))
        );
    }

    #[test]
    fn parse_if() {
        let result = parse_header_line("#IF 1").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::If("1"))
        );
    }

    #[test]
    fn parse_elseif() {
        let result = parse_header_line("#ELSEIF 0").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::ElseIf("0"))
        );
    }

    #[test]
    fn parse_else() {
        let result = parse_header_line("#ELSE").unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Else));
    }

    #[test]
    fn parse_endif() {
        let result = parse_header_line("#ENDIF").unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::EndIf));
    }

    #[test]
    fn parse_endrandom() {
        let result = parse_header_line("#ENDRANDOM").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndRandom)
        );
    }

    #[test]
    fn parse_switch() {
        let result = parse_header_line("#SWITCH 3").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Switch("3"))
        );
    }

    #[test]
    fn parse_setswitch() {
        let result = parse_header_line("#SETSWITCH 2").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::SetSwitch("2"))
        );
    }

    #[test]
    fn parse_case() {
        let result = parse_header_line("#CASE 1").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Case("1"))
        );
    }

    #[test]
    fn parse_skip() {
        let result = parse_header_line("#SKIP 1").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::Skip("1"))
        );
    }

    #[test]
    fn parse_def() {
        let result = parse_header_line("#DEF").unwrap();
        assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Def));
    }

    #[test]
    fn parse_endsw() {
        let result = parse_header_line("#ENDSW").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
        );
    }

    #[test]
    fn parse_endswitch_alias() {
        let result = parse_header_line("#ENDSWITCH").unwrap();
        assert_eq!(
            result,
            BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
        );
    }

    // ---- Indexed commands ----

    #[test]
    fn parse_wav_indexed() {
        let result = parse_header_line("#WAV01 kick.wav").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                index: "01",
                filename: "kick.wav"
            })
        );
    }

    #[test]
    fn parse_wav_36ary_index() {
        let result = parse_header_line("#WAV2A snare.wav").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                index: "2A",
                filename: "snare.wav"
            })
        );
    }

    #[test]
    fn parse_exwav_indexed() {
        let result = parse_header_line("#EXWAV01 extra.ogg").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav {
                index: "01",
                filename: "extra.ogg"
            })
        );
    }

    #[test]
    fn parse_bmp_indexed() {
        let result = parse_header_line("#BMP01 bg.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bmp {
                index: "01",
                filename: "bg.bmp"
            })
        );
    }

    #[test]
    fn parse_exbmp_indexed() {
        let result = parse_header_line("#EXBMP01 extra.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExBmp {
                index: "01",
                filename: "extra.bmp"
            })
        );
    }

    #[test]
    fn parse_bga_indexed() {
        let result = parse_header_line("#BGA01 layer.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga {
                index: "01",
                filename: "layer.bmp"
            })
        );
    }

    #[test]
    fn parse_at_bga_indexed() {
        let result = parse_header_line("#@BGA01 layer.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::AtBga {
                index: "01",
                filename: "layer.bmp"
            })
        );
    }

    #[test]
    fn parse_swbga_indexed() {
        let result = parse_header_line("#SWBGA01 switch.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::SwBga {
                index: "01",
                filename: "switch.bmp"
            })
        );
    }

    #[test]
    fn parse_argb_indexed() {
        let result = parse_header_line("#ARGB01 rgba.bmp").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Argb {
                index: "01",
                filename: "rgba.bmp"
            })
        );
    }

    #[test]
    fn parse_seek_indexed() {
        let result = parse_header_line("#SEEK01 1.5").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Seek {
                index: "01",
                value: "1.5"
            })
        );
    }

    #[test]
    fn parse_bpm_def_indexed() {
        let result = parse_header_line("#BPM01 180.0").unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::BpmDef {
                index: "01",
                value: "180.0"
            })
        );
    }

    #[test]
    fn parse_stop_indexed() {
        let result = parse_header_line("#STOP01 192").unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::StopDef {
                index: "01",
                value: "192"
            })
        );
    }

    #[test]
    fn parse_scroll_indexed() {
        let result = parse_header_line("#SCROLL01 1.5").unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::ScrollDef {
                index: "01",
                value: "1.5"
            })
        );
    }

    #[test]
    fn parse_speed_indexed() {
        let result = parse_header_line("#SPEED01 2.0").unwrap();
        assert_eq!(
            result,
            BmsHeader::Timing(BmsHeaderTiming::SpeedDef {
                index: "01",
                value: "2.0"
            })
        );
    }

    #[test]
    fn parse_exrank_indexed() {
        let result = parse_header_line("#EXRANK01 5").unwrap();
        assert_eq!(
            result,
            BmsHeader::Gameplay(BmsHeaderGameplay::ExRank {
                index: "01",
                value: "5"
            })
        );
    }

    // ---- WAVCMD is not confused with WAV + index ----

    #[test]
    fn wavcmd_not_confused_as_wav_indexed() {
        let result = parse_header_line("#WAVCMD test").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd("test"))
        );
    }

    // ---- Extension ----

    #[test]
    fn parse_unknown_header() {
        let result = parse_header_line("#MYEXT abc123").unwrap();
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
        let result = parse_header_line("%CUSTOM x").unwrap();
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
        assert_eq!(parse_header_line(""), None);
    }

    #[test]
    fn whitespace_only_returns_none() {
        assert_eq!(parse_header_line("   "), None);
    }

    #[test]
    fn comment_line_returns_none() {
        assert_eq!(parse_header_line("// comment"), None);
    }

    #[test]
    fn just_hash_returns_none() {
        assert_eq!(parse_header_line("#"), None);
    }

    #[test]
    fn just_percent_returns_none() {
        assert_eq!(parse_header_line("%"), None);
    }

    #[test]
    fn case_insensitive_title() {
        let result = parse_header_line("#title lowercase").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("lowercase"))
        );
    }

    #[test]
    fn case_insensitive_wav() {
        let result = parse_header_line("#wav01 sound.wav").unwrap();
        assert_eq!(
            result,
            BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
                index: "01",
                filename: "sound.wav"
            })
        );
    }

    #[test]
    fn value_with_multiple_spaces() {
        let result = parse_header_line("#TITLE   My   Song").unwrap();
        assert_eq!(
            result,
            BmsHeader::Metadata(BmsHeaderMetadata::Title("My   Song"))
        );
    }

    #[test]
    fn non_header_line() {
        assert_eq!(parse_header_line("just some text"), None);
    }

    #[test]
    fn channel_line_not_confused() {
        assert_eq!(parse_header_line("#00111:1122"), None);
    }

    #[test]
    fn hash_comment_line() {
        assert_eq!(parse_header_line("## just a comment"), None);
    }
}
