//! 兼容层：单行语法类型已迁移到 `bms-parser`。
//!
//! 本 crate 仅用于迁移期测试与旧调用方，公开 API 请使用
//! `bms-parser`（facade 为 `bmsrs::bms::parser`）。

pub use bms_parser::*;
