//! Public facade integration tests.

#[test]
fn bms_parse_and_process_return_outcomes() {
    let parsed = bmsrs::bms::parse(bmsrs::bms::Input::text(
        "#TITLE A\n#BPM 120\n#TOTAL 100\n#WAV01 a.wav\n#00111:0100\n",
        bmsrs::pipeline::SourceFormat::Bms,
    ))
    .expect("valid BMS");
    assert_eq!(parsed.value.metadata.title.as_deref(), Some("A"));

    let processed = bmsrs::bms::process(&parsed.value).expect("valid chart");
    assert_ne!(processed.value.chart.data().events().len(), 0);
    assert_eq!(processed.warnings.len(), 0);

    let text = bmsrs::bms::write(&parsed.value);
    assert!(text.contains("#TITLE A"));
}

#[test]
fn bms_parse_reads_a_chart_from_a_path() {
    let path = std::env::temp_dir().join(format!("bmsrs-facade-test-{}.bms", std::process::id()));
    std::fs::write(&path, "#TITLE A\n#BPM 120\n#WAV01 a.wav\n#00111:0100\n")
        .expect("write fixture");
    let parsed = bmsrs::bms::parse(&path).expect("valid BMS file");
    std::fs::remove_file(&path).expect("remove fixture");
    assert_eq!(parsed.value.metadata.title.as_deref(), Some("A"));
}

#[test]
fn bms_parse_with_accepts_declared_bytes_and_curated_option_types() {
    // Bytes declare their format; the option types resolve through `bms::`,
    // not only through the low-level `raw` path this facade is meant to hide.
    let bytes = b"#TITLE A\n#BPM 120\n".to_vec();
    let parsed = bmsrs::bms::parse_with(
        bmsrs::bms::Input::bytes(bytes, bmsrs::pipeline::SourceFormat::Bms),
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
    let (recorded, selection) = bmsrs::bms::parse_with_recording(
        bmsrs::bms::Input::text(text, bmsrs::pipeline::SourceFormat::Bms),
        &bmsrs::bms::ParseOptions::default(),
        &mut rng,
    )
    .expect("valid BMS");
    let title_from_seed = recorded.value.metadata.title;
    assert!(
        title_from_seed.as_deref() == Some("ONE") || title_from_seed.as_deref() == Some("FIVE"),
        "a branch was selected: {title_from_seed:?}"
    );

    let replayed = bmsrs::bms::parse_from_selection(
        bmsrs::bms::Input::text(text, bmsrs::pipeline::SourceFormat::Bms),
        &selection,
        &bmsrs::bms::ParseOptions::default(),
    )
    .expect("replay ok");
    assert_eq!(replayed.value.metadata.title, title_from_seed);

    let drifted = text.replace("#RANDOM 5", "#RANDOM 4");
    let error = bmsrs::bms::parse_from_selection(
        bmsrs::bms::Input::text(&drifted, bmsrs::pipeline::SourceFormat::Bms),
        &selection,
        &bmsrs::bms::ParseOptions::default(),
    );
    assert!(matches!(error, Err(bmsrs::bms::ParseError::Branch(_))));
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
        bmsrs::bms::Input::text(
            "#RANDOM 0\n#IF 1\n#TITLE zero\n#ENDIF\n",
            bmsrs::pipeline::SourceFormat::Bms,
        ),
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
    let parsed = bmsrs::bms::parse(&path).expect("valid BMD file");
    std::fs::remove_file(&path).expect("remove fixture");

    assert_eq!(parsed.value.format, bmsrs::pipeline::SourceFormat::Bmd);
    assert_eq!(parsed.value.audio.wav_sources.len(), 1);
    assert!(matches!(
        parsed.value.audio.wav_sources.values().next(),
        Some(bmsrs::raw::bms::model::WavSource::Synth(_))
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
        named::<bmsrs::bms::Outcome<bmsrs::bms::Bms, bmsrs::bms::BmsDiagnostic>>(),
        named::<bmsrs::bms::Processed>(),
        named::<bmsrs::bms::ProcessWarning>(),
        named::<bmsrs::bms::ProcessError>(),
        named::<bmsrs::bms::AssetLookup>(),
        named::<bmsrs::bms::layout::ChannelPos>(),
        named::<bmsrs::bms::layout::DscOctFp>(),
        named::<bmsrs::bms::layout::Nanasi>(),
        named::<bmsrs::bms::layout::Pms>(),
        named::<bmsrs::bms::layout::PmsBme>(),
        named::<bmsrs::bms::TextRange>(),
        named::<bmsrs::bms::BmsTokenizeError>(),
        named::<bmsrs::bms::TextDecodeError>(),
        named::<bmsrs::bms::ParseOptions>(),
        named::<bmsrs::bms::Input>(),
        named::<bmsrs::bms::SourceFormat>(),
        named::<bmsrs::bms::BranchSelection>(),
        named::<bmsrs::bms::BranchMismatch>(),
        named::<bmsrs::bms::ControlFlowCompat>(),
        named::<bmsrs::bms::ParseError>(),
        named::<bmsrs::bms::BmsDiagnostic>(),
        named::<bmsrs::bms::BmsDiagnosticKind>(),
    ];
    assert_eq!(resolved.len(), 23);
    // `type_name` reports the defining crate, not the re-export path, so the
    // value only has to exist; resolving these paths is the assertion.
    assert!(resolved.iter().all(|name| !name.is_empty()));

    // The layout family is a bound, not a value, so name it through a call.
    bms_layout::<bmsrs::bms::Bme>();
    bms_layout::<bmsrs::bms::layout::PmsLayout>();
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
        named::<bmsrs::bmson::layout::GenericLayout>(),
        named::<
            bmsrs::bmson::Outcome<bmsrs::bmson::BmsonProcessed, bmsrs::bmson::BmsonProcessWarning>,
        >(),
        named::<bmsrs::bmson::BmsonProcessed>(),
        named::<bmsrs::bmson::BmsonProcessWarning>(),
        named::<bmsrs::bmson::ProcessError>(),
        named::<bmsrs::bmson::BmsonParseError>(),
    ];
    assert_eq!(resolved.len(), 7);
    assert!(resolved.iter().all(|name| !name.is_empty()));

    bmson_layout::<bmsrs::bmson::Beat>();
    bmson_layout::<bmsrs::bmson::layout::GenericLayout>();
}

#[test]
fn pipeline_diagnostics_keep_the_syntax_control_flow_distinction() {
    use bmsrs::pipeline::{Diagnostic, DiagnosticCode};

    // An unclosed control-flow block is a structure warning, not a syntax one.
    let unclosed = bmsrs::bms::parse(bmsrs::bms::Input::text(
        "#RANDOM 2\n#IF 1\n#TITLE A\n",
        bmsrs::pipeline::SourceFormat::Bms,
    ))
    .expect("valid BMS");
    let codes: Vec<_> = unclosed
        .warnings
        .into_iter()
        .map(Diagnostic::from)
        .filter_map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&DiagnosticCode::ControlFlow));
    assert!(!codes.contains(&DiagnosticCode::Syntax));

    // A line whose command word is empty is a syntax error.
    let invalid = bmsrs::bms::parse(bmsrs::bms::Input::text(
        "# BAD\n",
        bmsrs::pipeline::SourceFormat::Bms,
    ))
    .expect("valid BMS");
    let invalid_codes: Vec<_> = invalid
        .warnings
        .into_iter()
        .map(Diagnostic::from)
        .filter_map(|diagnostic| diagnostic.code)
        .collect();
    assert!(invalid_codes.contains(&DiagnosticCode::Syntax));
    assert!(!invalid_codes.contains(&DiagnosticCode::ControlFlow));
}

#[test]
fn fail_fast_keeps_parsing_past_a_control_flow_warning() {
    use bmsrs::bms::{BmsDiagnosticKind, ParseOptions};
    use bmsrs::pipeline::{DiagnosticCode, ErrorStrategy, Severity};

    // An unclosed `#RANDOM` is warning-grade in the syntax layer, so FailFast
    // keeps parsing and reports it in the list instead of failing.
    let parsed = bmsrs::bms::parse_with(
        bmsrs::bms::Input::text(
            "#RANDOM 2\n#IF 1\n#TITLE A\n",
            bmsrs::pipeline::SourceFormat::Bms,
        ),
        &ParseOptions {
            error_strategy: ErrorStrategy::FailFast,
            ..ParseOptions::default()
        },
        &mut |_| 1,
    )
    .expect("a control-flow warning is not fatal");
    assert_eq!(parsed.value.metadata.title.as_deref(), Some("A"));

    let unclosed = parsed
        .warnings
        .iter()
        .find(|warning| {
            matches!(
                warning.kind,
                BmsDiagnosticKind::Cst {
                    code: DiagnosticCode::ControlFlow,
                    ..
                }
            )
        })
        .expect("the unclosed block is reported");
    assert_eq!(unclosed.severity, Severity::Warning);
    assert!(!unclosed.is_error());
}

#[test]
fn syntax_error_keeps_error_severity_at_the_pipeline_exit() {
    let parsed = bmsrs::bms::parse(bmsrs::bms::Input::text(
        "# BAD\n",
        bmsrs::pipeline::SourceFormat::Bms,
    ))
    .expect("collect-all parses");
    let diagnostic = parsed
        .warnings
        .iter()
        .find(|warning| matches!(warning.kind, bmsrs::bms::BmsDiagnosticKind::Cst { .. }))
        .cloned()
        .map(bmsrs::pipeline::Diagnostic::from)
        .expect("one CST diagnostic");

    assert_eq!(diagnostic.severity, bmsrs::pipeline::Severity::Error);
    assert_eq!(
        diagnostic.code,
        Some(bmsrs::pipeline::DiagnosticCode::Syntax)
    );
    assert!(diagnostic.span.is_some_and(|span| !span.is_empty()));
}

#[test]
fn bmd_bytes_declare_their_format() {
    use bmsrs::bms::Input;
    use bmsrs::pipeline::SourceFormat;

    let bytes = b"#WAV01 P0 N60 V100 G48\n".to_vec();
    let declared =
        bmsrs::bms::parse(Input::bytes(bytes.clone(), SourceFormat::Bmd)).expect("valid BMD bytes");
    assert_eq!(declared.value.format, SourceFormat::Bmd);
    assert!(matches!(
        declared.value.audio.wav_sources.values().next(),
        Some(bmsrs::raw::bms::model::WavSource::Synth(_))
    ));

    // The same bytes declared as plain BMS keep the literal `#WAV` value.
    let plain = bmsrs::bms::parse(Input::bytes(bytes, SourceFormat::Bms)).expect("valid BMS bytes");
    assert_eq!(plain.value.format, SourceFormat::Bms);
    assert!(matches!(
        plain.value.audio.wav_sources.values().next(),
        Some(bmsrs::raw::bms::model::WavSource::File(_))
    ));
}

#[test]
fn replay_takes_the_options_the_recording_was_made_with() {
    use bmsrs::bms::Input;
    use bmsrs::pipeline::SourceFormat;

    let bytes = b"#WAV01 P0 N60 V100 G48\n".to_vec();
    let (recorded, selection) = bmsrs::bms::parse_with_recording(
        Input::bytes(bytes.clone(), SourceFormat::Bmd),
        &bmsrs::bms::ParseOptions::default(),
        &mut |_| 1,
    )
    .expect("record");
    let replayed = bmsrs::bms::parse_from_selection(
        Input::bytes(bytes, SourceFormat::Bmd),
        &selection,
        &bmsrs::bms::ParseOptions::default(),
    )
    .expect("replay");

    assert_eq!(recorded.value.format, replayed.value.format);
    assert_eq!(
        recorded.value.audio.wav_sources,
        replayed.value.audio.wav_sources
    );
    assert!(matches!(
        replayed.value.audio.wav_sources.values().next(),
        Some(bmsrs::raw::bms::model::WavSource::Synth(_))
    ));
}

#[test]
fn prelude_exports_every_type_and_no_functions() {
    use bmsrs::prelude::*;

    // Traits are named through a bound, not a type argument.
    fn bounded_by<T: BmsLayout>() {}
    fn drawn_by<T: ControlFlowRng>() {}
    fn bmson_bounded_by<T: BmsonLayout>() {}

    fn named<T>() -> &'static str {
        std::any::type_name::<T>()
    }

    // The prelude is types and traits only; every item is named here so a
    // dropped re-export fails to compile.
    let resolved = [
        named::<AssetLookup>(),
        named::<Bms>(),
        named::<BmsDiagnostic>(),
        named::<BmsDiagnosticKind>(),
        named::<BranchMismatch>(),
        named::<BranchSelection>(),
        named::<ControlFlowCompat>(),
        named::<ErrorStrategy>(),
        named::<FirstBranch>(),
        named::<Input>(),
        named::<JavaRandom>(),
        named::<Lr2Mt19937>(),
        named::<Outcome<Bms, BmsDiagnostic>>(),
        named::<ParseError>(),
        named::<ParseOptions>(),
        named::<ProcessError>(),
        named::<ProcessWarning>(),
        named::<Processed>(),
        named::<SourceFormat>(),
        named::<TextDecodeError>(),
        named::<TextRange>(),
        named::<BmsTokenizeError>(),
        named::<Bme>(),
        named::<Beat>(),
        named::<GenericLayout>(),
        named::<Bmson>(),
        named::<BmsonParseError>(),
        named::<BmsonProcessed>(),
        named::<BmsonProcessWarning>(),
        named::<Chart>(),
        named::<ChartData>(),
        named::<Event>(),
        named::<EventKind<()>>(),
        named::<Lane>(),
        named::<Note>(),
        named::<NoteKind>(),
        named::<NoteRole>(),
        named::<TimingTrack>(),
        named::<Diagnostic>(),
        named::<DiagnosticCode>(),
        named::<Severity>(),
        named::<SourceSpan>(),
        named::<Player>(),
    ];
    assert_eq!(resolved.len(), 43);
    assert!(resolved.iter().all(|name| !name.is_empty()));
    bounded_by::<Bme>();
    drawn_by::<JavaRandom>();
    bmson_bounded_by::<Beat>();
}

#[test]
fn bms_parse_rejects_a_bmson_path() {
    // A `.bmson` path used to decode as an empty BMS chart with zero
    // warnings; the BMS entry points refuse the format before reading.
    let name = [
        "bmsrs-facade-bmson-",
        &std::process::id().to_string(),
        ".bmson",
    ]
    .concat();
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, r#"{"version":"2.0.0"}"#).expect("write fixture");
    let error = bmsrs::bms::parse(&path).expect_err("a bmson path is not BMS input");
    std::fs::remove_file(&path).expect("remove fixture");
    assert!(matches!(
        error,
        bmsrs::bms::ParseError::Format {
            format: bmsrs::bms::SourceFormat::Bmson
        }
    ));

    // Text and bytes carry the format explicitly and hit the same check.
    let text_error = bmsrs::bms::parse(bmsrs::bms::Input::text(
        "{}",
        bmsrs::bms::SourceFormat::Bmson,
    ))
    .expect_err("declared Bmson text is not BMS input");
    assert!(matches!(text_error, bmsrs::bms::ParseError::Format { .. }));
}

#[test]
fn parse_from_selection_fail_fast_returns_the_first_error() {
    use bmsrs::bms::{Input, ParseOptions};
    use bmsrs::pipeline::{DiagnosticCode, ErrorStrategy, SourceFormat};

    // Record under CollectAll: the syntax error stays in the warning list.
    let text = "# BAD\n#RANDOM 2\n#IF 1\n#TITLE A\n#ENDIF\n#ENDRANDOM\n#BPM 120\n";
    let (_, selection) = bmsrs::bms::parse_with_recording(
        Input::text(text, SourceFormat::Bms),
        &ParseOptions::default(),
        &mut |_| 1,
    )
    .expect("record under CollectAll");

    // Replaying under FailFast must surface the same error the document
    // path reports, instead of returning it inside the warning list.
    let error = bmsrs::bms::parse_from_selection(
        Input::text(text, SourceFormat::Bms),
        &selection,
        &ParseOptions {
            error_strategy: ErrorStrategy::FailFast,
            ..ParseOptions::default()
        },
    )
    .expect_err("FailFast stops at the syntax error");
    let bmsrs::bms::ParseError::Diagnostic(diagnostic) = error else {
        panic!("expected ParseError::Diagnostic, got {error:?}");
    };
    assert!(diagnostic.is_error());
    assert!(matches!(
        diagnostic.kind,
        bmsrs::bms::BmsDiagnosticKind::Cst {
            code: DiagnosticCode::Syntax,
            ..
        }
    ));
}
