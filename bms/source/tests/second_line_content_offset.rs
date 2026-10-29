//! Regression test for the second line's `content` range.
//!
//! When a document is built, `scanned.content` is relative to the line body;
//! converting it to absolute document bytes must add the line start offset.
//! Historically only the BOM length was added: the first line happened to be
//! correct while lines from the second onward were misplaced (editing wrote the
//! old line content into the new line).

#[test]
fn replace_content_on_second_line_keeps_other_lines_intact() {
    let mut doc = bmsrs_bms_source::Document::from_bytes(b"#TITLE t\n#BPM 160\n".to_vec());
    doc.replace_content(1, "#BPM 170").expect("replace");
    let out = String::from_utf8(doc.render()).expect("utf8");
    eprintln!("RENDERED={out:?}");
    assert_eq!(out, "#TITLE t\n#BPM 170\n");
}
