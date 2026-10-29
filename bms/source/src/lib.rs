//! BMS 源文本层。
//!
//! 本 crate 只处理“字节到行”的部分，不解析 BMS 语法：
//!
//! - **字节保真**：持有原始字节与编码，未被编辑的行在写回时原样切片，
//!   不经过任何解码与重新编码。
//! - **行级结构**：把文件切成行，记录每行的字节区间、行尾终止符、
//!   内容段与空白/注释 trivia。
//! - **编辑与写回**：按行替换、插入、删除，[`Document::render`] 输出
//!   包含未编辑行的新字节序列。
//!
//! 头部命令、通道行与 `#RANDOM` / `#SWITCH` 的结构由 `bmsrs-bms-syntax` 的
//! CST 负责。本层只把一行分类为空白、注释、普通文本或解码错误。
//!
//! # 行模型
//!
//! 一行由四段组成：
//!
//! ```text
//! <前缀 trivia><内容段><后缀 trivia><行尾终止符>
//! ```
//!
//! 内容段是第一个连续的非 trivia 区段；前缀与后缀包含缩进、行内注释与
//! 跨行块注释的残余。结构化编辑只替换内容段，前后缀按原始字节拼接，
//! 因此被编辑行里的注释与缩进不会被重新编码。
//!
//! # 编码
//!
//! 支持全部 ASCII 兼容且无状态的编码（UTF-8、Shift_JIS、EUC-KR、GBK、
//! Big5 等），以及带 BOM 的 UTF-16LE / UTF-16BE。ISO-2022-JP 与
//! `replacement` 编码被拒绝：前者有跨行状态，无法按行独立编码；后者
//! 不可解码。

mod classify;
mod document;
mod encoding;
mod line;
mod source;

pub use classify::{ScanState, Scanned};
pub use document::{Document, DocumentOptions, EditError, TextDecodeError};
pub use encoding::{
    DetectedEncoding, DetectionMethod, SourceEncoding, UnmappableCharacter, UnsupportedEncoding,
};
pub use line::{Line, LineEnding, LineError, LineKind};
