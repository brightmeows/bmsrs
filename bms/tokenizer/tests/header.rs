//! 通过 [`parse_header_line`] 解析 [`BmsHeader`] 的集成测试。

use bms_tokenizer::{
    BmpIndex, BmsBase, BmsHeader, BmsHeaderControlFlow, BmsHeaderDisplay, BmsHeaderFallback,
    BmsHeaderGameplay, BmsHeaderMetadata, BmsHeaderResDefAudio, BmsHeaderResDefVisual,
    BmsHeaderTiming, BpmIndex, ChangeOptionIndex, DifficultyLevel, ExRankIndex, LnMode, LnObjIndex,
    LnType, PlayerMode, PoorBgaMode, Rank, ScrollIndex, SeekIndex, SpeedIndex, StopIndex,
    StpParams, TextIndex, WavCmdKind, WavCmdParams, WavIndex,
};

#[test]
fn parse_title() {
    let result = bms_tokenizer::parse_header_line("#TITLE My Song", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Title("My Song".to_owned()))
    );
}

#[test]
fn parse_title_empty() {
    let result = bms_tokenizer::parse_header_line("#TITLE", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Title(String::new()))
    );
}

#[test]
fn parse_subtitle() {
    let result = bms_tokenizer::parse_header_line("#SUBTITLE (short ver.)", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Subtitle("(short ver.)".to_owned()))
    );
}

#[test]
fn parse_artist() {
    let result = bms_tokenizer::parse_header_line("#ARTIST composer", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Artist("composer".to_owned()))
    );
}

#[test]
fn parse_subartist() {
    let result = bms_tokenizer::parse_header_line("#SUBARTIST co-writer", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::SubArtist("co-writer".to_owned()))
    );
}

#[test]
fn parse_genre() {
    let result = bms_tokenizer::parse_header_line("#GENRE Piano", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Genre("Piano".to_owned()))
    );
}

#[test]
fn parse_maker() {
    let result = bms_tokenizer::parse_header_line("#MAKER chart-creator", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Maker("chart-creator".to_owned()))
    );
}

#[test]
fn parse_comment() {
    let result = bms_tokenizer::parse_header_line("#COMMENT hello", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Comment("hello".to_owned()))
    );
}

#[test]
fn parse_text() {
    let result = bms_tokenizer::parse_header_line("#TEXT01 in-game text", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Text {
            id: TextIndex::try_from("01").unwrap(),
            value: "in-game text".to_owned()
        })
    );
}

#[test]
fn parse_text_with_quotes() {
    let result = bms_tokenizer::parse_header_line("#TEXT00 \"MISS!!\"", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Text {
            id: TextIndex::try_from("00").unwrap(),
            value: "\"MISS!!\"".to_owned()
        })
    );
}

#[test]
fn parse_song_as_text() {
    let result = bms_tokenizer::parse_header_line("#SONG01 some text", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Text {
            id: TextIndex::try_from("01").unwrap(),
            value: "some text".to_owned()
        })
    );
}

#[test]
fn parse_charset() {
    let result = bms_tokenizer::parse_header_line("#CHARSET UTF-8", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Charset("UTF-8".to_owned()))
    );
}

#[test]
fn parse_url() {
    let result = bms_tokenizer::parse_header_line("%URL https://example.com", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Url("https://example.com".to_owned()))
    );
}

#[test]
fn parse_email() {
    let result = bms_tokenizer::parse_header_line("%EMAIL user@example.com", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Email("user@example.com".to_owned()))
    );
}

#[test]
fn parse_player() {
    let result = bms_tokenizer::parse_header_line("#PLAYER 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::Player(PlayerMode::Single))
    );
}

#[test]
fn parse_rank() {
    let result = bms_tokenizer::parse_header_line("#RANK 2", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::Rank(Rank::Normal))
    );
}

#[test]
fn parse_defexrank() {
    let result = bms_tokenizer::parse_header_line("#DEFEXRANK 3", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::DefExRank(3.0))
    );
}

#[test]
fn parse_total() {
    let result = bms_tokenizer::parse_header_line("#TOTAL 300", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::Total(300.0)));
}

#[test]
fn parse_volwav() {
    let result = bms_tokenizer::parse_header_line("#VOLWAV 100", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::VolWav(100.0))
    );
}

#[test]
fn parse_lntype() {
    let result = bms_tokenizer::parse_header_line("#LNTYPE 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::LnType(LnType::Type1))
    );
}

#[test]
fn parse_lnobj() {
    let result = bms_tokenizer::parse_header_line("#LNOBJ 01", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::LnObj(
            LnObjIndex::try_from("01").unwrap()
        ))
    );
}

#[test]
fn parse_lnmode() {
    let result = bms_tokenizer::parse_header_line("#LNMODE 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::LnMode(LnMode::Ln))
    );
}

#[test]
fn parse_oct() {
    let result = bms_tokenizer::parse_header_line("#OCT 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
}

#[test]
fn parse_fp() {
    let result = bms_tokenizer::parse_header_line("#FP 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
}

#[test]
fn parse_octfp() {
    let result = bms_tokenizer::parse_header_line("#OCT/FP 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::Gameplay(BmsHeaderGameplay::OctFp));
}

#[test]
fn parse_option() {
    let result = bms_tokenizer::parse_header_line("#OPTION -R", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::Option("-R".to_owned()))
    );
}

#[test]
fn parse_changeoption() {
    let result =
        bms_tokenizer::parse_header_line("#CHANGEOPTION01 774:HIDDEN_STEALTH", &['#', '%'])
            .unwrap()
            .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::ChangeOption {
            id: ChangeOptionIndex::try_from("01").unwrap(),
            value: "774:HIDDEN_STEALTH".to_owned()
        })
    );
}

#[test]
fn parse_stagefile() {
    let result = bms_tokenizer::parse_header_line("#STAGEFILE stage.png", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Display(BmsHeaderDisplay::StageFile("stage.png".to_owned()))
    );
}

#[test]
fn parse_banner() {
    let result = bms_tokenizer::parse_header_line("#BANNER banner.bmp", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Display(BmsHeaderDisplay::Banner("banner.bmp".to_owned()))
    );
}

#[test]
fn parse_backbmp() {
    let result = bms_tokenizer::parse_header_line("#BACKBMP bg.png", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Display(BmsHeaderDisplay::BackBmp("bg.png".to_owned()))
    );
}

#[test]
fn parse_charfile() {
    let result = bms_tokenizer::parse_header_line("#CHARFILE char.bmp", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Display(BmsHeaderDisplay::CharFile("char.bmp".to_owned()))
    );
}

#[test]
fn parse_playlevel() {
    let result = bms_tokenizer::parse_header_line("#PLAYLEVEL 12", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Display(BmsHeaderDisplay::PlayLevel(12.0))
    );
}

#[test]
fn parse_difficulty() {
    let result = bms_tokenizer::parse_header_line("#DIFFICULTY 3", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Display(BmsHeaderDisplay::Difficulty(
            "3".parse::<DifficultyLevel>().unwrap()
        ))
    );
}

#[test]
fn parse_preview() {
    let result = bms_tokenizer::parse_header_line("#PREVIEW preview.ogg", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Display(BmsHeaderDisplay::Preview("preview.ogg".to_owned()))
    );
}

#[test]
fn parse_bpm_global() {
    let result = bms_tokenizer::parse_header_line("#BPM 180", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm(180.0)));
}

#[test]
fn parse_bpm_global_float() {
    let result = bms_tokenizer::parse_header_line("#BPM 180.0", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::Bpm(180.0)));
}

#[test]
fn parse_basebpm() {
    let result = bms_tokenizer::parse_header_line("#BASEBPM 180", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::Timing(BmsHeaderTiming::BaseBpm(180.0)));
}

#[test]
fn parse_wavcmd() {
    let result = bms_tokenizer::parse_header_line("#WAVCMD 01 05 100", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd {
            params: WavCmdParams {
                command: WavCmdKind::Volume,
                wav_index: "05".parse().unwrap(),
                value: 100,
            }
        })
    );
}

#[test]
fn parse_cdda() {
    let result = bms_tokenizer::parse_header_line("#CDDA track01.bin", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Cdda("track01.bin".to_owned()))
    );
}

#[test]
fn parse_midifile() {
    let result = bms_tokenizer::parse_header_line("#MIDIFILE song.mid", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Midifile("song.mid".to_owned()))
    );
}

#[test]
fn parse_path_wav() {
    let result = bms_tokenizer::parse_header_line("#PATH_WAV ./sounds/", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::PathWav("./sounds/".to_owned()))
    );
}

#[test]
fn parse_poorbga() {
    let result = bms_tokenizer::parse_header_line("#POORBGA 0", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Default))
    );
}

#[test]
fn parse_poorbga_overlay() {
    let result = bms_tokenizer::parse_header_line("#POORBGA 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Overlay))
    );
}

#[test]
fn parse_poorbga_hidden() {
    let result = bms_tokenizer::parse_header_line("#POORBGA 2", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::PoorBga(PoorBgaMode::Hidden))
    );
}

#[test]
fn parse_poorbga_invalid_fallback() {
    let result = bms_tokenizer::parse_header_line("#POORBGA 3", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(result, BmsHeader::Fallback(_)));
}

#[test]
fn parse_videofile() {
    let result = bms_tokenizer::parse_header_line("#VIDEOFILE bg.avi", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::VideoFile("bg.avi".to_owned()))
    );
}

#[test]
fn parse_movie() {
    let result = bms_tokenizer::parse_header_line("#MOVIE intro.mpg", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Movie("intro.mpg".to_owned()))
    );
}

#[test]
fn parse_extchr() {
    let result = bms_tokenizer::parse_header_line("#ExtChr extra", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::ExtChr("extra".to_owned()))
    );
}

#[test]
fn parse_random() {
    let result = bms_tokenizer::parse_header_line("#RANDOM 10", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(10))
    );
}

#[test]
fn parse_setrandom() {
    let result = bms_tokenizer::parse_header_line("#SETRANDOM 5", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::SetRandom(5))
    );
}

#[test]
fn parse_if() {
    let result = bms_tokenizer::parse_header_line("#IF 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::If(1)));
}

#[test]
fn parse_elseif() {
    let result = bms_tokenizer::parse_header_line("#ELSEIF 0", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::ElseIf(0))
    );
}

#[test]
fn parse_else() {
    let result = bms_tokenizer::parse_header_line("#ELSE", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Else));
}

#[test]
fn parse_endif() {
    let result = bms_tokenizer::parse_header_line("#ENDIF", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::EndIf));
}

#[test]
fn parse_endrandom() {
    let result = bms_tokenizer::parse_header_line("#ENDRANDOM", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::EndRandom)
    );
}

#[test]
fn parse_switch() {
    let result = bms_tokenizer::parse_header_line("#SWITCH 3", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::Switch(3))
    );
}

#[test]
fn parse_setswitch() {
    let result = bms_tokenizer::parse_header_line("#SETSWITCH 2", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::SetSwitch(2))
    );
}

#[test]
fn parse_case() {
    let result = bms_tokenizer::parse_header_line("#CASE 1", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::Case(1))
    );
}

#[test]
fn parse_skip() {
    let result = bms_tokenizer::parse_header_line("#SKIP", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Skip));
}

#[test]
fn parse_def() {
    let result = bms_tokenizer::parse_header_line("#DEF", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(result, BmsHeader::ControlFlow(BmsHeaderControlFlow::Def));
}

#[test]
fn parse_endsw() {
    let result = bms_tokenizer::parse_header_line("#ENDSW", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
    );
}

#[test]
fn parse_endswitch_alias() {
    let result = bms_tokenizer::parse_header_line("#ENDSWITCH", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::EndSwitch)
    );
}

#[test]
fn parse_wav_indexed() {
    let result = bms_tokenizer::parse_header_line("#WAV01 kick.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
            id: WavIndex::try_from("01").unwrap(),
            filename: "kick.wav".to_owned()
        })
    );
}

#[test]
fn parse_wav_36ary_index() {
    let result = bms_tokenizer::parse_header_line("#WAV2A snare.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
            id: WavIndex::try_from("2A").unwrap(),
            filename: "snare.wav".to_owned()
        })
    );
}

#[test]
fn parse_exwav_indexed() {
    let result = bms_tokenizer::parse_header_line("#EXWAV01 extra.ogg", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { id, params }) = result {
        assert_eq!(id.as_str(), "01");
        assert_eq!(params.filename, "extra.ogg");
        assert!(params.pan.is_none());
        assert!(params.volume.is_none());
        assert!(params.frequency.is_none());
    } else {
        panic!("expected ExWav variant");
    }
}

#[test]
fn parse_exwav_with_flags() {
    let result =
        bms_tokenizer::parse_header_line("#EXWAV01 pvf -100 -50 440 sound.wav", &['#', '%'])
            .unwrap()
            .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { id, params }) = result {
        assert_eq!(id.as_str(), "01");
        assert_eq!(params.pan, Some(-100));
        assert_eq!(params.volume, Some(-50));
        assert_eq!(params.frequency, Some(440));
        assert_eq!(params.filename, "sound.wav");
    } else {
        panic!("expected ExWav variant");
    }
}

#[test]
fn parse_bmp_indexed() {
    let result = bms_tokenizer::parse_header_line("#BMP01 bg.bmp", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bmp {
            id: BmpIndex::try_from("01").unwrap(),
            filename: "bg.bmp".to_owned()
        })
    );
}

#[test]
fn parse_exbmp_indexed() {
    let result = bms_tokenizer::parse_header_line("#EXBMP01 255,0,128,64 overlay.png", &['#', '%'])
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
fn parse_bga_single_digit_coordinates_succeeds() {
    let result = bms_tokenizer::parse_header_line("#BGA01 1 0 0 9 8 1 2", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga { params, .. }) = result {
        assert_eq!(params.bmp_index, 1);
        assert_eq!(params.x2, 9);
        assert_eq!(params.y2, 8);
    } else {
        panic!("expected Bga variant");
    }
}

#[test]
fn parse_bga_negative_coordinates_succeeds() {
    let result = bms_tokenizer::parse_header_line("#BGA01 02 -10 -20 100 200 5 15", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga { params, .. }) = result {
        assert_eq!(params.x1, -10);
        assert_eq!(params.y1, -20);
    } else {
        panic!("expected Bga variant");
    }
}

#[test]
fn parse_bga_zero_width_succeeds() {
    let result = bms_tokenizer::parse_header_line("#BGA01 02 50 50 50 100 0 0", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga { params, .. }) = result {
        assert_eq!(params.x1, 50);
        assert_eq!(params.x2, 50);
        assert_eq!(params.dx, 0);
        assert_eq!(params.dy, 0);
    } else {
        panic!("expected Bga variant");
    }
}

#[test]
fn parse_bga_large_coordinates_succeeds() {
    let result = bms_tokenizer::parse_header_line("#BGA01 02 0 0 4095 8191 999 -999", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Bga { params, .. }) = result {
        assert_eq!(params.x2, 4095);
        assert_eq!(params.y2, 8191);
        assert_eq!(params.dx, 999);
        assert_eq!(params.dy, -999);
    } else {
        panic!("expected Bga variant");
    }
}

#[test]
fn parse_bga_indexed() {
    let result = bms_tokenizer::parse_header_line("#BGA01 02 0 0 100 100 10 20", &['#', '%'])
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
fn parse_at_bga_zero_width_height_succeeds() {
    let result = bms_tokenizer::parse_header_line("#@BGA01 03 5 10 0 0 0 0", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::AtBga { params, .. }) = result {
        assert_eq!(params.sx, 5);
        assert_eq!(params.sy, 10);
        assert_eq!(params.w, 0);
        assert_eq!(params.h, 0);
    } else {
        panic!("expected AtBga variant");
    }
}

#[test]
fn parse_at_bga_negative_dx_dy_succeeds() {
    let result = bms_tokenizer::parse_header_line("#@BGA01 03 5 10 200 150 -5 -10", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::AtBga { params, .. }) = result {
        assert_eq!(params.dx, -5);
        assert_eq!(params.dy, -10);
    } else {
        panic!("expected AtBga variant");
    }
}

#[test]
fn parse_at_bga_large_dimensions() {
    let result = bms_tokenizer::parse_header_line("#@BGA01 03 5 10 4095 8191 0 0", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefVisual(BmsHeaderResDefVisual::AtBga { params, .. }) = result {
        assert_eq!(params.sx, 5);
        assert_eq!(params.sy, 10);
        assert_eq!(params.w, 4095);
        assert_eq!(params.h, 8191);
    } else {
        panic!("expected AtBga variant");
    }
}

#[test]
fn parse_at_bga_indexed() {
    let result = bms_tokenizer::parse_header_line("#@BGA01 03 5 10 200 150 0 0", &['#', '%'])
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
    let result =
        bms_tokenizer::parse_header_line("#SWBGA01 30:60:1:0:255,0,0,128 pattern.bmp", &['#', '%'])
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
    let result = bms_tokenizer::parse_header_line("#ARGB01 128,255,0,64", &['#', '%'])
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
    let result = bms_tokenizer::parse_header_line("#BGA01 bad", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(result, BmsHeader::Fallback(_)));
}

#[test]
fn parse_argb_invalid_fallback() {
    let result = bms_tokenizer::parse_header_line("#ARGB01 bad", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(result, BmsHeader::Fallback(_)));
}

#[test]
fn parse_seek_indexed() {
    let result = bms_tokenizer::parse_header_line("#SEEK01 1.5", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefVisual(BmsHeaderResDefVisual::Seek {
            id: SeekIndex::try_from("01").unwrap(),
            value: 1.5
        })
    );
}

#[test]
fn parse_bpm_def_indexed() {
    let result = bms_tokenizer::parse_header_line("#BPM01 180.0", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Timing(BmsHeaderTiming::BpmDef {
            id: BpmIndex::try_from("01").unwrap(),
            value: 180.0
        })
    );
}

#[test]
fn parse_stop_indexed() {
    let result = bms_tokenizer::parse_header_line("#STOP01 192", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Timing(BmsHeaderTiming::StopDef {
            id: StopIndex::try_from("01").unwrap(),
            value: 192.0
        })
    );
}

#[test]
fn parse_scroll_indexed() {
    let result = bms_tokenizer::parse_header_line("#SCROLL01 1.5", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Timing(BmsHeaderTiming::ScrollDef {
            id: ScrollIndex::try_from("01").unwrap(),
            value: 1.5
        })
    );
}

#[test]
fn parse_speed_indexed() {
    let result = bms_tokenizer::parse_header_line("#SPEED01 2.0", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Timing(BmsHeaderTiming::SpeedDef {
            id: SpeedIndex::try_from("01").unwrap(),
            value: 2.0
        })
    );
}

#[test]
fn parse_exrank_indexed() {
    let result = bms_tokenizer::parse_header_line("#EXRANK01 5", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::ExRank {
            id: ExRankIndex::try_from("01").unwrap(),
            value: 5.0
        })
    );
}

#[test]
fn wavcmd_not_confused_as_wav_indexed() {
    let result = bms_tokenizer::parse_header_line("#WAVCMD 00 01 100", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd {
            params: WavCmdParams {
                command: WavCmdKind::Pitch,
                wav_index: "01".parse().unwrap(),
                value: 100,
            }
        })
    );
}

#[test]
fn parse_unknown_header() {
    let result = bms_tokenizer::parse_header_line("#MYEXT abc123", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Fallback(BmsHeaderFallback {
            command: "MYEXT".to_owned(),
            value: "abc123".to_owned(),
        })
    );
}

#[test]
fn parse_unknown_percent_header() {
    let result = bms_tokenizer::parse_header_line("%CUSTOM x", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Fallback(BmsHeaderFallback {
            command: "CUSTOM".to_owned(),
            value: "x".to_owned(),
        })
    );
}

#[test]
fn empty_line_returns_none() {
    assert_eq!(
        bms_tokenizer::parse_header_line("", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn whitespace_only_returns_none() {
    assert_eq!(
        bms_tokenizer::parse_header_line("   ", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn comment_line_returns_none() {
    assert_eq!(
        bms_tokenizer::parse_header_line("// comment", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn just_hash_returns_none() {
    assert_eq!(
        bms_tokenizer::parse_header_line("#", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn just_percent_returns_none() {
    assert_eq!(
        bms_tokenizer::parse_header_line("%", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn case_insensitive_title() {
    let result = bms_tokenizer::parse_header_line("#title lowercase", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Title("lowercase".to_owned()))
    );
}

#[test]
fn case_insensitive_wav() {
    let result = bms_tokenizer::parse_header_line("#wav01 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::Wav {
            id: WavIndex::try_from("01").unwrap(),
            filename: "sound.wav".to_owned()
        })
    );
}

#[test]
fn value_with_multiple_spaces() {
    let result = bms_tokenizer::parse_header_line("#TITLE   My   Song", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Title("My   Song".to_owned()))
    );
}

#[test]
fn non_header_line() {
    assert_eq!(
        bms_tokenizer::parse_header_line("just some text", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn channel_line_not_confused() {
    assert_eq!(
        bms_tokenizer::parse_header_line("#00111:1122", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn hash_comment_line() {
    assert_eq!(
        bms_tokenizer::parse_header_line("## just a comment", &['#', '%']).unwrap(),
        None
    );
}

#[test]
fn invalid_player_returns_error() {
    assert!(bms_tokenizer::parse_header_line("#PLAYER xyz", &['#', '%']).is_err());
}

#[test]
fn invalid_difficulty_returns_error() {
    assert!(bms_tokenizer::parse_header_line("#DIFFICULTY 0", &['#', '%']).is_err());
}

#[test]
fn invalid_rank_returns_error() {
    assert!(bms_tokenizer::parse_header_line("#RANK abc", &['#', '%']).is_err());
}

#[test]
fn invalid_wav_index_returns_error() {
    assert!(bms_tokenizer::parse_header_line("#WAV!! file.wav", &['#', '%']).is_err());
}

#[test]
fn invalid_bpm_returns_error() {
    assert!(bms_tokenizer::parse_header_line("#BPM notanumber", &['#', '%']).is_err());
}

#[test]
fn parse_exbpm_indexed() {
    let result = bms_tokenizer::parse_header_line("#EXBPM01 180.0", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Timing(BmsHeaderTiming::ExBpm {
            id: BpmIndex::try_from("01").unwrap(),
            value: 180.0
        })
    );
}

#[test]
fn parse_stp_with_position() {
    let result = bms_tokenizer::parse_header_line("#STP 001.128 500", &['#', '%'])
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
    let result = bms_tokenizer::parse_header_line("#STP 001 500.5", &['#', '%'])
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
    let result = bms_tokenizer::parse_header_line("#STP invalid", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(result, BmsHeader::Fallback(_)));
}

#[test]
fn parse_stp_measure_999_accepted() {
    let result = bms_tokenizer::parse_header_line("#STP 999 500", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Timing(BmsHeaderTiming::Stp {
            params: StpParams {
                measure: 999,
                position: 0,
                duration_ms: 500.0
            }
        })
    );
}

#[test]
fn parse_stp_measure_over_999_fallback() {
    let result = bms_tokenizer::parse_header_line("#STP 1000 500", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(result, BmsHeader::Fallback(_)));
}

#[test]
fn parse_stp_measure_0_accepted() {
    let result = bms_tokenizer::parse_header_line("#STP 000 500", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Timing(BmsHeaderTiming::Stp {
            params: StpParams {
                measure: 0,
                position: 0,
                duration_ms: 500.0
            }
        })
    );
}

#[test]
fn parse_genre_alias() {
    let result = bms_tokenizer::parse_header_line("#GENLE Pop", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Metadata(BmsHeaderMetadata::Genre("Pop".to_owned()))
    );
}

#[test]
fn parse_random_alias() {
    let result = bms_tokenizer::parse_header_line("#RONDAM 5", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(5))
    );
}

#[test]
fn parse_base_16() {
    let result = bms_tokenizer::parse_header_line("#BASE 16", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::Base(BmsBase::Base16))
    );
}

#[test]
fn parse_base_36() {
    let result = bms_tokenizer::parse_header_line("#BASE 36", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::Base(BmsBase::Base36))
    );
}

#[test]
fn parse_base_62() {
    let result = bms_tokenizer::parse_header_line("#BASE 62", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::Gameplay(BmsHeaderGameplay::Base(BmsBase::Base62))
    );
}

#[test]
fn parse_base_unknown_fallback() {
    let result = bms_tokenizer::parse_header_line("#BASE 99", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(result, BmsHeader::Fallback(_)));
}

// #EXWAV 参数范围校验

#[test]
fn parse_exwav_pan_at_bounds() {
    let r = bms_tokenizer::parse_header_line("#EXWAV01 p 10000 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { params, .. }) = r {
        assert_eq!(params.pan, Some(10000));
    } else {
        panic!("expected ExWav variant");
    }

    let r2 = bms_tokenizer::parse_header_line("#EXWAV01 p -10000 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { params, .. }) = r2 {
        assert_eq!(params.pan, Some(-10000));
    } else {
        panic!("expected ExWav variant");
    }
}

#[test]
fn parse_exwav_pan_out_of_range_fallback() {
    let r = bms_tokenizer::parse_header_line("#EXWAV01 p 10001 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(r, BmsHeader::Fallback(_)));

    let r2 = bms_tokenizer::parse_header_line("#EXWAV01 p -10001 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(r2, BmsHeader::Fallback(_)));
}

#[test]
fn parse_exwav_volume_at_bounds() {
    let r = bms_tokenizer::parse_header_line("#EXWAV01 v 0 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { params, .. }) = r {
        assert_eq!(params.volume, Some(0));
    } else {
        panic!("expected ExWav variant");
    }

    let r2 = bms_tokenizer::parse_header_line("#EXWAV01 v -10000 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { params, .. }) = r2 {
        assert_eq!(params.volume, Some(-10000));
    } else {
        panic!("expected ExWav variant");
    }
}
#[test]
fn parse_exwav_volume_positive_fallback() {
    let r = bms_tokenizer::parse_header_line("#EXWAV01 v 1 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(r, BmsHeader::Fallback(_)));
}
#[test]
fn parse_exwav_frequency_at_bounds() {
    let r = bms_tokenizer::parse_header_line("#EXWAV01 f 100 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { params, .. }) = r {
        assert_eq!(params.frequency, Some(100));
    } else {
        panic!("expected ExWav variant");
    }

    let r2 = bms_tokenizer::parse_header_line("#EXWAV01 f 100000 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { params, .. }) = r2 {
        assert_eq!(params.frequency, Some(100_000));
    } else {
        panic!("expected ExWav variant");
    }
}
#[test]
fn parse_exwav_frequency_out_of_range_fallback() {
    let r = bms_tokenizer::parse_header_line("#EXWAV01 f 99 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(r, BmsHeader::Fallback(_)));

    let r2 = bms_tokenizer::parse_header_line("#EXWAV01 f 100001 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(r2, BmsHeader::Fallback(_)));
}
#[test]
fn parse_exwav_multi_flag_pan_out_of_range_fallback() {
    let r = bms_tokenizer::parse_header_line("#EXWAV01 pf 10000 440 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    if let BmsHeader::ResDefAudio(BmsHeaderResDefAudio::ExWav { params, .. }) = r {
        assert_eq!(params.pan, Some(10000));
        assert_eq!(params.frequency, Some(440));
    } else {
        panic!("expected ExWav variant");
    }

    // pan 越界导致整个 EXWAV 回退
    let r2 = bms_tokenizer::parse_header_line("#EXWAV01 pf 10001 440 sound.wav", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(r2, BmsHeader::Fallback(_)));
}

// #WAVCMD pitch 范围校验

#[test]
fn parse_wavcmd_pitch_0_accepted() {
    let result = bms_tokenizer::parse_header_line("#WAVCMD 00 01 0", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd {
            params: WavCmdParams {
                command: WavCmdKind::Pitch,
                wav_index: "01".parse().unwrap(),
                value: 0,
            }
        })
    );
}

#[test]
fn parse_wavcmd_pitch_127_accepted() {
    let result = bms_tokenizer::parse_header_line("#WAVCMD 00 01 127", &['#', '%'])
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd {
            params: WavCmdParams {
                command: WavCmdKind::Pitch,
                wav_index: "01".parse().unwrap(),
                value: 127,
            }
        })
    );
}

#[test]
fn parse_wavcmd_pitch_128_fallback() {
    let result = bms_tokenizer::parse_header_line("#WAVCMD 00 01 128", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(result, BmsHeader::Fallback(_)));
}

#[test]
fn parse_wavcmd_pitch_0_boundary_still_allows_volume_over_100() {
    // 音量命令（01）不受 pitch 范围限制
    let result = bms_tokenizer::parse_header_line("#WAVCMD 01 01 200", &['#', '%'])
        .unwrap()
        .unwrap();
    assert!(matches!(
        result,
        BmsHeader::ResDefAudio(BmsHeaderResDefAudio::WavCmd { .. })
    ));
}
