//! CST to `Bms` tests.

use bmsrs_bms_parser::BmsParser;
use bmsrs_bms_syntax::SourceFile;

#[test]
fn cst_builds_metadata_and_messages() {
    let parse = SourceFile::parse("#TITLE A\n#BPM 180\n#00111:0100\n");
    let result = BmsParser::parse(&parse.source_file(), |_| 1);
    assert_eq!(
        (
            result.bms.metadata.title,
            result.bms.timing.bpm,
            result.bms.messages.note_events.len()
        ),
        (Some("A".to_owned()), Some(180.0), 1)
    );
}

#[test]
fn cst_selects_random_branch() {
    let parse =
        SourceFile::parse("#RANDOM 2\n#IF 1\n#TITLE A\n#ELSE\n#TITLE B\n#ENDIF\n#ENDRANDOM\n");
    let result = BmsParser::parse(&parse.source_file(), |_| 2);
    assert_eq!(result.bms.metadata.title, Some("B".to_owned()));
}

#[test]
fn cst_reports_tokenize_warning() {
    let parse = SourceFile::parse("#BPM abc\n");
    let result = BmsParser::parse(&parse.source_file(), |_| 1);
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn tokenize_lines_returns_typed_message() {
    let parse = SourceFile::parse("#TITLE A\n#00111:0100\n");
    let lines = bmsrs_bms_parser::tokenize_lines(&parse.source_file());
    assert!(lines.iter().any(|entry| matches!(
        entry,
        bmsrs_bms_parser::LineToken { line, token: Ok(bmsrs_bms_values::BmsToken::Message(_)), .. }
            if line.get() == 2
    )));
}

#[test]
fn from_document_collects_unselected_branch_warning() {
    let document = bmsrs_bms_source::Document::from_text(
        "#RANDOM 2\n#IF 1\n#BPM 120\n#ENDIF\n#IF 2\n#BPM abc\n#ENDIF\n#ENDRANDOM\n",
    );
    let result = BmsParser::parse_document(
        &document,
        |_| 1,
        bmsrs_bms_parser::ErrorStrategy::CollectAll,
    )
    .unwrap();
    assert!(matches!(
        result.warnings.as_slice(),
        [bmsrs_bms_parser::BmsParseWarning { line, kind: bmsrs_bms_parser::BmsParseWarningKind::Tokenize(_), .. }]
            if line.get() == 6
    ));
}

#[test]
fn from_document_fail_fast_returns_unselected_branch_warning() {
    let document = bmsrs_bms_source::Document::from_text(
        "#RANDOM 2\n#IF 1\n#BPM 120\n#ENDIF\n#IF 2\n#BPM abc\n#ENDIF\n#ENDRANDOM\n",
    );
    let result =
        BmsParser::parse_document(&document, |_| 1, bmsrs_bms_parser::ErrorStrategy::FailFast);
    assert!(matches!(
        result,
        Err(bmsrs_bms_parser::BmsParseWarning { line, .. }) if line.get() == 6
    ));
}

#[test]
fn from_document_accepts_custom_header_prefix() {
    let document = bmsrs_bms_source::Document::from_text("@TITLE A\n");
    let options = bmsrs_bms_syntax::ParseOptions {
        header_prefixes: vec!['@'],
    };
    let result = BmsParser::parse_document_with(
        &document,
        &options,
        |_| 1,
        bmsrs_bms_parser::ErrorStrategy::CollectAll,
    )
    .unwrap();
    assert_eq!(result.bms.metadata.title, Some("A".to_owned()));
}
