//! Incremental parser behaviour tests.

use bmsrs_bms_syntax::{IncrementalParser, ParseOptions, SourceFile};

fn full(text: &str) -> SourceFile {
    SourceFile::parse(text).source_file()
}

#[test]
fn single_line_edit_matches_full_parse() {
    let old = "#TITLE A\n#BPM 120\n#00111:0101\n";
    let new = "#TITLE B\n#BPM 120\n#00111:0101\n";
    let mut parser = IncrementalParser::new(old, ParseOptions::default());
    parser.update(new);

    assert_eq!(parser.text(), new);
    assert_eq!(
        parser.parse().source_file().syntax().to_string(),
        full(new).syntax().to_string()
    );
    assert_eq!(parser.parse().errors(), SourceFile::parse(new).errors());
}

#[test]
fn line_insertion_falls_back_and_matches_full_parse() {
    let old = "#TITLE A\n#BPM 120\n";
    let new = "#TITLE A\n#ARTIST B\n#BPM 120\n";
    let mut parser = IncrementalParser::new(old, ParseOptions::default());
    parser.update(new);

    assert_eq!(
        parser.parse().source_file().syntax().to_string(),
        full(new).syntax().to_string()
    );
    assert_eq!(parser.parse().errors(), SourceFile::parse(new).errors());
}

#[test]
fn control_flow_edit_falls_back_and_matches_full_parse() {
    let old = "#RANDOM 1\n#IF 1\n#TITLE A\n#ENDIF\n#ENDRANDOM\n";
    let new = "#RANDOM 2\n#IF 1\n#TITLE A\n#ENDIF\n#ENDRANDOM\n";
    let mut parser = IncrementalParser::new(old, ParseOptions::default());
    parser.update(new);

    assert_eq!(
        parser.parse().source_file().syntax().to_string(),
        full(new).syntax().to_string()
    );
    assert_eq!(parser.parse().errors(), SourceFile::parse(new).errors());
}
