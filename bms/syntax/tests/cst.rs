//! CST 解析测试。

use bms_syntax::{LineKind, ParseOptions, SourceFile, SyntaxKind};

#[test]
fn cst_text_is_lossless() {
    let text =
        "#TITLE Song\n/* note */\n#00111:0102\n\n#RANDOM 2\n#IF 1\n#BPM 180\n#ENDIF\n#ENDRANDOM\n";
    let parse = SourceFile::parse(text);
    assert_eq!(parse.syntax().text().to_string(), text);
}

#[test]
fn header_line_exposes_command_and_value() {
    let parse = SourceFile::parse("#TITLE Song\n");
    let line = parse
        .source_file()
        .lines()
        .next()
        .unwrap_or_else(|| panic!("line"));
    assert_eq!(
        (
            line.command_name().map(|token| token.text().to_owned()),
            line.value().map(|token| token.text().to_owned())
        ),
        (Some("TITLE".to_owned()), Some("Song".to_owned()))
    );
}

#[test]
fn message_line_exposes_measure_channel_and_chunks() {
    let parse = SourceFile::parse("#00111:0102\n");
    let line = parse
        .source_file()
        .lines()
        .next()
        .unwrap_or_else(|| panic!("line"));
    let chunks: Vec<_> = line
        .body_chunks()
        .map(|token| token.text().to_owned())
        .collect();
    assert_eq!(
        (
            line.measure().map(|token| token.text().to_owned()),
            line.channel().map(|token| token.text().to_owned()),
            chunks
        ),
        (
            Some("001".to_owned()),
            Some("11".to_owned()),
            vec!["01".to_owned(), "02".to_owned()]
        )
    );
}

#[test]
fn comment_and_blank_lines_are_recognized() {
    let parse = SourceFile::parse("// note\n\n## banner\n");
    let kinds: Vec<_> = parse
        .source_file()
        .lines()
        .map(|line| line.kind())
        .collect();
    assert_eq!(
        kinds,
        vec![LineKind::Comment, LineKind::Blank, LineKind::Comment]
    );
}

#[test]
fn text_line_is_recognized() {
    let parse = SourceFile::parse("hello world\n");
    let line = parse
        .source_file()
        .lines()
        .next()
        .unwrap_or_else(|| panic!("line"));
    assert_eq!(line.kind(), LineKind::Text);
}

#[test]
fn random_block_contains_one_chain() {
    let parse =
        SourceFile::parse("#RANDOM 2\n#IF 1\n#TITLE A\n#ELSE\n#TITLE B\n#ENDIF\n#ENDRANDOM\n");
    let file = parse.source_file();
    let random = file
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::RandomBlock);
    let chains = random
        .as_ref()
        .map(|node| {
            node.children()
                .filter(|child| child.kind() == SyntaxKind::RandomChain)
                .count()
        })
        .unwrap_or_default();
    assert_eq!(chains, 1);
}

#[test]
fn switch_block_contains_cases() {
    let parse = SourceFile::parse("#SWITCH 2\n#CASE 1\n#TITLE A\n#CASE 2\n#TITLE B\n#ENDSW\n");
    let file = parse.source_file();
    let switch = file
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::SwitchBlock);
    let cases = switch
        .as_ref()
        .map(|node| {
            node.children()
                .filter(|child| child.kind() == SyntaxKind::SwitchCase)
                .count()
        })
        .unwrap_or_default();
    assert_eq!(cases, 2);
}

#[test]
fn unclosed_random_reports_diagnostic() {
    let parse = SourceFile::parse("#RANDOM 2\n#IF 1\n#TITLE A\n");
    assert!(
        parse
            .errors()
            .iter()
            .any(|error| error.message().contains("unclosed #RANDOM"))
    );
}

#[test]
fn unmatched_endif_reports_diagnostic() {
    let parse = SourceFile::parse("#ENDIF\n");
    assert!(
        parse
            .errors()
            .iter()
            .any(|error| error.message().contains("unmatched"))
    );
}

#[test]
fn token_at_offset_finds_command_name() {
    let parse = SourceFile::parse("#TITLE Song\n");
    let file = parse.source_file();
    let found = file.token_at_offset(rowan::TextSize::from(3));
    assert_eq!(
        found.map(|token| token.text().to_owned()),
        Some("TITLE".to_owned())
    );
}

#[test]
fn custom_header_prefix_is_recognized() {
    let options = ParseOptions {
        header_prefixes: vec!['@'],
    };
    let parse = SourceFile::parse_with("@TITLE Song\n", &options);
    let line = parse
        .source_file()
        .lines()
        .next()
        .unwrap_or_else(|| panic!("line"));
    assert_eq!(
        (
            line.command_name().map(|token| token.text().to_owned()),
            line.value().map(|token| token.text().to_owned())
        ),
        (Some("TITLE".to_owned()), Some("Song".to_owned()))
    );
}

#[test]
fn custom_header_prefix_is_text_by_default() {
    let parse = SourceFile::parse("@TITLE Song\n");
    let line = parse
        .source_file()
        .lines()
        .next()
        .unwrap_or_else(|| panic!("line"));
    assert_eq!(line.kind(), LineKind::Text);
}
