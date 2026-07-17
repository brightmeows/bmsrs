//! `#[derive(BmsToken)]` 往返转换的集成测试。
//!
//! 每个测试验证 `try_match_header` → `format_header` → 再次解析
//! 是恒等的（或至少是一致的）。

use bms_tokenizer::{
    BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderGameplay, BmsHeaderMetadata,
    BmsHeaderResDefAudio, BmsHeaderResDefVisual, BmsHeaderTiming, ChangeOptionIndex,
    DifficultyLevel, LnMode, LnType, PlayerMode, Rank, TextIndex, WavCmdKind, WavCmdParams,
};

#[test]
fn metadata_title_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("TITLE", "My Song")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Title("My Song"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#TITLE");
    assert_eq!(val, "My Song");
}

#[test]
fn metadata_subtitle_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("SUBTITLE", "(short ver.)")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Subtitle("(short ver.)"));
}

#[test]
fn metadata_artist_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("ARTIST", "composer")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Artist("composer"));
}

#[test]
fn metadata_subartist_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("SUBARTIST", "co-writer")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::SubArtist("co-writer"));
}

#[test]
fn metadata_genre_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("GENRE", "Piano")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Genre("Piano"));
}

#[test]
fn metadata_maker_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("MAKER", "creator")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Maker("creator"));
}

#[test]
fn metadata_comment_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("COMMENT", "hello")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Comment("hello"));
}

#[test]
fn metadata_text_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("TEXT01", "in-game text")
        .unwrap()
        .unwrap();
    assert_eq!(
        parsed,
        BmsHeaderMetadata::Text {
            id: TextIndex::try_from("01").unwrap(),
            value: "in-game text"
        }
    );
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#TEXT01");
    assert_eq!(val, "in-game text");
}

#[test]
fn metadata_song_alias() {
    let parsed = BmsHeaderMetadata::try_match_header("SONG01", "some text")
        .unwrap()
        .unwrap();
    assert_eq!(
        parsed,
        BmsHeaderMetadata::Text {
            id: TextIndex::try_from("01").unwrap(),
            value: "some text"
        }
    );
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#TEXT01"); // SONG 是别名；规范形式为 TEXT
    assert_eq!(val, "some text");
}

#[test]
fn metadata_charset_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("CHARSET", "UTF-8")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Charset("UTF-8"));
}

#[test]
fn metadata_url_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("URL", "https://example.com")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Url("https://example.com"));
    let (cmd, _) = parsed.format_header();
    assert_eq!(cmd, "%URL");
}

#[test]
fn metadata_email_roundtrip() {
    let parsed = BmsHeaderMetadata::try_match_header("EMAIL", "user@example.com")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderMetadata::Email("user@example.com"));
    let (cmd, _) = parsed.format_header();
    assert_eq!(cmd, "%EMAIL");
}

#[test]
fn gameplay_player_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("PLAYER", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Player(PlayerMode::Single));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#PLAYER");
    assert_eq!(val, "1");
}

#[test]
fn gameplay_rank_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("RANK", "2")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Rank(Rank::Normal));
}

#[test]
fn gameplay_total_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("TOTAL", "300")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Total(300.0));
}

#[test]
fn gameplay_lntype_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("LNTYPE", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::LnType(LnType::Type1));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#LNTYPE");
    assert_eq!(val, "1");
}

#[test]
fn gameplay_lnmode_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("LNMODE", "2")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::LnMode(LnMode::Cn));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#LNMODE");
    assert_eq!(val, "2");
}

#[test]
fn gameplay_option_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("OPTION", "-R")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::Option("-R"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#OPTION");
    assert_eq!(val, "-R");
}

#[test]
fn display_stagefile_roundtrip() {
    let parsed = BmsHeaderDisplay::<&str>::try_match_header("STAGEFILE", "stage.png")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderDisplay::StageFile("stage.png"));
}

#[test]
fn display_difficulty_roundtrip() {
    let parsed = BmsHeaderDisplay::<&str>::try_match_header("DIFFICULTY", "3")
        .unwrap()
        .unwrap();
    assert_eq!(
        parsed,
        BmsHeaderDisplay::Difficulty("3".parse::<DifficultyLevel>().unwrap())
    );
}

#[test]
fn timing_bpm_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("BPM", "180")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderTiming::Bpm(180.0));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#BPM");
    assert_eq!(val, "180");
}

#[test]
fn timing_basebpm_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("BASEBPM", "120")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderTiming::BaseBpm(120.0));
}

#[test]
fn timing_bpmdef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("BPM01", "180.0")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    // 格式化后 BPM 值为 "180"（而非 "180.0"），这是 to_string 的结果。
    assert_eq!(cmd, "#BPM01");
    assert_eq!(val, "180");
}

#[test]
fn timing_stopdef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("STOP01", "192")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#STOP01");
    assert_eq!(val, "192");
}

#[test]
fn audio_wav_roundtrip() {
    let parsed = BmsHeaderResDefAudio::<&str>::try_match_header("WAV01", "kick.wav")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#WAV01");
    assert_eq!(val, "kick.wav");
}

#[test]
fn audio_exwav_roundtrip() {
    let parsed = BmsHeaderResDefAudio::<&str>::try_match_header("EXWAV01", "extra.ogg")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#EXWAV01");
    assert_eq!(val, "extra.ogg");
}

#[test]
fn visual_bmp_roundtrip() {
    let parsed = BmsHeaderResDefVisual::<&str>::try_match_header("BMP01", "bg.bmp")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#BMP01");
    assert_eq!(val, "bg.bmp");
}

#[test]
fn visual_at_bga_roundtrip() {
    let parsed = BmsHeaderResDefVisual::<&str>::try_match_header("@BGA01", "3 5 10 200 150 0 0")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#@BGA01");
    assert_eq!(val, "3 5 10 200 150 0 0");
}

#[test]
fn visual_seek_roundtrip() {
    let parsed = BmsHeaderResDefVisual::<&str>::try_match_header("SEEK01", "1.5")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SEEK01");
    assert_eq!(val, "1.5");
}

#[test]
fn control_random_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("RANDOM", "10")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Random(10));
}

#[test]
fn control_else_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("ELSE", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Else);
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#ELSE");
    assert_eq!(val, "");
}

#[test]
fn control_endswitch_alias() {
    let parsed = BmsHeaderControlFlow::try_match_header("ENDSW", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::EndSwitch);
    let parsed2 = BmsHeaderControlFlow::try_match_header("ENDSWITCH", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed2, BmsHeaderControlFlow::EndSwitch);
}

#[test]
fn control_if_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("IF", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::If(1));
}

#[test]
fn control_setrandom_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("SETRANDOM", "5")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::SetRandom(5));
}

#[test]
fn control_skip_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("SKIP", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Skip);
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SKIP");
    assert_eq!(val, "");
}

#[test]
fn audio_wavcmd_roundtrip() {
    let parsed = BmsHeaderResDefAudio::<&str>::try_match_header("WAVCMD", "00 01 100")
        .unwrap()
        .unwrap();
    assert_eq!(
        parsed,
        BmsHeaderResDefAudio::WavCmd {
            params: WavCmdParams {
                command: WavCmdKind::Pitch,
                wav_index: "01".parse().unwrap(),
                value: 100,
            }
        }
    );
}

#[test]
fn visual_poorbga_roundtrip() {
    let parsed = BmsHeaderResDefVisual::<&str>::try_match_header("POORBGA", "0")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#POORBGA");
    assert_eq!(val, "0");
}

#[test]
fn visual_extchr_roundtrip() {
    // 命令在分发前由 parse_header_line 转为大写。
    let parsed = BmsHeaderResDefVisual::<&str>::try_match_header("EXTCHR", "extra")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderResDefVisual::ExtChr("extra"));
}

#[test]
fn bpm_mixed_indexed_roundtrip() {
    // 验证 #BPM（非索引）与 #BPM1A（索引）正确路由。
    let non_idx = BmsHeaderTiming::try_match_header("BPM", "120")
        .unwrap()
        .unwrap();
    assert_eq!(non_idx, BmsHeaderTiming::Bpm(120.0));

    let idx = BmsHeaderTiming::try_match_header("BPM1A", "140.5")
        .unwrap()
        .unwrap();
    assert!(matches!(idx, BmsHeaderTiming::BpmDef { .. }));
    let (cmd, val) = idx.format_header();
    assert_eq!(cmd, "#BPM1A");
    assert_eq!(val, "140.5");
}

#[test]
fn gameplay_volwav_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("VOLWAV", "100")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::VolWav(100.0));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#VOLWAV");
    assert_eq!(val, "100");
}

#[test]
fn gameplay_oct_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("OCT", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::OctFp);
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#OCT/FP");
    assert_eq!(val, "");
}

#[test]
fn gameplay_fp_roundtrip() {
    let parsed = BmsHeaderGameplay::<&str>::try_match_header("FP", "1")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderGameplay::OctFp);
}

#[test]
fn gameplay_changeoption_roundtrip() {
    let parsed =
        BmsHeaderGameplay::<&str>::try_match_header("CHANGEOPTION01", "774:HIDDEN_STEALTH")
            .unwrap()
            .unwrap();
    assert_eq!(
        parsed,
        BmsHeaderGameplay::ChangeOption {
            id: ChangeOptionIndex::try_from("01").unwrap(),
            value: "774:HIDDEN_STEALTH"
        }
    );
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#CHANGEOPTION01");
    assert_eq!(val, "774:HIDDEN_STEALTH");
}

#[test]
fn display_backbmp_roundtrip() {
    let parsed = BmsHeaderDisplay::<&str>::try_match_header("BACKBMP", "bg.png")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderDisplay::BackBmp("bg.png"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#BACKBMP");
    assert_eq!(val, "bg.png");
}

#[test]
fn display_playlevel_roundtrip() {
    let parsed = BmsHeaderDisplay::<&str>::try_match_header("PLAYLEVEL", "12")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderDisplay::PlayLevel(12.0));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#PLAYLEVEL");
    assert_eq!(val, "12");
}

#[test]
fn timing_scrolldef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("SCROLL01", "1.5")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SCROLL01");
    assert_eq!(val, "1.5");
}

#[test]
fn timing_speeddef_roundtrip() {
    let parsed = BmsHeaderTiming::try_match_header("SPEED01", "2.0")
        .unwrap()
        .unwrap();
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SPEED01");
    assert_eq!(val, "2");
}

#[test]
fn audio_cdda_roundtrip() {
    let parsed = BmsHeaderResDefAudio::<&str>::try_match_header("CDDA", "track01.bin")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderResDefAudio::Cdda("track01.bin"));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#CDDA");
    assert_eq!(val, "track01.bin");
}

#[test]
fn control_switch_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("SWITCH", "3")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Switch(3));
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#SWITCH");
    assert_eq!(val, "3");
}

#[test]
fn control_def_roundtrip() {
    let parsed = BmsHeaderControlFlow::try_match_header("DEF", "")
        .unwrap()
        .unwrap();
    assert_eq!(parsed, BmsHeaderControlFlow::Def);
    let (cmd, val) = parsed.format_header();
    assert_eq!(cmd, "#DEF");
    assert_eq!(val, "");
}

#[test]
fn unmatched_command_returns_none() {
    let result = BmsHeaderTiming::try_match_header("TITLE", "foo").unwrap();
    assert!(result.is_none());
}

#[test]
fn invalid_value_returns_error() {
    let result = BmsHeaderTiming::try_match_header("BPM", "not-a-number");
    assert!(result.is_err());
}

#[test]
fn invalid_player_returns_error() {
    let result = BmsHeaderGameplay::<&str>::try_match_header("PLAYER", "xyz");
    assert!(result.is_err());
}

#[test]
fn invalid_wav_index_returns_error() {
    let result = BmsHeaderResDefAudio::<&str>::try_match_header("WAV!!", "file.wav");
    assert!(result.is_err());
}
