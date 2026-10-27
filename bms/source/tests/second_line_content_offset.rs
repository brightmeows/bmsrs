//! 第二行 `content` 区间的回归测试。
//!
//! 构建时 `scanned.content` 相对行 body 起算，换算文档绝对字节须加
//! 行起始偏移；历史上只加了 BOM 长度，首行碰巧正确而第二行起错位
//! （编辑写回时旧行内容混入新行）。

#[test]
fn replace_content_on_second_line_keeps_other_lines_intact() {
    let mut doc = bms_source::Document::from_bytes(b"#TITLE t\n#BPM 160\n".to_vec());
    doc.replace_content(1, "#BPM 170").expect("replace");
    let out = String::from_utf8(doc.render()).expect("utf8");
    eprintln!("RENDERED={out:?}");
    assert_eq!(out, "#TITLE t\n#BPM 170\n");
}
