//! BMS semantic parser: CST to [`Bms`](bmsrs_bms_model::Bms).
//!
//! [`BmsParser`] tokenizes CST lines, selects `#RANDOM` / `#SWITCH` branches,
//! and builds the whole-file model. The model itself lives in
//! `bmsrs-bms-model`.

mod cst;

pub use bmsrs_pipeline::ErrorStrategy;
pub use cst::{BmsParse, BmsParser, Diagnostic, DiagnosticKind, LineToken};
