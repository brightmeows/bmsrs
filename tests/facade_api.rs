//! Public facade integration tests.

#[test]
fn bms_parse_and_process_return_outcomes() {
    let parsed = bmsrs::bms::parse("#TITLE A\n#BPM 120\n#TOTAL 100\n#WAV01 a.wav\n#00111:0100\n")
        .expect("valid BMS");
    assert_eq!(parsed.value.metadata.title.as_deref(), Some("A"));

    let processed = bmsrs::bms::process(&parsed.value).expect("valid chart");
    assert_ne!(processed.value.chart.data().events().len(), 0);
    assert_eq!(processed.warnings.len(), 0);

    let text = bmsrs::bms::write(&parsed.value);
    assert!(text.contains("#TITLE A"));
}

#[test]
fn bms_parse_file_reads_chart_from_path() {
    let path = std::env::temp_dir().join(format!("bmsrs-facade-test-{}.bms", std::process::id()));
    std::fs::write(&path, "#TITLE A\n#BPM 120\n#WAV01 a.wav\n#00111:0100\n")
        .expect("write fixture");
    let parsed = bmsrs::bms::parse_file(&path).expect("valid BMS file");
    std::fs::remove_file(&path).expect("remove fixture");
    assert_eq!(parsed.value.metadata.title.as_deref(), Some("A"));
}

#[test]
fn java_random_seed_reproduces_branch_layout_and_replays() {
    let text =
        "#RANDOM 5\n#IF 1\n#TITLE ONE\n#ENDIF\n#IF 5\n#TITLE FIVE\n#ENDIF\n#ENDRANDOM\n#BPM 120\n";
    let mut rng = bmsrs::bms::JavaRandom::new(123_456_789);
    let (recorded, selection) = bmsrs::bms::parse_with_recording(
        text,
        &bmsrs::bms::ParseOptions::default(),
        |max| rng.choice(max),
        bmsrs::bms::ErrorStrategy::CollectAll,
    )
    .expect("valid BMS");
    let title_from_seed = recorded.value.metadata.title;
    assert!(
        title_from_seed.as_deref() == Some("ONE") || title_from_seed.as_deref() == Some("FIVE"),
        "a branch was selected: {title_from_seed:?}"
    );

    let replayed = bmsrs::bms::parse_from_selection(text, &selection).expect("replay ok");
    assert_eq!(replayed.value.metadata.title, title_from_seed);

    let drifted = text.replace("#RANDOM 5", "#RANDOM 4");
    let error = bmsrs::bms::parse_from_selection(&drifted, &selection);
    assert!(matches!(
        error,
        Err(bmsrs::bms::ReplayParseError::Branch(_))
    ));
}

#[cfg(feature = "bmson")]
#[test]
fn bmson_parse_and_process_return_outcomes() {
    let json = r#"{
        "version": "2.0.0",
        "song_info": {"title": "T", "artist": "A", "genre": "G"},
        "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
        "chart_data": {"init_bpm": 120}
    }"#;
    let bmson = bmsrs::bmson::parse_str(json).expect("valid BMSON");
    let processed = bmsrs::bmson::process(&bmson).expect("valid chart");
    assert_ne!(processed.value.data().events().len(), 0);
}

#[cfg(feature = "bmson")]
#[test]
fn bmson_parse_file_reads_chart_from_path() {
    let path = std::env::temp_dir().join(format!("bmsrs-facade-test-{}.bmson", std::process::id()));
    let json = r#"{
        "version": "2.0.0",
        "song_info": {"title": "T", "artist": "A", "genre": "G"},
        "chart_info": {"level": 1, "bga": {"bga_header": [], "bga_events": [], "layer_events": [], "poor_events": []}},
        "chart_data": {"init_bpm": 120}
    }"#;
    std::fs::write(&path, json).expect("write fixture");
    let bmson = bmsrs::bmson::parse_file(&path).expect("valid BMSON file");
    std::fs::remove_file(&path).expect("remove fixture");
    assert_eq!(bmson.song_info.title, "T");
}
