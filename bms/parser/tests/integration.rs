//! Integration tests for `bms-parser`.
//!
//! These tests consume the crate through its public API only, exactly as
//! an external consumer would.

use bms_parser::*;
use bms_tokenizer::{
    Base62, BmpTag, BmsBaseMode, BmsIndex, BmsTokenizer, BpmTag, ChannelTag, LnMode, LnType,
    PlayerMode, Rank, StopTag, WavTag,
};

/// Helper: parse a BMS string into a `Bms`.
fn parse(input: &str) -> Bms {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>>(input)
        .into_iter()
        .filter_map(|(_, res)| res.ok())
        .collect();
    Bms::from_flat_tokens(tokens)
}

// Re-export accessibility

/// Verify that all key types are accessible via the public re-exports.
#[test]
fn event_types_accessible() {
    // Position
    let pos = Position::new(1, 0, 4);
    let _ = pos.fraction();

    // Event types are constructible
    let _bgm = BgmEvent {
        position: pos,
        wav_id: "01".parse().unwrap(),
    };
    let _note = NoteEvent {
        position: pos,
        player: 1,
        lane: 3,
        key_type: KeyType::Visible,
        wav_id: "AA".parse().unwrap(),
    };
    let _ln = LongNoteEvent {
        position: pos,
        player: 1,
        lane: 5,
        wav_id: "ZZ".parse().unwrap(),
    };
    let _mine = MineEvent {
        position: pos,
        player: 2,
        lane: 7,
    };
    let _bpm = BpmChange {
        position: pos,
        value: BpmValue::Absolute(180.0),
    };
    let _stop = StopEvent {
        position: pos,
        stop_id: "01".parse().unwrap(),
    };
    let _scroll = ScrollEvent {
        position: pos,
        scroll_id: "AA".parse().unwrap(),
    };
    let _bga = BgaEvent {
        position: pos,
        layer: BgaLayer::Base,
        bmp_id: "03".parse().unwrap(),
    };
    let _ml = MeasureLength {
        measure: 1,
        length_percent: 200,
    };
    let _stp = StpEvent {
        position: pos,
        duration_ms: 500.0,
    };
}

/// Sub-struct types are accessible.
#[test]
fn sub_struct_types_accessible() {
    let _meta = Metadata::default();
    let _gameplay = Gameplay::default();
    let _display = Display::default();
    let _timing = Timing::default();
    let _audio = Audio::default();
    let _visual = Visual::default();
    let _msgs = Messages::default();
}

/// Owned param types are accessible.
#[test]
fn owned_param_types_accessible() {
    let _ex = OwnedExBmpParams {
        a: 255,
        r: 0,
        g: 128,
        b: 64,
        filename: "overlay.png".into(),
    };
    let _sw = OwnedSwBgaParams {
        fr: 30,
        time: 60,
        line: 1,
        r#loop: false,
        a: 255,
        r: 0,
        g: 0,
        b: 128,
        pattern: "pattern.bmp".into(),
    };
}

// Full BMS header parsing

/// Parse a realistic BMS header section covering all major categories.
#[test]
fn full_header_parse() {
    let bms = parse(
        "\
#TITLE My Song
#SUBTITLE (short ver.)
#ARTIST composer
#SUBARTIST co-writer
#GENRE Piano
#MAKER creator
#COMMENT hello
#CHARSET UTF-8
%URL https://example.com
%EMAIL user@example.com
#PLAYER 1
#RANK 2
#DEFEXRANK 3
#TOTAL 300
#VOLWAV 100
#LNTYPE 1
#LNOBJ 01
#LNMODE 1
#BASE 36
#BPM 180
#BASEBPM 180
#STAGEFILE stage.png
#BANNER banner.bmp
#BACKBMP bg.png
#CHARFILE char.bmp
#PLAYLEVEL 12
#DIFFICULTY 3
#PREVIEW preview.ogg
",
    );

    // Metadata
    assert_eq!(bms.metadata.title.as_deref(), Some("My Song"));
    assert_eq!(bms.metadata.subtitle.as_deref(), Some("(short ver.)"));
    assert_eq!(bms.metadata.artist.as_deref(), Some("composer"));
    assert_eq!(bms.metadata.sub_artist.as_deref(), Some("co-writer"));
    assert_eq!(bms.metadata.genre.as_deref(), Some("Piano"));
    assert_eq!(bms.metadata.maker.as_deref(), Some("creator"));
    assert_eq!(bms.metadata.comment.as_deref(), Some("hello"));
    assert_eq!(bms.metadata.charset.as_deref(), Some("UTF-8"));
    assert_eq!(bms.metadata.url.as_deref(), Some("https://example.com"));
    assert_eq!(bms.metadata.email.as_deref(), Some("user@example.com"));

    // Gameplay
    assert_eq!(bms.gameplay.player, Some(PlayerMode::Single));
    assert_eq!(bms.gameplay.rank, Some(Rank::Normal));
    assert_eq!(bms.gameplay.def_ex_rank, Some(3.0));
    assert_eq!(bms.gameplay.total, Some(300.0));
    assert_eq!(bms.gameplay.vol_wav, Some(100.0));
    assert_eq!(bms.gameplay.ln_type, Some(LnType::Type1));
    assert!(bms.gameplay.ln_obj.is_some());
    assert_eq!(bms.gameplay.ln_mode, Some(LnMode::Ln));
    assert_eq!(bms.gameplay.base, Some(BmsBaseMode::Base36));

    // Timing
    assert_eq!(bms.timing.bpm, Some(180.0));
    assert_eq!(bms.timing.base_bpm, Some(180.0));

    // Display
    assert_eq!(bms.display.stage_file.as_deref(), Some("stage.png"));
    assert_eq!(bms.display.banner.as_deref(), Some("banner.bmp"));
    assert_eq!(bms.display.back_bmp.as_deref(), Some("bg.png"));
    assert_eq!(bms.display.char_file.as_deref(), Some("char.bmp"));
    assert_eq!(bms.display.play_level, Some(12.0));
    assert_eq!(bms.display.difficulty, Some("3".parse().unwrap()));
    assert_eq!(bms.display.preview.as_deref(), Some("preview.ogg"));

    // No messages, no fallback
    assert!(bms.messages.raw.is_empty());
    assert!(bms.fallback_headers.is_empty());
}

// Previously-dropped header storage

#[test]
fn dropped_audio_headers_stored() {
    let bms = parse("#WAVCMD some-cmd\n#CDDA track.bin\n#MIDIFILE song.mid\n#PATH_WAV ./sounds/");
    assert_eq!(bms.audio.wav_cmd.as_deref(), Some("some-cmd"));
    assert_eq!(bms.audio.cdda.as_deref(), Some("track.bin"));
    assert_eq!(bms.audio.midifile.as_deref(), Some("song.mid"));
    assert_eq!(bms.audio.path_wav.as_deref(), Some("./sounds/"));
}

#[test]
fn dropped_gameplay_headers_stored() {
    let bms = parse("#OCT 1\n#OPTION -R\n");
    assert_eq!(bms.gameplay.oct_fp, Some(true));
    assert_eq!(bms.gameplay.option.as_deref(), Some("-R"));
}

#[test]
fn dropped_visual_headers_stored() {
    let bms = parse(
        "\
#BMP01 bg.bmp
#EXBMP01 255,0,128,64 overlay.png
#BGA01 02 0 0 100 100 10 20
#@BGA01 03 5 10 200 150 0 0
#SWBGA01 30:60:1:0:255,0,0,128 pattern.bmp
#ARGB01 128,255,0,64
#ExtChr extra
#VIDEOf/s 30
#VIDEOCOLORS 16
#VIDEODLY 1.5
",
    );

    let id01: BmsIndex<BmpTag> = "01".try_into().unwrap();

    // BMP is stored
    assert_eq!(
        bms.visual.bmp_files.get(&id01).map(String::as_str),
        Some("bg.bmp")
    );

    // EXBMP — owned params
    let ex = bms.visual.ex_bmp_defs.get(&id01);
    assert!(ex.is_some());
    assert_eq!(ex.unwrap().filename, "overlay.png");

    // BGA / @BGA / SWBGA / ARGB
    assert!(bms.visual.crop_defs.contains_key(&id01));
    assert!(bms.visual.alt_crop_defs.contains_key(&id01));
    assert!(bms.visual.sw_bga_defs.contains_key(&id01));
    assert!(bms.visual.argb_defs.contains_key(&id01));

    // ExtChr
    assert_eq!(bms.visual.ext_chr.as_deref(), Some("extra"));

    // Video
    assert_eq!(bms.visual.video_fps, Some(30.0));
    assert_eq!(bms.visual.video_colors, Some(16.0));
    assert_eq!(bms.visual.video_dly, Some(1.5));
}

#[test]
fn stp_header_stored_as_event() {
    let bms = parse("#STP 001.128 500");
    assert_eq!(bms.messages.stp_events.len(), 1);
    let ev = &bms.messages.stp_events[0];
    assert_eq!(ev.position.measure, 1);
    assert_eq!(ev.position.numer, 128);
    assert_eq!(ev.position.denom, 1000);
    assert!((ev.duration_ms - 500.0).abs() < f64::EPSILON);
}

// Message channel parsing

#[test]
fn bgm_messages_parsed() {
    let bms = parse("#00101:AABBCC");
    assert_eq!(bms.messages.bgm_events.len(), 3);
    assert_eq!(bms.messages.bgm_events[0].wav_id, "AA".parse().unwrap());
    assert_eq!(bms.messages.bgm_events[1].wav_id, "BB".parse().unwrap());
    assert_eq!(bms.messages.bgm_events[2].wav_id, "CC".parse().unwrap());
    // Positions
    assert_eq!(bms.messages.bgm_events[0].position.numer, 0);
    assert_eq!(bms.messages.bgm_events[0].position.denom, 3);
    assert_eq!(bms.messages.bgm_events[2].position.numer, 2);
}

#[test]
fn note_messages_parsed() {
    let bms = parse("#00111:1122\n#00121:3344\n#00131:5566");
    // 1P visible (ch 11)
    assert_eq!(bms.messages.note_events.len(), 6);
    let p1v: Vec<_> = bms
        .messages
        .note_events
        .iter()
        .filter(|n| n.player == 1 && n.key_type == KeyType::Visible)
        .collect();
    assert_eq!(p1v.len(), 2);

    // 2P visible (ch 21)
    let p2v: Vec<_> = bms
        .messages
        .note_events
        .iter()
        .filter(|n| n.player == 2 && n.key_type == KeyType::Visible)
        .collect();
    assert_eq!(p2v.len(), 2);

    // 1P invisible (ch 31)
    let p1i: Vec<_> = bms
        .messages
        .note_events
        .iter()
        .filter(|n| n.player == 1 && n.key_type == KeyType::Invisible)
        .collect();
    assert_eq!(p1i.len(), 2);
}

#[test]
fn long_note_messages_parsed() {
    let bms = parse("#00151:0102\n#00161:0304");
    assert_eq!(bms.messages.long_note_events.len(), 4);

    // 1P LN
    let p1: Vec<_> = bms
        .messages
        .long_note_events
        .iter()
        .filter(|n| n.player == 1)
        .collect();
    assert_eq!(p1.len(), 2);

    // 2P LN
    let p2: Vec<_> = bms
        .messages
        .long_note_events
        .iter()
        .filter(|n| n.player == 2)
        .collect();
    assert_eq!(p2.len(), 2);
}

#[test]
fn mine_messages_parsed() {
    let bms = parse("#001D1:01\n#001E1:02");
    assert_eq!(bms.messages.mine_events.len(), 2);
    assert_eq!(bms.messages.mine_events[0].player, 1);
    assert_eq!(bms.messages.mine_events[0].lane, 1);
    assert_eq!(bms.messages.mine_events[1].player, 2);
    assert_eq!(bms.messages.mine_events[1].lane, 1);
}

#[test]
fn timing_messages_parsed() {
    let bms = parse("#00103:7F\n#00108:05\n");
    // BPM absolute
    assert_eq!(bms.messages.bpm_changes.len(), 2);
    let abs_bpm = &bms.messages.bpm_changes[0];
    assert!(matches!(abs_bpm.value, BpmValue::Absolute(v) if (v - 127.0).abs() < f64::EPSILON));
    // BPM reference
    let ref_bpm = &bms.messages.bpm_changes[1];
    assert!(matches!(&ref_bpm.value, BpmValue::Reference(id) if id.as_str() == "05"));
}

#[test]
fn stop_messages_parsed() {
    let bms = parse("#00109:01");
    assert_eq!(bms.messages.stop_events.len(), 1);
    assert_eq!(bms.messages.stop_events[0].stop_id, "01".parse().unwrap());
}

#[test]
fn scroll_messages_parsed() {
    let bms = parse("#0010A:ZZ");
    assert_eq!(bms.messages.scroll_events.len(), 1);
    assert_eq!(
        bms.messages.scroll_events[0].scroll_id,
        "ZZ".parse().unwrap()
    );
}

#[test]
fn bga_messages_parsed() {
    let bms = parse("#00104:AA\n#00106:BB\n#00107:CC\n");
    assert_eq!(bms.messages.bga_events.len(), 3);
    assert!(
        bms.messages
            .bga_events
            .iter()
            .any(|e| e.layer == BgaLayer::Base)
    );
    assert!(
        bms.messages
            .bga_events
            .iter()
            .any(|e| e.layer == BgaLayer::Poor)
    );
    assert!(
        bms.messages
            .bga_events
            .iter()
            .any(|e| e.layer == BgaLayer::Layer)
    );
}

#[test]
fn measure_length_parsed() {
    let bms = parse("#00102:200");
    assert_eq!(bms.messages.measure_lengths.len(), 1);
    assert_eq!(bms.messages.measure_lengths[0].measure, 1);
    assert_eq!(bms.messages.measure_lengths[0].length_percent, 200);
}

// Message concatenation

#[test]
fn same_channel_concat() {
    let bms = parse("#00101:AABB\n#00101:CCDD");
    let ch: BmsIndex<ChannelTag, Base62> = "01".try_into().unwrap();
    assert_eq!(
        bms.messages.raw.get(&1).and_then(|m| m.get(&ch)),
        Some(&"AABBCCDD".to_owned())
    );
    // Events are parsed from per-line values with correct offsets:
    // First line AABB → events at (0/2, 1/2), second line CCDD → (2/4, 3/4)
    assert_eq!(bms.messages.bgm_events.len(), 4);
    // First line: known only 2 objects at this point
    assert_eq!(bms.messages.bgm_events[0].position.numer, 0);
    assert_eq!(bms.messages.bgm_events[0].position.denom, 2);
    assert_eq!(bms.messages.bgm_events[1].position.numer, 1);
    assert_eq!(bms.messages.bgm_events[1].position.denom, 2);
    // Second line: knows total is 4 (concatenated), offset is 2
    assert_eq!(bms.messages.bgm_events[2].position.numer, 2);
    assert_eq!(bms.messages.bgm_events[2].position.denom, 4);
    assert_eq!(bms.messages.bgm_events[3].position.numer, 3);
    assert_eq!(bms.messages.bgm_events[3].position.denom, 4);
}

#[test]
fn different_channels_independent() {
    let bms = parse("#00101:AA\n#00111:BB");
    assert_eq!(bms.messages.bgm_events.len(), 1);
    assert_eq!(bms.messages.note_events.len(), 1);
}

// Edge cases

#[test]
fn empty_input() {
    let bms = parse("");
    // All defaults — no panics
    assert!(bms.metadata.title.is_none());
    assert!(bms.gameplay.player.is_none());
    assert!(bms.timing.bpm.is_none());
    assert!(bms.display.stage_file.is_none());
    assert!(bms.audio.wav_files.is_empty());
    assert!(bms.visual.bmp_files.is_empty());
    assert!(bms.messages.raw.is_empty());
    assert!(bms.fallback_headers.is_empty());
}

#[test]
fn unknown_header_fallback() {
    let bms = parse("#MYEXT abc123\n#UNKNOWN value");
    assert_eq!(bms.fallback_headers.len(), 2);
    assert_eq!(bms.fallback_headers[0], ("MYEXT".into(), "abc123".into()));
    assert_eq!(bms.fallback_headers[1], ("UNKNOWN".into(), "value".into()));
}

#[test]
fn unknown_channel_stays_raw() {
    let bms = parse("#001FF:1122");
    // Channel FF is unknown — only in raw, no events
    assert!(bms.messages.bgm_events.is_empty());
    assert!(bms.messages.note_events.is_empty());
    let ch: BmsIndex<ChannelTag, Base62> = "FF".try_into().unwrap();
    assert_eq!(
        bms.messages.raw.get(&1).and_then(|m| m.get(&ch)),
        Some(&"1122".to_owned())
    );
}

#[test]
fn empty_message_values() {
    let bms = parse("#00111:");
    // Values empty — no events, but raw has empty string
    assert!(bms.messages.note_events.is_empty());
    let ch: BmsIndex<ChannelTag, Base62> = "11".try_into().unwrap();
    assert_eq!(
        bms.messages.raw.get(&1).and_then(|m| m.get(&ch)),
        Some(&"".to_owned())
    );
}

#[test]
fn headers_without_values() {
    // Headers with no value should not cause errors
    let bms = parse("#TITLE\n#ARTIST\n");
    assert_eq!(bms.metadata.title.as_deref(), Some(""));
    assert_eq!(bms.metadata.artist.as_deref(), Some(""));
}

// Interleaved headers and messages

#[test]
fn mixed_headers_and_messages() {
    let bms = parse(
        "\
#TITLE My Song
#BPM 180
#WAV01 kick.wav
#00111:11223344
#WAV02 snare.wav
#PLAYER 2
#00121:AABB
",
    );

    assert_eq!(bms.metadata.title.as_deref(), Some("My Song"));
    assert_eq!(bms.timing.bpm, Some(180.0));
    assert_eq!(bms.audio.wav_files.len(), 2);
    assert_eq!(bms.gameplay.player, Some(PlayerMode::Couple));
    assert_eq!(bms.messages.note_events.len(), 6); // 4 from ch11 + 2 from ch21
}

// Resource definitions integration

#[test]
fn wav_and_bpm_defs() {
    let bms = parse("#WAV01 a.wav\n#WAV02 b.wav\n#BPM01 180.0\n#EXBPM02 200.0\n#STOP01 192\n");
    let wav1: BmsIndex<WavTag> = "01".parse().unwrap();
    let wav2: BmsIndex<WavTag> = "02".parse().unwrap();
    let bpm1: BmsIndex<BpmTag> = "01".parse().unwrap();
    let bpm2: BmsIndex<BpmTag> = "02".parse().unwrap();
    let stp1: BmsIndex<StopTag> = "01".parse().unwrap();

    assert_eq!(
        bms.audio.wav_files.get(&wav1).map(String::as_str),
        Some("a.wav")
    );
    assert_eq!(
        bms.audio.wav_files.get(&wav2).map(String::as_str),
        Some("b.wav")
    );
    assert_eq!(bms.timing.bpm_defs.get(&bpm1), Some(&180.0));
    assert_eq!(bms.timing.bpm_defs.get(&bpm2), Some(&200.0));
    assert_eq!(bms.timing.stop_defs.get(&stp1), Some(&192.0));
}

// Bms default

#[test]
fn default_bms_is_empty() {
    let bms = Bms::default();
    assert!(bms.metadata.title.is_none());
    assert!(bms.audio.wav_files.is_empty());
    assert!(bms.messages.raw.is_empty());
    assert!(bms.fallback_headers.is_empty());
}
