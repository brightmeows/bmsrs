//! 控制流选择测试。

use bmsrs_bms_syntax::{Line, SourceFile};

/// 提取选中行的 `#TITLE` 值。
fn titles(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| {
            line.command_name()
                .is_some_and(|token| token.text().eq_ignore_ascii_case("TITLE"))
                .then(|| line.value().map(|token| token.text().to_owned()))
                .flatten()
        })
        .collect()
}

#[test]
fn random_selects_matching_if_branch() {
    let parse =
        SourceFile::parse("#RANDOM 2\n#IF 1\n#TITLE A\n#ELSE\n#TITLE B\n#ENDIF\n#ENDRANDOM\n");
    let (lines, selection) = parse.source_file().select_lines(|_| 1);
    assert_eq!(titles(&lines), vec!["A".to_owned()]);
    assert_eq!(selection.decisions().len(), 1);
}

#[test]
fn random_selects_else_branch() {
    let parse =
        SourceFile::parse("#RANDOM 2\n#IF 1\n#TITLE A\n#ELSE\n#TITLE B\n#ENDIF\n#ENDRANDOM\n");
    let (lines, _selection) = parse.source_file().select_lines(|_| 2);
    assert_eq!(titles(&lines), vec!["B".to_owned()]);
}

#[test]
fn set_random_does_not_call_rng() {
    let parse = SourceFile::parse("#SETRANDOM 1\n#IF 1\n#TITLE A\n#ENDIF\n#ENDRANDOM\n");
    let mut called = false;
    let (lines, _selection) = parse.source_file().select_lines(|max| {
        called = true;
        max
    });
    assert!(!called && titles(&lines) == vec!["A".to_owned()]);
}

#[test]
fn switch_selects_matching_case() {
    let parse = SourceFile::parse("#SWITCH 2\n#CASE 1\n#TITLE A\n#CASE 2\n#TITLE B\n#ENDSW\n");
    let (lines, selection) = parse.source_file().select_lines(|_| 2);
    assert_eq!(titles(&lines), vec!["B".to_owned()]);
    assert_eq!(selection.decisions()[0].selected_index(), 1);
}

#[test]
fn switch_falls_through_until_skip() {
    let parse = SourceFile::parse(
        "#SWITCH 1\n#CASE 1\n#TITLE A\n#CASE 2\n#TITLE B\n#SKIP\n#CASE 3\n#TITLE C\n#ENDSW\n",
    );
    let (lines, _selection) = parse.source_file().select_lines(|_| 1);
    assert_eq!(titles(&lines), vec!["A".to_owned(), "B".to_owned()]);
}

#[test]
fn nested_random_selects_inner_branch() {
    let parse = SourceFile::parse(
        "#RANDOM 1\n#IF 1\n#RANDOM 1\n#IF 1\n#TITLE A\n#ENDIF\n#ENDRANDOM\n#ENDIF\n#ENDRANDOM\n",
    );
    let (lines, selection) = parse.source_file().select_lines(|_| 1);
    assert_eq!(
        (titles(&lines), selection.decisions().len()),
        (vec!["A".to_owned()], 2)
    );
}
