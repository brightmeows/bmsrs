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
fn bms_parse_bytes_with_accepts_curated_option_types() {
    // Both option types a caller needs must resolve through `bms::`, not only
    // through the low-level `raw` path this facade is meant to hide.
    let bytes = b"#TITLE A\n#BPM 120\n".to_vec();
    let parsed = bmsrs::bms::parse_bytes_with(
        bytes,
        &bmsrs::bms::DocumentOptions::default(),
        &bmsrs::bms::ParseOptions::default(),
        &mut |_| 1,
    )
    .expect("valid BMS bytes");
    assert_eq!(parsed.value.metadata.title.as_deref(), Some("A"));
}

#[test]
fn java_random_seed_reproduces_branch_layout_and_replays() {
    let text =
        "#RANDOM 5\n#IF 1\n#TITLE ONE\n#ENDIF\n#IF 5\n#TITLE FIVE\n#ENDIF\n#ENDRANDOM\n#BPM 120\n";
    let mut rng = bmsrs::bms::JavaRandom::new(123_456_789);
    let (recorded, selection) =
        bmsrs::bms::parse_with_recording(text, &bmsrs::bms::ParseOptions::default(), &mut rng)
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
    assert_ne!(processed.value.chart.data().events().len(), 0);
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

#[test]
fn jbms_compat_mode_and_lr2_rng_run_through_the_facade() {
    use bmsrs::bms::{ControlFlowCompat, ControlFlowRng as _, JavaRandom, Lr2Mt19937};

    // beatoraja-jbms behavior: `#RANDOM 0` clamps to 1 and draws once.
    let parsed = bmsrs::bms::parse_with(
        "#RANDOM 0\n#IF 1\n#TITLE zero\n#ENDIF\n",
        &bmsrs::bms::ParseOptions {
            control_flow_compat: ControlFlowCompat::Jbms,
            ..bmsrs::bms::ParseOptions::default()
        },
        &mut |_| 1,
    )
    .expect("parse should succeed");
    assert_eq!(parsed.value.metadata.title.as_deref(), Some("zero"));

    // Both standard sources satisfy the draw contract, so one call site can
    // swap engines.
    let mut java = JavaRandom::new(1);
    let mut lr2 = Lr2Mt19937::new(1);
    assert!((1..=4).contains(&java.draw(4)));
    assert!((1..=4).contains(&lr2.draw(4)));
}

#[test]
fn bmd_parse_file_selects_the_dialect_format() {
    let path = std::env::temp_dir().join(format!("bmsrs-facade-test-{}.bmd", std::process::id()));
    std::fs::write(
        &path,
        "#WAV01 P0 N60 V100 G48\n#00111:01 ; inline comment\n",
    )
    .expect("write fixture");
    let parsed = bmsrs::bms::parse_file(&path).expect("valid BMD file");
    std::fs::remove_file(&path).expect("remove fixture");

    assert_eq!(parsed.value.format, bmsrs::pipeline::SourceFormat::Bmd);
    assert_eq!(parsed.value.audio.wav_sources.len(), 1);
    assert!(matches!(
        parsed.value.audio.wav_sources.values().next(),
        Some(bmsrs::raw::model::WavSource::Synth(_))
    ));
}

#[test]
fn curated_modules_name_the_types_their_signatures_use() {
    fn named<T>() -> &'static str {
        std::any::type_name::<T>()
    }
    fn bms_layout<T: bmsrs::bms::BmsLayout>() {}

    // Each path below appears in a curated entry point's signature, so a
    // caller must be able to name it from the same module without reaching
    // into `raw`.
    let resolved = [
        named::<bmsrs::bms::Bms>(),
        named::<bmsrs::bms::Outcome<bmsrs::bms::Bms, bmsrs::bms::FromTextWarning>>(),
        named::<bmsrs::bms::Processed>(),
        named::<bmsrs::bms::ProcessWarning>(),
        named::<bmsrs::bms::ProcessError>(),
        named::<bmsrs::bms::TextRange>(),
        named::<bmsrs::bms::BmsTokenizeError>(),
        named::<bmsrs::bms::TextDecodeError>(),
        named::<bmsrs::bms::ParseOptions>(),
        named::<bmsrs::bms::DocumentOptions>(),
        named::<bmsrs::bms::BranchSelection>(),
        named::<bmsrs::bms::BranchMismatch>(),
        named::<bmsrs::bms::ControlFlowCompat>(),
        named::<bmsrs::bms::FromTextError>(),
        named::<bmsrs::bms::FromTextWarning>(),
        named::<bmsrs::bms::ReplayParseError>(),
    ];
    assert_eq!(resolved.len(), 16);
    // `type_name` reports the defining crate, not the re-export path, so the
    // value only has to exist; resolving these paths is the assertion.
    assert!(resolved.iter().all(|name| !name.is_empty()));

    // The layout family is a bound, not a value, so name it through a call.
    bms_layout::<bmsrs::bms::Bme>();
}

#[cfg(feature = "bmson")]
#[test]
fn curated_bmson_module_names_the_types_its_signatures_use() {
    fn named<T>() -> &'static str {
        std::any::type_name::<T>()
    }
    fn bmson_layout<T: bmsrs::bmson::BmsonLayout>() {}

    let resolved = [
        named::<bmsrs::bmson::Bmson>(),
        named::<
            bmsrs::bmson::Outcome<bmsrs::bmson::BmsonProcessed, bmsrs::bmson::BmsonProcessWarning>,
        >(),
        named::<bmsrs::bmson::BmsonProcessed>(),
        named::<bmsrs::bmson::BmsonProcessWarning>(),
        named::<bmsrs::bmson::ProcessError>(),
        named::<bmsrs::bmson::BmsonParseError>(),
    ];
    assert_eq!(resolved.len(), 6);
    assert!(resolved.iter().all(|name| !name.is_empty()));

    bmson_layout::<bmsrs::bmson::Beat>();
}

#[test]
fn pipeline_diagnostics_keep_the_syntax_control_flow_distinction() {
    use bmsrs::pipeline::{Diagnostic, DiagnosticCode};

    // An unclosed control-flow block is a structure warning, not a syntax one.
    let unclosed = bmsrs::bms::parse("#RANDOM 2\n#IF 1\n#TITLE A\n").expect("valid BMS");
    let codes: Vec<_> = unclosed
        .warnings
        .into_iter()
        .map(Diagnostic::from)
        .filter_map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&DiagnosticCode::ControlFlow));
    assert!(!codes.contains(&DiagnosticCode::Syntax));

    // A line whose command word is empty is a syntax error.
    let invalid = bmsrs::bms::parse("# BAD\n").expect("valid BMS");
    let invalid_codes: Vec<_> = invalid
        .warnings
        .into_iter()
        .map(Diagnostic::from)
        .filter_map(|diagnostic| diagnostic.code)
        .collect();
    assert!(invalid_codes.contains(&DiagnosticCode::Syntax));
    assert!(!invalid_codes.contains(&DiagnosticCode::ControlFlow));
}
