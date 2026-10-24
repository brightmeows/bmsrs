//! token 抽取与行号测试。

use bms_document::{Document, DocumentOptions, LineKind};

#[test]
fn token_line_numbers_are_physical_lines() {
    let document = Document::from_text("#TITLE A\n// c\n\n#00111:0102\n");
    let numbers: Vec<_> = document.tokens().map(|(number, _)| number.get()).collect();
    assert_eq!(numbers, vec![1, 4]);
}

#[test]
fn token_results_report_parse_errors() {
    let document = Document::from_text("#BPM abc\n");
    assert!(matches!(document.token_results().next(), Some((_, Err(_)))));
}

#[test]
fn token_results_skip_trivia() {
    let document = Document::from_text("// c\n\n#TITLE A\n");
    assert_eq!(document.token_results().count(), 1);
}

#[test]
fn percent_prefix_is_recognized() {
    let document = Document::from_text("%TITLE A\n");
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Header(_)))
    );
}

#[test]
fn custom_prefix_is_recognized() {
    let document = Document::from_bytes_with(
        b"@TITLE A\n".to_vec(),
        &DocumentOptions {
            encoding: None,
            header_prefixes: vec!['@'],
        },
    );
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Header(_)))
    );
}

#[test]
fn excluded_prefix_leaves_line_as_text() {
    let document = Document::from_bytes_with(
        b"@TITLE A\n".to_vec(),
        &DocumentOptions {
            encoding: None,
            header_prefixes: vec!['#'],
        },
    );
    assert!(
        document
            .line(0)
            .is_some_and(|line| matches!(line.kind(), LineKind::Text))
    );
}
