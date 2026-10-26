//! `bmsedit` 可执行入口。

use std::process::ExitCode;

use bmsedit::{cli::BmsEdit, output::Output};

use clap::Parser as _;

fn main() -> ExitCode {
    let args = BmsEdit::parse();
    let mut out = Output::new(args.json);
    let mut exit = ExitCode::SUCCESS;
    if let Err(err) = bmsedit::run(&args, &mut out) {
        out.error(&err);
        exit = ExitCode::from(err.exit_code());
    }
    out.flush();
    exit
}
