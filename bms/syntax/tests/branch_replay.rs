//! Recorded-branch replay tests.

use bmsrs_bms_syntax::{BlockKind, BranchSelection, SourceFile, SyntaxLine};

const FIXTURE: &str = "#RANDOM 5\n\
    #IF 1\n\
    #TITLE A\n\
    #ENDIF\n\
    #IF 5\n\
    #SUBTITLE B\n\
    #ENDIF\n\
    #ENDRANDOM\n\
    #SWITCH 3\n\
    #CASE 3\n\
    #SUBARTIST C\n\
    #SKIP\n\
    #CASE 9\n\
    #SUBARTIST D\n\
    #ENDSW\n\
    #SETRANDOM 2\n\
    #IF 2\n\
    #GENRE G\n\
    #ENDIF\n\
    #ENDRANDOM\n";

#[test]
fn replay_of_rng_selection_reproduces_lines_and_decisions() {
    let parse = SourceFile::parse(FIXTURE);
    let (rng_lines, selection) = parse.source_file().select_lines(|max| max);
    let (replay_lines, replay_selection) =
        parse.source_file().select_lines_with(&selection).unwrap();
    let rng_text: Vec<_> = rng_lines.iter().map(SyntaxLine::content_range).collect();
    let replay_text: Vec<_> = replay_lines.iter().map(SyntaxLine::content_range).collect();
    assert_eq!(rng_text, replay_text);
    assert_eq!(selection, replay_selection);
    assert_eq!(selection.decisions().len(), 3);
    assert_eq!(selection.decisions()[0].kind(), BlockKind::Random);
    assert_eq!(selection.decisions()[0].max(), Some(5));
    assert_eq!(selection.decisions()[1].kind(), BlockKind::Switch);
    assert_eq!(selection.decisions()[1].max(), Some(3));
    assert_eq!(selection.decisions()[2].kind(), BlockKind::Random);
    assert_eq!(
        selection.decisions()[2].max(),
        None,
        "SETRANDOM has no bound"
    );
}

#[test]
fn replay_rejects_kind_drift() {
    let parse = SourceFile::parse(FIXTURE);
    let (_, selection) = parse.source_file().select_lines(|max| max);
    let drifted = SourceFile::parse(&FIXTURE.replace("#SWITCH 3", "#RANDOM 3"));
    let error = drifted
        .source_file()
        .select_lines_with(&selection)
        .unwrap_err();
    assert_eq!(error.block_index(), 1);
    assert_eq!(error.expected(), Some((BlockKind::Random, Some(3))));
    assert_eq!(error.recorded(), Some((BlockKind::Switch, Some(3))));
}

#[test]
fn replay_rejects_max_drift() {
    let parse = SourceFile::parse(FIXTURE);
    let (_, selection) = parse.source_file().select_lines(|max| max);
    let drifted = SourceFile::parse(&FIXTURE.replace("#RANDOM 5", "#RANDOM 4"));
    let error = drifted
        .source_file()
        .select_lines_with(&selection)
        .unwrap_err();
    assert_eq!(error.block_index(), 0);
    assert_eq!(error.expected(), Some((BlockKind::Random, Some(4))));
    assert_eq!(error.recorded(), Some((BlockKind::Random, Some(5))));
}

#[test]
fn replay_rejects_extra_block_in_file() {
    let parse = SourceFile::parse(FIXTURE);
    let (_, selection) = parse.source_file().select_lines(|max| max);
    let drifted = SourceFile::parse(&format!(
        "{FIXTURE}#RANDOM 2\n#IF 1\n#TITLE X\n#ENDIF\n#ENDRANDOM\n"
    ));
    let error = drifted
        .source_file()
        .select_lines_with(&selection)
        .unwrap_err();
    assert_eq!(error.block_index(), 3);
    assert_eq!(error.expected(), Some((BlockKind::Random, Some(2))));
    assert_eq!(error.recorded(), None);
}

#[test]
fn replay_takes_setrandom_value_from_current_file() {
    let parse = SourceFile::parse(FIXTURE);
    let (_, selection) = parse.source_file().select_lines(|max| max);
    let edited = SourceFile::parse(&FIXTURE.replace("#SETRANDOM 2", "#SETRANDOM 3"));
    let (lines, replay) = edited.source_file().select_lines_with(&selection).unwrap();
    // Value 3 no longer matches `#IF 2`, so the genre line drops out of the
    // selection: the current file, not the recording, drives set values.
    assert!(!lines.iter().any(|line| {
        line.command_name()
            .is_some_and(|token| token.text().eq_ignore_ascii_case("GENRE"))
    }));
    assert_eq!(replay.decisions()[2].value(), 3);
}

#[test]
fn zero_random_block_never_calls_the_rng() {
    let parse = SourceFile::parse("#RANDOM 0\n#IF 1\n#TITLE A\n#ENDIF\n#ENDRANDOM\n");
    let (lines, selection) = parse
        .source_file()
        .select_lines(|_| panic!("the rng must not be called"));
    assert_eq!(lines.len(), 0, "value 0 matches no branch");
    assert_eq!(selection.decisions()[0].max(), Some(0));
    assert_eq!(selection.decisions()[0].value(), 0);
}

#[test]
fn replay_of_a_recorded_zero_block_succeeds() {
    let parse = SourceFile::parse("#RANDOM 0\n#IF 1\n#TITLE A\n#ENDIF\n#ENDRANDOM\n");
    let (_, selection) = parse.source_file().select_lines(|_| panic!("no rng calls"));
    let (lines, _) = parse.source_file().select_lines_with(&selection).unwrap();
    assert_eq!(lines.len(), 0);
}

/// Keep the type in scope for the fixture-driven assertions above.
#[test]
fn branch_selection_type_round_trips_through_clone() {
    let parse = SourceFile::parse(FIXTURE);
    let (_, selection) = parse.source_file().select_lines(|max| max);
    let clone: BranchSelection = selection.clone();
    assert_eq!(clone, selection);
}
