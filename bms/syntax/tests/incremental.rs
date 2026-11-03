//! Incremental parser behaviour tests.

use bmsrs_bms_syntax::{IncrementalParser, ParseOptions, SourceFile};
use proptest::prelude::*;

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

proptest! {
    #[test]
    fn random_single_line_edit_matches_full_parse(
        lines in prop::collection::vec(
            prop::sample::select(vec![
                "#TITLE A",
                "#BPM 120",
                "#00111:0100",
                "#WAV01 a.wav",
                "#RANDOM 1",
                "#IF 1",
                "#ENDIF",
                "#ENDRANDOM",
                "// comment",
                "",
            ]),
            1..30,
        ),
        index in 0usize..30,
        replacement in "[A-Za-z0-9 #:._]{0,12}",
    ) {
        let old = format!("{}\n", lines.join("\n"));
        let line = index % lines.len();
        let mut new_lines = lines;
        new_lines[line] = &replacement;
        let new = format!("{}\n", new_lines.join("\n"));

        let mut parser = IncrementalParser::new(&old, ParseOptions::default());
        parser.update(&new);
        let full = SourceFile::parse(&new);
        prop_assert_eq!(
            parser.parse().source_file().syntax().to_string(),
            full.source_file().syntax().to_string()
        );
        let mut incremental_errors = parser.parse().errors().to_vec();
        let mut full_errors = full.errors().to_vec();
        let key = |error: &bmsrs_bms_syntax::Diagnostic| {
            (error.range().start(), error.range().end())
        };
        incremental_errors.sort_by_key(key);
        full_errors.sort_by_key(key);
        prop_assert_eq!(incremental_errors, full_errors);
    }
}
