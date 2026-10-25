//! `write_header_line` 的替换/插入语义测试。

use bmsedit::patch::BmsDoc;

#[test]
fn existing_header_line_is_replaced_in_place() {
    let mut doc = BmsDoc::from_bytes(b"#TITLE t\n#BPM 160\n".to_vec(), 1).expect("parse");
    doc.write_header_line("BPM", "170").expect("write");
    let rendered = String::from_utf8(doc.document.render()).expect("utf8");
    assert_eq!(rendered, "#TITLE t\n#BPM 170\n");
}
