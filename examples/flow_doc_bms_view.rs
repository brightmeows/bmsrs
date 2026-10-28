//! 示例：在 rowan CST 上选择控制流分支并查看每轮结果。
//!
//! `SourceFile::select_lines` 保留所有分支的 CST，只按随机值返回选中行；
//! `Bms::from_cst` 在同一份 CST 上做语义 lowering。本示例对同一份谱面
//! 分别取随机值 1 和 2，展示两次选择的行与标题。
//!
//! 运行方式：
//!
//! ```sh
//! cargo run --example flow_doc_bms_view
//! ```

// 示例的职责就是向 stdout 输出；工作区内禁用
// `println!`（改用 `tracing`）的规则在此不适用。
#![expect(clippy::print_stdout, reason = "example prints to stdout by design")]

use bmsrs::bms::parser::Bms;
use bmsrs::bms::syntax::SourceFile;

/// 一份极小谱面：含一个 `#RANDOM` 块，两条分支各带自己的标题与通道消息。
const SOURCE: &str = "\
#TITLE Demo
#BPM 120
#WAV01 a.wav
#WAV02 b.wav
#RANDOM 2
#IF 1
#TITLE Branch A
#00101:0101
#ENDIF
#IF 2
#TITLE Branch B
#00101:0202
#ENDIF
#ENDRANDOM
";

fn main() {
    let parse = SourceFile::parse(SOURCE);
    let file = parse.source_file();

    for value in 1..=2 {
        let (lines, selection) = file.select_lines(|_| value);
        let parsed = Bms::from_cst(&file, |_| value);
        println!(
            "value {value}: {} selected lines, {} decisions, title {:?}",
            lines.len(),
            selection.decisions().len(),
            parsed.bms.metadata.title
        );
        for line in &lines {
            if let Some(command) = line.command_name() {
                let line_value = line
                    .value()
                    .map(|token| token.text().to_owned())
                    .unwrap_or_default();
                println!("  #{} {}", command.text(), line_value);
            }
        }
    }
}
