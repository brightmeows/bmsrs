//! 错误路径测试。

#![expect(clippy::unwrap_used, reason = "test code")]

mod helper;

use osu_processor::{OsuError, OsuProcessError, OsuProcessor};

fn mutate(from: &str, to: &str) -> String {
    helper::MANIA_4K.replace(from, to)
}

fn parse_error(content: &str) -> OsuError {
    OsuProcessor::from_str(content).unwrap_err()
}

#[test]
fn standard_mode_is_rejected() {
    let error = parse_error(&mutate("Mode: 3", "Mode: 0"));
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::UnsupportedMode { .. })
    ));
}

#[test]
fn fractional_key_count_is_rejected() {
    let error = parse_error(&mutate("CircleSize: 4", "CircleSize: 4.5"));
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::InvalidKeyCount { .. })
    ));
}

#[test]
fn missing_timing_point_is_rejected() {
    let content = "osu file format v14\n\
                   [General]\n\
                   AudioFilename: audio.ogg\n\
                   Mode: 3\n\
                   [Difficulty]\n\
                   CircleSize: 4\n\
                   [HitObjects]\n\
                   64,192,1000,1,0,0:0:0:0:\n";
    let error = parse_error(content);
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::MissingTimingPoint)
    ));
}

#[test]
fn missing_audio_is_rejected() {
    let error = parse_error(&mutate("AudioFilename: audio.ogg", "AudioFilename:"));
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::MissingAudioFilename)
    ));
}

#[test]
fn negative_hit_object_is_rejected() {
    let error = parse_error(&mutate("64,192,1000,1,0", "64,192,-1,1,0"));
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::NegativeHitObjectTime { .. })
    ));
}

#[test]
fn zero_length_hold_is_rejected() {
    let error = parse_error(&mutate(
        "192,192,1500,128,0,2000",
        "192,192,1500,128,0,1500",
    ));
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::InvalidHold { .. })
    ));
}

#[test]
fn slider_is_rejected() {
    let error = parse_error(&mutate(
        "64,192,1000,1,0,0:0:0:0:",
        "64,192,1000,2,0,L|128:192,1,100,0:0:0:0:",
    ));
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::UnsupportedObject { kind: "slider", .. })
    ));
}

#[test]
fn spinner_is_rejected() {
    let error = parse_error(&mutate(
        "64,192,1000,1,0,0:0:0:0:",
        "64,192,1000,8,0,1500,0:0:0:0:",
    ));
    assert!(matches!(
        error,
        OsuError::Process(OsuProcessError::UnsupportedObject {
            kind: "spinner",
            ..
        })
    ));
}

#[test]
fn time_overflow_is_rejected() {
    let mut map: rosu_map::Beatmap = rosu_map::from_str(helper::MANIA_4K).unwrap();
    let Some(note) = map.hit_objects.first_mut() else {
        panic!("fixture must contain a hit object");
    };
    note.start_time = 1.0e30;
    let error = OsuProcessor::process(&map).unwrap_err();
    assert!(matches!(error, OsuProcessError::TimeOverflow { .. }));
}
