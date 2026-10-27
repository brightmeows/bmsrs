//! CST -> `Bms` 测试。

use bms_parser::Bms;
use bms_syntax::SourceFile;

#[test]
fn cst_builds_metadata_and_messages() {
    let parse = SourceFile::parse("#TITLE A\n#BPM 180\n#00111:0100\n");
    let result = Bms::from_cst(&parse.source_file(), |_| 1);
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
    let result = Bms::from_cst(&parse.source_file(), |_| 2);
    assert_eq!(result.bms.metadata.title, Some("B".to_owned()));
}

#[test]
fn cst_reports_tokenize_warning() {
    let parse = SourceFile::parse("#BPM abc\n");
    let result = Bms::from_cst(&parse.source_file(), |_| 1);
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn tokenize_lines_returns_typed_message() {
    let parse = SourceFile::parse("#TITLE A\n#00111:0100\n");
    let lines = bms_parser::tokenize_lines(&parse.source_file());
    assert!(lines.iter().any(|entry| matches!(
        entry,
        bms_parser::LineToken { line, token: Ok(bms_parser::BmsToken::Message(_)), .. }
            if line.get() == 2
    )));
}

#[test]
fn from_document_collects_unselected_branch_warning() {
    let document = bms_source::Document::from_text(
        "#RANDOM 2\n#IF 1\n#BPM 120\n#ENDIF\n#IF 2\n#BPM abc\n#ENDIF\n#ENDRANDOM\n",
    );
    let result =
        Bms::from_document(&document, |_| 1, bms_parser::ErrorStrategy::CollectAll).unwrap();
    assert!(matches!(
        result.warnings.as_slice(),
        [bms_parser::CstWarning { line, kind: bms_parser::CstWarningKind::Tokenize(_), .. }]
            if line.get() == 6
    ));
}

#[test]
fn from_document_fail_fast_returns_unselected_branch_warning() {
    let document = bms_source::Document::from_text(
        "#RANDOM 2\n#IF 1\n#BPM 120\n#ENDIF\n#IF 2\n#BPM abc\n#ENDIF\n#ENDRANDOM\n",
    );
    let result = Bms::from_document(&document, |_| 1, bms_parser::ErrorStrategy::FailFast);
    assert!(matches!(
        result,
        Err(bms_parser::CstWarning { line, .. }) if line.get() == 6
    ));
}

#[test]
fn from_document_accepts_custom_header_prefix() {
    let document = bms_source::Document::from_text("@TITLE A\n");
    let options = bms_syntax::ParseOptions {
        header_prefixes: vec!['@'],
    };
    let result = Bms::from_document_with(
        &document,
        &options,
        |_| 1,
        bms_parser::ErrorStrategy::CollectAll,
    )
    .unwrap();
    assert_eq!(result.bms.metadata.title, Some("A".to_owned()));
}
