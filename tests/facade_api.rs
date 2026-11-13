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
