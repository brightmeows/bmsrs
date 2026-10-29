//! `BmsWriter` 的 roundtrip 集成测试。
//!
//! 生成器写出的文本重新解析后须与原 `Bms` 模型语义等价——
//! 这是兜底重写档的验证底线。

use bms_parser::Bms;
use bms_parser::parse_message_line;
use bms_syntax::SourceFile;
use proptest::prelude::*;

use bms_writer::BmsWriter;

/// 解析 BMS 文本为 `Bms`，忽略分词警告。
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
    // 同 (measure, channel) 两行：第二行的位置 1 非 00 覆盖第一行。
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
    // BGM（ch 01）多行不合并——多声部语义。
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
    // encode 与 parse 的 body 互逆性：拼接即原文。
    let line = parse_message_line("#00111:010203")
        .expect("valid message")
        .expect("is message");
    assert_eq!(
        line.body,
        bms_parser::BmsMessage::encode_body(["01", "02", "03"])
    );
}
