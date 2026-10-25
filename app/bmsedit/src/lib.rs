//! `bmsedit`——供大模型调用的谱面编辑 CLI 库层。
//!
//! 无状态子命令设计：每条命令独立进程，读文件 → 操作 → 写文件
//! → 退出。人话输出为默认，`--json` 输出结构化结果。

pub mod cli;
pub mod detect;
pub mod error;
pub mod exec_bms;
pub mod exec_bmson;
pub mod lane_map;
pub mod output;
pub mod patch;

use crate::cli::BmsEdit;
use crate::error::BmsEditError;
use crate::output::Output;

/// 进程主逻辑：解析后分发到对应后端。
///
/// # Errors
///
/// 命令执行失败时透传 [`BmsEditError`]。
pub fn run(args: &BmsEdit, out: &mut Output) -> Result<(), BmsEditError> {
    if is_bmson_target(args) {
        return dispatch_bmson(args, out);
    }
    dispatch_bms(args, out)
}

/// 提取目标文件路径。
fn target_file(args: &BmsEdit) -> std::path::PathBuf {
    #[expect(
        clippy::match_same_arms,
        reason = "按命令组罗列目标文件路径，可读性优先于去重"
    )]
    match &args.command {
        crate::cli::Command::Info { file }
        | crate::cli::Command::Validate { file }
        | crate::cli::Command::Lint { file } => file.clone(),
        crate::cli::Command::Header { file, .. }
        | crate::cli::Command::Notes { file, .. }
        | crate::cli::Command::Timing { file, .. }
        | crate::cli::Command::Defs { file, .. }
        | crate::cli::Command::Measure { file, .. }
        | crate::cli::Command::Transform { file, .. } => file.clone(),
    }
}

/// 目标是否为 bmson 文件。
fn is_bmson_target(args: &BmsEdit) -> bool {
    let file = target_file(args);
    std::fs::read(&file)
        .is_ok_and(|bytes| crate::detect::detect(&file, &bytes) == crate::detect::FileFormat::Bmson)
}

/// bmson 后端分发。
#[expect(
    clippy::too_many_lines,
    reason = "子命令分发是平坦的长 match，拆分会破坏对照可读性"
)]
fn dispatch_bmson(args: &BmsEdit, out: &mut Output) -> Result<(), BmsEditError> {
    use crate::cli::{HeaderAction, MeasureAction, NotesAction, TimingAction, TransformAction};
    match &args.command {
        crate::cli::Command::Info { file } => exec_bmson::info(file, out),
        crate::cli::Command::Header {
            file,
            action: HeaderAction::Get { key, .. },
        } => exec_bmson::header_get(file, key.as_deref(), out),
        crate::cli::Command::Header {
            file,
            action: HeaderAction::Set { pairs, .. },
        } => exec_bmson::header_set(file, pairs, out),
        crate::cli::Command::Notes {
            file,
            action:
                NotesAction::List {
                    from_measure,
                    to_measure,
                    lane,
                    ..
                },
        } => exec_bmson::notes_list(file, *from_measure, *to_measure, lane.as_deref(), out),
        crate::cli::Command::Notes {
            file,
            action:
                NotesAction::Add {
                    measure,
                    lane,
                    pos,
                    def,
                    bgm,
                    ln_to,
                    ..
                },
        } => exec_bmson::notes_add(
            file,
            &exec_bmson::NotesAddArgs {
                measure: *measure,
                lane,
                pos,
                def,
                bgm: *bgm,
                ln_to: ln_to.as_deref(),
            },
            out,
        ),
        crate::cli::Command::Notes {
            file,
            action: NotesAction::Remove {
                measure, lane, pos, ..
            },
        } => exec_bmson::notes_remove(file, *measure, lane, pos.as_deref(), out),
        crate::cli::Command::Notes {
            file,
            action: NotesAction::Move { lane, from, to, .. },
        } => exec_bmson::notes_move(file, lane, from, to, out),
        crate::cli::Command::Notes {
            file,
            action:
                NotesAction::Copy {
                    from_measure,
                    to_measure,
                    dest_measure,
                    ..
                },
        } => exec_bmson::notes_copy(file, *from_measure, *to_measure, *dest_measure, out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::List { .. },
        } => exec_bmson::timing_list(file, out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::Bpm { bpm, .. },
        } => exec_bmson::timing_bpm(file, *bpm, out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::StopAdd { at, duration, .. },
        } => exec_bmson::timing_stop_add(file, at, u64::from(*duration), out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::StopRemove { at, .. },
        } => exec_bmson::timing_stop_remove(file, at, out),
        crate::cli::Command::Timing {
            action: TimingAction::Length { .. },
            ..
        }
        | crate::cli::Command::Timing {
            action: TimingAction::ScrollAdd { .. },
            ..
        } => Err(BmsEditError::Unsupported(
            "bmson timing: measure length and scroll are not mapped yet".to_owned(),
        )),
        crate::cli::Command::Defs { .. } => Err(exec_bmson::defs_unsupported()),
        crate::cli::Command::Measure {
            file,
            action: MeasureAction::Insert { at, count, .. },
        } => exec_bmson::measure_insert(file, *at, *count, out),
        crate::cli::Command::Measure {
            file,
            action: MeasureAction::Remove { at, count, .. },
        } => exec_bmson::measure_remove(file, *at, *count, out),
        crate::cli::Command::Measure {
            file,
            action: MeasureAction::CloneContent { from, to, .. },
        } => exec_bmson::measure_clone(file, *from, *to, out),
        crate::cli::Command::Transform {
            file,
            action: TransformAction::Mirror { .. },
        } => exec_bmson::transform_mirror(file, out),
        crate::cli::Command::Transform {
            file,
            action: TransformAction::Sparsify { grid, .. },
        } => exec_bmson::transform_sparsify(file, *grid, out),
        crate::cli::Command::Validate { file } => exec_bmson::validate(file, out),
        crate::cli::Command::Lint { file } => exec_bmson::lint(file, out),
    }
}

/// BMS 后端分发。
#[expect(
    clippy::too_many_lines,
    reason = "子命令分发是平坦的长 match，拆分会破坏对照可读性"
)]
fn dispatch_bms(args: &BmsEdit, out: &mut Output) -> Result<(), BmsEditError> {
    use crate::cli::{
        DefsAction, HeaderAction, MeasureAction, NotesAction, TimingAction, TransformAction,
    };
    match &args.command {
        crate::cli::Command::Info { file } => exec_bms::info(file, args.seed, out),
        crate::cli::Command::Header {
            file,
            action: HeaderAction::Get { key, .. },
        } => exec_bms::header_get(file, args.seed, key.as_deref(), out),
        crate::cli::Command::Header {
            file,
            action: HeaderAction::Set { pairs, .. },
        } => exec_bms::header_set(file, args.seed, pairs, out),
        crate::cli::Command::Notes {
            file,
            action:
                NotesAction::List {
                    from_measure,
                    to_measure,
                    lane,
                    ..
                },
        } => exec_bms::notes_list(
            file,
            args.seed,
            *from_measure,
            *to_measure,
            lane.as_deref(),
            out,
        ),
        crate::cli::Command::Notes {
            file,
            action:
                NotesAction::Add {
                    measure,
                    lane,
                    pos,
                    def,
                    bgm,
                    ln_to,
                    ..
                },
        } => exec_bms::notes_add(
            file,
            args.seed,
            &exec_bms::NotesAddArgs {
                measure: *measure,
                lane,
                pos,
                def,
                bgm: *bgm,
                ln_to: ln_to.as_deref(),
            },
            out,
        ),
        crate::cli::Command::Notes {
            file,
            action: NotesAction::Remove {
                measure, lane, pos, ..
            },
        } => exec_bms::notes_remove(file, args.seed, *measure, lane, pos.as_deref(), out),
        crate::cli::Command::Notes {
            file,
            action: NotesAction::Move { lane, from, to, .. },
        } => exec_bms::notes_move(file, args.seed, lane, from, to, out),
        crate::cli::Command::Notes {
            file,
            action:
                NotesAction::Copy {
                    from_measure,
                    to_measure,
                    dest_measure,
                    ..
                },
        } => exec_bms::notes_copy(
            file,
            args.seed,
            *from_measure,
            *to_measure,
            *dest_measure,
            out,
        ),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::List { .. },
        } => exec_bms::timing_list(file, args.seed, out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::Bpm { bpm, .. },
        } => exec_bms::timing_bpm(file, args.seed, *bpm, out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::Length { measure, ratio, .. },
        } => exec_bms::timing_length(file, args.seed, *measure, ratio.as_deref(), out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::StopAdd { at, duration, .. },
        } => exec_bms::timing_stop_add(file, args.seed, at, *duration, out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::StopRemove { at, .. },
        } => exec_bms::timing_stop_remove(file, args.seed, at, out),
        crate::cli::Command::Timing {
            file,
            action: TimingAction::ScrollAdd { at, ratio, .. },
        } => exec_bms::timing_scroll_add(file, args.seed, at, *ratio, out),
        crate::cli::Command::Defs {
            file,
            action: DefsAction::List { kind, .. },
        } => exec_bms::defs_list(file, args.seed, kind, out),
        crate::cli::Command::Defs {
            file,
            action: DefsAction::Rename { kind, from, to, .. },
        } => exec_bms::defs_rename(file, args.seed, kind, from, to, out),
        crate::cli::Command::Defs {
            file,
            action: DefsAction::Dedup { kind, .. },
        } => exec_bms::defs_dedup(file, args.seed, kind, out),
        crate::cli::Command::Defs {
            file,
            action: DefsAction::Compact { kind, .. },
        } => exec_bms::defs_compact(file, args.seed, kind, out),
        crate::cli::Command::Measure {
            file,
            action: MeasureAction::Insert { at, count, .. },
        } => exec_bms::measure_insert(file, args.seed, *at, *count, out),
        crate::cli::Command::Measure {
            file,
            action: MeasureAction::Remove { at, count, .. },
        } => exec_bms::measure_remove(file, args.seed, *at, *count, out),
        crate::cli::Command::Measure {
            file,
            action: MeasureAction::CloneContent { from, to, .. },
        } => exec_bms::measure_clone(file, args.seed, *from, *to, out),
        crate::cli::Command::Transform {
            file,
            action: TransformAction::Mirror { .. },
        } => exec_bms::transform_mirror(file, args.seed, out),
        crate::cli::Command::Transform {
            file,
            action: TransformAction::Sparsify { grid, .. },
        } => exec_bms::transform_sparsify(file, args.seed, *grid, out),
        crate::cli::Command::Validate { file } => exec_bms::validate(file, args.seed, out),
        crate::cli::Command::Lint { file } => exec_bms::lint(file, args.seed, out),
    }
}
