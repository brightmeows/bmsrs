//! Integration tests for `#[derive(BmsToken)]` roundtrip conversion.
//!
//! Each test verifies that `try_match_header` → `format_header` → parse
//! again is identity (or at least consistent).

use bms_tokenizer::{
    BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderGameplay, BmsHeaderMetadata,
    BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming, DifficultyLevel, LnMode, LnType,
    PlayerMode,
};

#[test]
fn metadata_title_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("TITLE", "TITLE", "My Song")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Title("My Song"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#TITLE");
    assert_eq!(val, "My Song");
}

#[test]
fn metadata_subtitle_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("SUBTITLE", "SUBTITLE", "(short ver.)")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Subtitle("(short ver.)"));
}

#[test]
fn metadata_artist_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("ARTIST", "ARTIST", "composer")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Artist("composer"));
}

#[test]
fn metadata_subartist_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("SUBARTIST", "SUBARTIST", "co-writer")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::SubArtist("co-writer"));
}

#[test]
fn metadata_genre_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("GENRE", "GENRE", "Piano")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Genre("Piano"));
}

#[test]
fn metadata_maker_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("MAKER", "MAKER", "creator")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Maker("creator"));
}

#[test]
fn metadata_comment_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("COMMENT", "COMMENT", "hello")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Comment("hello"));
}

#[test]
fn metadata_text_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("TEXT", "TEXT", "in-game text")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Text("in-game text"));
}

#[test]
fn metadata_song_alias() {
    let parsed = BmsHeaderMetadata::try_match_header("SONG", "SONG", "some text")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Text("some text"));
}

#[test]
fn metadata_charset_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("CHARSET", "CHARSET", "UTF-8")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Charset("UTF-8"));
}

#[test]
fn metadata_url_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("URL", "URL", "https://example.com")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Url("https://example.com"));
    let (cmd, _) = parsed.format_header();
    assert_eq!(cmd, "%URL");
}

#[test]
fn metadata_email_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("EMAIL", "EMAIL", "user@example.com")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Email("user@example.com"));
    let (cmd, _) = parsed.format_header();
    assert_eq!(cmd, "%EMAIL");
}

#[test]
fn gameplay_player_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("PLAYER", "PLAYER", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Player(PlayerMode::Single));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#PLAYER");
    assert_eq!(val, "1");
}

#[test]
fn gameplay_rank_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("RANK", "RANK", "2")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Rank(2));
}

#[test]
fn gameplay_total_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("TOTAL", "TOTAL", "300")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Total(300.0));
}

#[test]
fn gameplay_lntype_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("LNTYPE", "LNTYPE", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::LnType(LnType::Type1));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#LNTYPE");
    assert_eq!(val, "1");
}

#[test]
fn gameplay_lnmode_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("LNMODE", "LNMODE", "2")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::LnMode(LnMode::Cn));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#LNMODE");
    assert_eq!(val, "2");
}

#[test]
fn gameplay_option_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("OPTION", "OPTION", "-R")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Option("-R"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#OPTION");
    assert_eq!(val, "-R");
}

#[test]
fn display_stagefile_roundtrip() {
    let parsed = BmsHeaderDisplay::try_match_header("STAGEFILE", "STAGEFILE", "stage.png")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderDisplay::StageFile("stage.png"));
}

#[test]
fn display_difficulty_roundtrip() {
    let parsed = BmsHeaderDisplay::try_match_header("DIFFICULTY", "DIFFICULTY", "3")
        .unwrap()
        .unwrap();
    assert_eq!(
        parsed,
        BmsHeaderDisplay::Difficulty("3".parse::<DifficultyLevel>().unwrap())
    );
}

#[test]
fn timing_bpm_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("BPM", "BPM", "180")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderTiming::Bpm(180.0));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#BPM");
    assert_eq!(val, "180");
}

#[test]
fn timing_basebpm_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("BASEBPM", "BASEBPM", "120")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderTiming::BaseBpm(120.0));
}

#[test]
fn timing_bpmdef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("BPM01", "BPM01", "180.0")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    // After format, the BPM value is "180" (not "180.0") due to to_string.
    assert_eq!(cmd, "#BPM01");
    assert_eq!(val, "180");
}

#[test]
fn timing_stopdef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("STOP01", "STOP01", "192")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#STOP01");
    assert_eq!(val, "192");
}

#[test]
fn audio_wav_roundtrip() {
    let parsed = BmsHeaderResDefAudio::try_match_header("WAV01", "WAV01", "kick.wav")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#WAV01");
    assert_eq!(val, "kick.wav");
}

#[test]
fn audio_exwav_roundtrip() {
    let parsed = BmsHeaderResDefAudio::try_match_header("EXWAV01", "EXWAV01", "extra.ogg")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#EXWAV01");
    assert_eq!(val, "extra.ogg");
}

#[test]
fn visual_bmp_roundtrip() {
    let parsed = BmsHeaderResDefVisual::try_match_header("BMP01", "BMP01", "bg.bmp")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#BMP01");
    assert_eq!(val, "bg.bmp");
}

#[test]
fn visual_at_bga_roundtrip() {
    let parsed = BmsHeaderResDefVisual::try_match_header("@BGA01", "@BGA01", "layer.bmp")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#@BGA01");
    assert_eq!(val, "layer.bmp");
}

#[test]
fn visual_seek_roundtrip() {
    let parsed = BmsHeaderResDefVisual::try_match_header("SEEK01", "SEEK01", "1.5")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SEEK01");
    assert_eq!(val, "1.5");
}

#[test]
fn control_random_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("RANDOM", "RANDOM", "10")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Random(10));
}

#[test]
fn control_else_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("ELSE", "ELSE", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Else);
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#ELSE");
    assert_eq!(val, "");
}

#[test]
fn control_endswitch_alias() {
    let parsed = BmsHeaderControlFlow::try_match_header("ENDSW", "ENDSW", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::EndSwitch);
    let parsed2 = BmsHeaderControlFlow::try_match_header("ENDSWITCH", "ENDSWITCH", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed2, BmsHeaderControlFlow::EndSwitch);
}

#[test]
fn control_if_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("IF", "IF", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::If(1));
}

#[test]
fn control_setrandom_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("SETRANDOM", "SETRANDOM", "5")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::SetRandom(5));
}

#[test]
fn control_skip_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("SKIP", "SKIP", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Skip(1));
}

#[test]
fn audio_wavcmd_roundtrip() {
    let parsed = BmsHeaderResDefAudio::try_match_header("WAVCMD", "WAVCMD", "test")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderResDefAudio::WavCmd("test"));
}

#[test]
fn visual_poorbga_roundtrip() {
    let parsed = BmsHeaderResDefVisual::try_match_header("POORBGA", "POORBGA", "fallback.bmp")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderResDefVisual::PoorBga("fallback.bmp"));
}

#[test]
fn visual_extchr_roundtrip() {
    // Command is uppercased by parse_header_line before dispatch.
    let parsed = BmsHeaderResDefVisual::try_match_header("EXTCHR", "EXTCHR", "extra")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderResDefVisual::ExtChr("extra"));
}

#[test]
fn bpm_mixed_indexed_roundtrip() {
    // Verify that #BPM (non-indexed) and #BPM1A (indexed) route correctly.
    let non_idx = BmsHeaderTiming::try_match_header("BPM", "BPM", "120")
        .unwrap()
        .unwrap();
    assert_eq!(non_idx, BmsHeaderTiming::Bpm(120.0));

    let idx = BmsHeaderTiming::try_match_header("BPM1A", "BPM1A", "140.5")
        .unwrap()
        .unwrap();
    assert!(matches!(idx, BmsHeaderTiming::BpmDef { .. }));
    let (cmd, val) = idx.format_header();
    assert_eq!(cmd, "#BPM1A");
    assert_eq!(val, "140.5");
}

#[test]
fn gameplay_volwav_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("VOLWAV", "VOLWAV", "100")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::VolWav(100.0));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#VOLWAV");
    assert_eq!(val, "100");
}

#[test]
fn gameplay_oct_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("OCT", "OCT", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Oct(1.0));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#OCT");
    assert_eq!(val, "1");
}

#[test]
fn gameplay_changeoption_roundtrip() {
    let parsed = BmsHeaderGameplay::try_match_header("CHANGEOPTION", "CHANGEOPTION", "RANDOM")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::ChangeOption("RANDOM"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#CHANGEOPTION");
    assert_eq!(val, "RANDOM");
}

#[test]
fn display_backbmp_roundtrip() {
    let parsed = BmsHeaderDisplay::try_match_header("BACKBMP", "BACKBMP", "bg.png")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderDisplay::BackBmp("bg.png"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#BACKBMP");
    assert_eq!(val, "bg.png");
}

#[test]
fn display_playlevel_roundtrip() {
    let parsed = BmsHeaderDisplay::try_match_header("PLAYLEVEL", "PLAYLEVEL", "12")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderDisplay::PlayLevel(12.0));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#PLAYLEVEL");
    assert_eq!(val, "12");
}

#[test]
fn timing_scrolldef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("SCROLL01", "SCROLL01", "1.5")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SCROLL01");
    assert_eq!(val, "1.5");
}

#[test]
fn timing_speeddef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("SPEED01", "SPEED01", "2.0")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SPEED01");
    assert_eq!(val, "2");
}

#[test]
fn audio_cdda_roundtrip() {
    let parsed = BmsHeaderResDefAudio::try_match_header("CDDA", "CDDA", "track01.bin")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderResDefAudio::Cdda("track01.bin"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#CDDA");
    assert_eq!(val, "track01.bin");
}

#[test]
fn control_switch_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("SWITCH", "SWITCH", "3")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Switch(3));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SWITCH");
    assert_eq!(val, "3");
}

#[test]
fn control_def_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("DEF", "DEF", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Def);
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#DEF");
    assert_eq!(val, "");
}

#[test]
fn unmatched_command_returns_none() {
    let result = BmsHeaderTiming::try_match_header("TITLE", "TITLE", "foo").unwrap();
    assert!(result.is_none());
}

#[test]
fn invalid_value_returns_error() {
    let result = BmsHeaderTiming::try_match_header("BPM", "BPM", "not-a-number");
    assert!(result.is_err());
}

#[test]
fn invalid_player_returns_error() {
    let result = BmsHeaderGameplay::try_match_header("PLAYER", "PLAYER", "xyz");
    assert!(result.is_err());
}

#[test]
fn invalid_wav_index_returns_error() {
    let result = BmsHeaderResDefAudio::try_match_header("WAV!!", "WAV!!", "file.wav");
    assert!(result.is_err());
}
