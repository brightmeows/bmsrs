//! Example: select control-flow branches on a rowan CST and inspect each round.
//!
//! `SourceFile::select_lines` keeps every branch in the CST and returns only the
//! selected lines for a random value; `Bms::from_cst` performs semantic lowering
//! on the same CST. This example evaluates the same chart with random values 1
//! and 2, showing the selected lines and titles for each round.
//!
//! Run:
//!
//! ```sh
//! cargo run --example flow_doc_bms_view
//! ```

// This example's job is to print to stdout; the workspace rule that bans
// `println!` in favor of `tracing` does not apply here.
#![expect(clippy::print_stdout, reason = "example prints to stdout by design")]

use bmsrs::bms::parser::Bms;
use bmsrs::bms::syntax::SourceFile;

/// A minimal chart with one `#RANDOM` block; each branch has its own title and
/// channel messages.
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
