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
