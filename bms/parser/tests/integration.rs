//! Integration tests for `bms-parser`.
//!
//! These tests consume the crate through its public API only, exactly as
//! an external consumer would.

use bms_parser::*;
use bms_tokenizer::{
    BmpIndex, BmsBase, BmsChannel, BmsTokenizer, BpmIndex, LnMode, LnType, PlayerMode, PoorBgaMode,
    Rank, ScrollIndex, SpeedIndex, StopIndex, WavIndex,
};
use bmsrs_chart::BgaLayer;

/// Helper: parse a BMS string into a `Bms` (default C = &str).
fn parse(input: &str) -> Bms {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, &str>(input)
        .into_iter()
        .filter_map(|(_, res)| res.ok())
        .collect();
    Bms::from_flat_tokens(tokens)
}

/// Helper: parse with `C = String` to verify owned-string pipeline works.
fn parse_string(input: &str) -> Bms {
    let tokens: Vec<_> = BmsTokenizer::new()
        .tokenize::<Vec<_>, String>(input)
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
    let _frac = pos.fraction();

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
    let _: MineEvent = MineEvent {
        position: pos,
        player: 2,
        lane: 7,
        damage: 5.0,
    };
    let _: BpmChange = BpmChange {
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
    let _: MeasureLength = MeasureLength {
        measure: 1,
        length_ratio: 2.0,
    };
    let _: StpEvent = StpEvent {
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
    assert_eq!(bms.gameplay.base, Some(BmsBase::Base36));

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

    let id01: BmpIndex = "01".try_into().unwrap();

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
    let visible_1p = bms
        .messages
        .note_events
        .iter()
        .filter(|n| n.player == 1 && n.key_type == KeyType::Visible)
        .count();
    assert_eq!(visible_1p, 2);

    // 2P visible (ch 21)
    let visible_2p = bms
        .messages
        .note_events
        .iter()
        .filter(|n| n.player == 2 && n.key_type == KeyType::Visible)
        .count();
    assert_eq!(visible_2p, 2);

    // 1P invisible (ch 31)
    let invisible_1p = bms
        .messages
        .note_events
        .iter()
        .filter(|n| n.player == 1 && n.key_type == KeyType::Invisible)
        .count();
    assert_eq!(invisible_1p, 2);
}

#[test]
fn long_note_messages_parsed() {
    let bms = parse("#00151:0102\n#00161:0304");
    assert_eq!(bms.messages.long_note_events.len(), 4);

    // 1P LN
    let ln_1p = bms
        .messages
        .long_note_events
        .iter()
        .filter(|n| n.player == 1)
        .count();
    assert_eq!(ln_1p, 2);

    // 2P LN
    let ln_2p = bms
        .messages
        .long_note_events
        .iter()
        .filter(|n| n.player == 2)
        .count();
    assert_eq!(ln_2p, 2);
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
    let bms = parse("#000SC:ZZ");
    assert_eq!(bms.messages.scroll_events.len(), 1);
    assert_eq!(
        bms.messages.scroll_events[0].scroll_id,
        "ZZ".parse().unwrap()
    );
}

#[test]
fn bga_messages_parsed() {
    let bms = parse("#00104:AA\n#00106:BB\n#00107:CC\n#0010A:DD\n");
    assert_eq!(bms.messages.bga_events.len(), 4);
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
    assert!(
        bms.messages
            .bga_events
            .iter()
            .any(|e| e.layer == BgaLayer::Layer2)
    );
}

#[test]
fn measure_length_parsed() {
    let bms = parse("#00102:2");
    assert_eq!(bms.messages.measure_lengths.len(), 1);
    assert_eq!(bms.messages.measure_lengths[0].measure, 1);
    assert!((bms.messages.measure_lengths[0].length_ratio - 2.0).abs() < f64::EPSILON);
}

#[test]
fn measure_length_fractional() {
    let bms = parse("#00102:0.75");
    assert_eq!(bms.messages.measure_lengths.len(), 1);
    assert!((bms.messages.measure_lengths[0].length_ratio - 0.75).abs() < f64::EPSILON);
}

// Message concatenation

#[test]
fn bgm_multi_line_polyphony() {
    // BGM lines are stored separately (polyphony support).
    let bms = parse("#00101:AABB\n#00101:CCDD");
    let ch = BmsChannel::from_raw("01").unwrap();
    assert_eq!(
        bms.messages.raw.get(&1).and_then(|m| m.get(&ch)),
        Some(&vec!["AABB".to_owned(), "CCDD".to_owned()])
    );
    // Each BGM line is independent: 2+2 = 4 events total.
    assert_eq!(bms.messages.bgm_events.len(), 4);
    // Line 1: events at (0/2, 1/2)
    assert_eq!(bms.messages.bgm_events[0].position.numer, 0);
    assert_eq!(bms.messages.bgm_events[0].position.denom, 2);
    assert_eq!(bms.messages.bgm_events[1].position.numer, 1);
    assert_eq!(bms.messages.bgm_events[1].position.denom, 2);
    // Line 2: events at (0/2, 1/2)
    assert_eq!(bms.messages.bgm_events[2].position.numer, 0);
    assert_eq!(bms.messages.bgm_events[2].position.denom, 2);
    assert_eq!(bms.messages.bgm_events[3].position.numer, 1);
    assert_eq!(bms.messages.bgm_events[3].position.denom, 2);
}

#[test]
fn different_channels_independent() {
    let bms = parse("#00101:AA\n#00111:BB");
    assert_eq!(bms.messages.bgm_events.len(), 1);
    assert_eq!(bms.messages.note_events.len(), 1);
}

// Edge cases

#[test]
fn note_channel_merge_two_lines() {
    // Non-BGM channels are position-merged.
    // Two lines on channel 11: line 1 (2 values), line 2 (2 values).
    // After merge: later line overwrites non-00, 00 preserves.
    let bms = parse("#00111:1100\n#00111:0022");
    // Channel 11 → player 1, key 1, visible. Two events (positions 0 and 1).
    assert_eq!(bms.messages.note_events.len(), 2);
    assert_eq!(bms.messages.note_events[0].wav_id, "11".try_into().unwrap());
    assert_eq!(bms.messages.note_events[1].wav_id, "22".try_into().unwrap());
    assert_eq!(bms.messages.note_events[0].position.numer, 0);
    assert_eq!(bms.messages.note_events[1].position.numer, 1);
}

#[test]
fn note_channel_merge_different_division() {
    // Merge where lines have different subdivisions.
    // Channel 11: line 1 = 4 values, line 2 = 2 values (00 at pos 0, 66 at pos 1).
    // Max count = 4.
    //   Line 1: "AA0000BB" → pos 0=AA, 1=00, 2=00, 3=BB
    //   Line 2: "0066" expanded to 4: pos 0=00(keep), pos 2=66(overwrite)
    //   Merged: "AA0066BB"
    // After "00" filtering: events for AA(pos0/num0), 66(pos2/num2), BB(pos3/num3).
    let bms = parse("#00111:AA0000BB\n#00111:0066");
    assert_eq!(bms.messages.note_events.len(), 3);
    assert_eq!(bms.messages.note_events[0].wav_id, "AA".try_into().unwrap());
    assert_eq!(bms.messages.note_events[1].wav_id, "66".try_into().unwrap());
    assert_eq!(bms.messages.note_events[2].wav_id, "BB".try_into().unwrap());
    assert_eq!(bms.messages.note_events[0].position.numer, 0);
    assert_eq!(bms.messages.note_events[1].position.numer, 2);
    assert_eq!(bms.messages.note_events[2].position.numer, 3);
}

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
    let ch = BmsChannel::from_raw("FF").unwrap();
    assert_eq!(
        bms.messages.raw.get(&1).and_then(|m| m.get(&ch)),
        Some(&vec!["1122".to_owned()])
    );
}

#[test]
fn empty_message_values() {
    let bms = parse("#00111:");
    // Values empty — no events, but raw has an empty-string entry
    assert!(bms.messages.note_events.is_empty());
    let ch = BmsChannel::from_raw("11").unwrap();
    assert_eq!(
        bms.messages.raw.get(&1).and_then(|m| m.get(&ch)),
        Some(&vec![String::new()])
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
    let wav1: WavIndex = "01".parse().unwrap();
    let wav2: WavIndex = "02".parse().unwrap();
    let bpm1: BpmIndex = "01".parse().unwrap();
    let bpm2: BpmIndex = "02".parse().unwrap();
    let stp1: StopIndex = "01".parse().unwrap();

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

/// Verify parser converges correctly with `C = String`.
#[test]
fn parse_with_string_container() {
    let bms = parse_string(
        "\
#TITLE My Song
#ARTIST composer
#WAV01 kick.wav
#00111:1122
#MYEXT abc123
",
    );
    assert_eq!(bms.metadata.title.as_deref(), Some("My Song"));
    assert_eq!(bms.metadata.artist.as_deref(), Some("composer"));
    let wav_id: WavIndex = "01".try_into().unwrap();
    assert_eq!(
        bms.audio.wav_files.get(&wav_id).map(String::as_str),
        Some("kick.wav")
    );
    assert_eq!(bms.fallback_headers.len(), 1);
    assert_eq!(bms.fallback_headers[0], ("MYEXT".into(), "abc123".into()));
}

#[test]
fn base62_wav_indices_case_sensitive() {
    // In Base62 mode, WAVAA and WAVaa are distinct indices.
    let bms = parse("#BASE 62\n#WAVAA kick.wav\n#WAVaa snare.wav\n#WAVaA hat.wav\n");
    // All three should be stored separately.
    assert_eq!(bms.audio.wav_files.len(), 3);
    let aa: WavIndex = "AA".parse().unwrap();
    let a_lower: WavIndex = "aa".parse().unwrap();
    let a_cap_lower: WavIndex = "aA".parse().unwrap();
    assert_eq!(
        bms.audio.wav_files.get(&aa).map(String::as_str),
        Some("kick.wav")
    );
    assert_eq!(
        bms.audio.wav_files.get(&a_lower).map(String::as_str),
        Some("snare.wav")
    );
    assert_eq!(
        bms.audio.wav_files.get(&a_cap_lower).map(String::as_str),
        Some("hat.wav")
    );
}

#[test]
fn base36_wav_indices_case_insensitive() {
    // Standard mode (no #BASE 62): AA and aa map to the same index (last wins).
    let bms = parse("#WAVAA kick.wav\n#WAVaa snare.wav\n");
    assert_eq!(bms.audio.wav_files.len(), 1);
    let idx: WavIndex = "AA".parse().unwrap();
    assert_eq!(
        bms.audio.wav_files.get(&idx).map(String::as_str),
        Some("snare.wav")
    );
}

#[test]
fn bmspec_bpm_basic() {
    // Equivalent to bmspec-1-05-BPM: #BPM 60, #00003:0078 → object at 3s.
    // Parser-level: verify the BPM change is correctly parsed.
    let bms = parse("#BPM 60\n#00003:0078");
    assert_eq!(bms.timing.bpm, Some(60.0));
    assert_eq!(bms.messages.bpm_changes.len(), 1);
    match bms.messages.bpm_changes[0].value {
        bms_parser::BpmValue::Absolute(v) => assert!((v - 120.0).abs() < f64::EPSILON),
        bms_parser::BpmValue::Reference(_) => panic!("expected absolute BPM"),
    }
}

#[test]
fn bmspec_bpm_extended() {
    // Equivalent to bmspec-1-05: #BPM 60, #BPM01 120, #00008:0001 → object at 3s.
    // Verify the BPM def and channel reference.
    let bms = parse("#BPM 60\n#BPM01 120\n#00008:0001");
    assert_eq!(bms.timing.bpm, Some(60.0));
    let bpm1: BpmIndex = "01".parse().unwrap();
    assert_eq!(bms.timing.bpm_defs.get(&bpm1), Some(&120.0));
    // #00008:0001 produces 2 events: "00" (reference to BPM00, typically
    // undefined) and "01" (reference to BPM01=120).
    assert_eq!(bms.messages.bpm_changes.len(), 2);
}

#[test]
fn bmspec_stop_basic() {
    // Equivalent to bmspec-1-06: #BPM 60, #STOP11 96, #00109:0011 → stop events.
    let bms = parse("#BPM 60\n#STOP11 96\n#00111:01000200\n#00109:00110000");
    let stop1: StopIndex = "11".parse().unwrap();
    assert_eq!(bms.timing.stop_defs.get(&stop1), Some(&96.0));
}

#[test]
fn bmspec_scroll_basic() {
    // Equivalent to bmspec-3: #SCROLL02 0.5, #001SC:02 → scroll speed = 0.5.
    let bms = parse("#SCROLL02 0.5\n#001SC:02");
    let idx: ScrollIndex = "02".parse().unwrap();
    assert!((bms.timing.scroll_defs.get(&idx).copied().unwrap_or(0.0) - 0.5).abs() < f64::EPSILON);
    assert_eq!(bms.messages.scroll_events.len(), 1);
}

#[test]
fn bmspec_speed_without_channel() {
    // Equivalent to bmspec-6: #SPEED01 0.5 without #001SP → no speed events.
    let bms = parse("#SPEED01 0.5\n");
    let idx: SpeedIndex = "01".parse().unwrap();
    assert!((bms.timing.speed_defs.get(&idx).copied().unwrap_or(0.0) - 0.5).abs() < f64::EPSILON);
    assert!(bms.messages.speed_events.is_empty());
}

#[test]
fn bmspec_speed_with_channel() {
    // Equivalent to bmspec-6: #SPEED01 0.5, #001SP:0001 → speed event.
    let bms = parse("#SPEED01 0.5\n#001SP:0001");
    assert_eq!(bms.messages.speed_events.len(), 2);
    let idx: SpeedIndex = "01".parse().unwrap();
    assert!((bms.timing.speed_defs.get(&idx).copied().unwrap_or(0.0) - 0.5).abs() < f64::EPSILON);
}

#[test]
fn bmspec_timesig_positioning() {
    // Equivalent to bmspec-1-04: #00102:0.750, #00111:0104 → object positions.
    let bms = parse("#00102:0.750\n#00111:0104");
    assert_eq!(bms.messages.measure_lengths.len(), 1);
    assert!((bms.messages.measure_lengths[0].length_ratio - 0.75).abs() < f64::EPSILON);
    assert_eq!(bms.messages.measure_lengths[0].measure, 1);
    // Verify notes parsed: 2 events (01 and 04).
    assert_eq!(bms.messages.note_events.len(), 2);
}

// Tests adapted from `lib.rs` `#[cfg(test)] mod tests` (public API only).
// The 4 `merge_channel` tests that use `crate::messages::merge_channel` are
// left inline in the source file and not duplicated here.

#[test]
fn header_override_last_wins() {
    let bms = parse("#TITLE First\n#TITLE Second");
    assert_eq!(bms.metadata.title.as_deref(), Some("Second"));
}

#[test]
fn wav_definitions_stored() {
    let bms = parse("#WAV01 a.wav\n#WAV02 b.wav");
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
    let bms = parse("#00101:1122");
    let ch = BmsChannel::from_raw("01").unwrap();
    let measure_map = bms.messages.raw.get(&1);
    assert!(measure_map.is_some());
    assert_eq!(
        measure_map
            .and_then(|m| m.get(&ch))
            .and_then(|v| v.first().map(String::as_str)),
        Some("1122")
    );
    assert_eq!(measure_map.and_then(|m| m.get(&ch).map(Vec::len)), Some(1));
}

#[test]
fn message_concat_same_channel() {
    let bms = parse("#00101:1122\n#00101:3344");
    let ch = BmsChannel::from_raw("01").unwrap();
    let measure_map = bms.messages.raw.get(&1);
    let lines = measure_map.and_then(|m| m.get(&ch));
    assert_eq!(lines, Some(&vec!["1122".to_owned(), "3344".to_owned()]));
    assert_eq!(bms.messages.bgm_events.len(), 4);
    assert_eq!(bms.messages.bgm_events[0].position.denom, 2);
    assert_eq!(bms.messages.bgm_events[2].position.denom, 2);
}

#[test]
fn fallback_headers_stored() {
    let bms = parse("#MYEXT abc123");
    assert_eq!(bms.fallback_headers.len(), 1);
    assert_eq!(
        bms.fallback_headers[0],
        ("MYEXT".to_owned(), "abc123".to_owned())
    );
}

#[test]
fn lib_mixed_headers_and_messages() {
    let bms = parse("#TITLE My Song\n#ARTIST composer\n#BPM 180\n#WAV01 kick.wav\n#00111:11223344");
    assert_eq!(bms.metadata.title.as_deref(), Some("My Song"));
    assert_eq!(bms.metadata.artist.as_deref(), Some("composer"));
    assert_eq!(bms.timing.bpm, Some(180.0));
    let wav_id: WavIndex = "01".try_into().unwrap();
    assert_eq!(
        bms.audio.wav_files.get(&wav_id).map(String::as_str),
        Some("kick.wav")
    );
    let ch = BmsChannel::from_raw("11").unwrap();
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
    let bms = parse("#BPM 120\n#BPM01 180.0\n#EXBPM02 200.0");
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
fn oct_fp_stored() {
    let bms = parse("#OCT 1");
    assert_eq!(bms.gameplay.oct_fp, Some(true));
}

#[test]
fn option_stored() {
    let bms = parse("#OPTION -R");
    assert_eq!(bms.gameplay.option.as_deref(), Some("-R"));
}

#[test]
fn wavcmd_stored() {
    let bms = parse("#WAVCMD some-command");
    assert_eq!(bms.audio.wav_cmd.as_deref(), Some("some-command"));
}

#[test]
fn cdda_stored() {
    let bms = parse("#CDDA track01.bin");
    assert_eq!(bms.audio.cdda.as_deref(), Some("track01.bin"));
}

#[test]
fn midifile_stored() {
    let bms = parse("#MIDIFILE song.mid");
    assert_eq!(bms.audio.midifile.as_deref(), Some("song.mid"));
}

#[test]
fn ext_chr_stored() {
    let bms = parse("#ExtChr extra");
    assert_eq!(bms.visual.ext_chr.as_deref(), Some("extra"));
}

#[test]
fn poor_bga_stored() {
    let bms = parse("#POORBGA 0");
    assert_eq!(bms.visual.poor_bga_mode, Some(PoorBgaMode::Default));
}

#[test]
fn ex_bmp_stored() {
    let bms = parse("#EXBMP01 255,0,128,64 overlay.png");
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
    let bms = parse("#BGA01 02 0 0 100 100 10 20");
    let id: BmpIndex = "01".try_into().unwrap();
    assert!(bms.visual.crop_defs.contains_key(&id));
}

#[test]
fn at_bga_stored() {
    let bms = parse("#@BGA01 03 5 10 200 150 0 0");
    let id: BmpIndex = "01".try_into().unwrap();
    assert!(bms.visual.alt_crop_defs.contains_key(&id));
}

#[test]
fn sw_bga_stored() {
    let bms = parse("#SWBGA01 30:60:1:0:255,0,0,128 pattern.bmp");
    let id: BmpIndex = "01".try_into().unwrap();
    assert!(bms.visual.sw_bga_defs.contains_key(&id));
}

#[test]
fn argb_stored() {
    let bms = parse("#ARGB01 128,255,0,64");
    let id: BmpIndex = "01".try_into().unwrap();
    assert!(bms.visual.argb_defs.contains_key(&id));
}

#[test]
fn stp_event_parsed_from_header() {
    let bms = parse("#STP 001.128 500");
    assert_eq!(bms.messages.stp_events.len(), 1);
    let ev = &bms.messages.stp_events[0];
    assert_eq!(ev.position.measure, 1);
    assert_eq!(ev.position.numer, 128);
    assert_eq!(ev.position.denom, 1000);
    assert!((ev.duration_ms - 500.0).abs() < f64::EPSILON);
}

#[test]
fn video_fps_stored() {
    let bms = parse("#VIDEOf/s 30");
    assert_eq!(bms.visual.video_fps, Some(30.0));
}

#[test]
fn video_colors_stored() {
    let bms = parse("#VIDEOCOLORS 16");
    assert_eq!(bms.visual.video_colors, Some(16.0));
}

#[test]
fn video_dly_stored() {
    let bms = parse("#VIDEODLY 1.5");
    assert_eq!(bms.visual.video_dly, Some(1.5));
}
