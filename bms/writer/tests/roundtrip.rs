//! Roundtrip integration tests for `BmsWriter`.
//!
//! Text written by the generator must be semantically equivalent to the
//! original `Bms` model when parsed again; this is the verification baseline for
//! the fallback-rewrite tier.

use bmsrs_bms_parser::Bms;
use bmsrs_bms_parser::parse_message_line;
use bmsrs_bms_syntax::SourceFile;
use proptest::prelude::*;

use bmsrs_bms_writer::BmsWriter;

/// Parse BMS text into `Bms`, ignoring tokenizer warnings.
fn parse(text: &str) -> Bms {
    let source = SourceFile::parse(text);
    Bms::from_cst(&source.source_file(), |_| 1).bms
}

#[test]
fn written_text_reparses_to_equal_model() {
    let text = concat!(
        "#TITLE 往生\n",
        "#ARTIST 白喵斯\n",
        "#PLAYER 1\n",
        "#PLAYLEVEL 12\n",
        "#BPM 195\n",
        "#WAV01 kick.wav\n",
        "#WAV02 snare.wav\n",
        "#STOP01 04\n",
        "#00111:010201\n",
        "#00203:0201\n",
        "#00101:0101\n",
        "#00255:96\n",
    );
    let bms = parse(text);
    let written = BmsWriter::write(&bms);
    let reparsed = parse(&written);
    assert_eq!(bms, reparsed);
}

#[test]
fn merged_channel_positions_preserve_nonzero_overwrite_semantics() {
    // Two lines for the same (measure, channel): position 1 in the second line
    // is non-00 and overrides the first line.
    let text = "#00111:1100\n#00111:0022\n";
    let bms = parse(text);
    let written = BmsWriter::write(&bms);
    let line = written
        .lines()
        .find(|l| l.starts_with("#00111:"))
        .expect("note line should survive rewrite");
    assert_eq!(line, "#00111:1122");
}

#[test]
fn unequal_length_lines_merge_by_fractional_position() {
    let text = "#00113:11111111\n#00113:0022332255224400\n#00113:0066\n";
    let bms = parse(text);
    let written = BmsWriter::write(&bms);
    let line = written
        .lines()
        .find(|l| l.starts_with("#00113:"))
        .expect("note line should survive rewrite");
    assert_eq!(line, "#00113:1122332266224400");
}

proptest! {
    #[test]
    fn multi_line_roundtrip_preserves_semantics(
        bodies in prop::collection::vec(
            prop::collection::vec("[0-9A-Z]{2}", 1..=8),
            1..=3,
        )
    ) {
        let mut text = String::new();
        for cells in &bodies {
            text.push_str("#00113:");
            text.push_str(&cells.concat());
            text.push('\n');
        }
        let mut before = parse(&text);
        let written = BmsWriter::write(&before);
        let mut after = parse(&written);
        before.messages.raw.clear();
        after.messages.raw.clear();
        prop_assert_eq!(before, after);
    }
}

#[test]
fn bgm_lines_stay_independent() {
    // BGM (ch 01) lines are not merged; this is polyphony semantics.
    let text = "#00101:1111\n#00101:2222\n";
    let bms = parse(text);
    let written = BmsWriter::write(&bms);
    let count = written.lines().filter(|l| l.starts_with("#00101:")).count();
    assert_eq!(count, 2);
}

#[test]
fn fallback_headers_are_preserved_in_order() {
    let text = "#TITLE X\n#XYZZY hello world\n#WAV01 a.wav\n#FOO bar\n";
    let bms = parse(text);
    let written = BmsWriter::write(&bms);
    assert!(written.contains("#XYZZY hello world"));
    assert!(written.contains("#FOO bar"));
}

#[test]
fn stp_events_are_written_with_position() {
    let text = "#TITLE S\n#STP 001.250 500\n";
    let bms = parse(text);
    let written = BmsWriter::write(&bms);
    assert!(written.contains("#STP 001.250 500"));
}

#[test]
fn encode_body_roundtrips_via_parse() {
    // encode and parse are inverses for the body: concatenation reproduces the
    // original text.
    let line = parse_message_line("#00111:010203")
        .expect("valid message")
        .expect("is message");
    assert_eq!(
        line.body,
        bmsrs_bms_parser::BmsMessage::encode_body(["01", "02", "03"])
    );
}
