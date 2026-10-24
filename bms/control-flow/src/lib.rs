//! BMS 的控制流树构建、分支选择与 roundtrip 转换。
//!
//! 本 crate 接收来自 [`bms_tokenizer`] 的扁平 token 流，构建一棵保留所有
//! 控制流分支的结构化树（[`FlowDoc`]）。随后可借助随机数生成器为每个块
//! 选择单个分支，或将树转换回扁平 token 序列。
//!
//! # Usage
//!
//! ```
//! use std::num::NonZeroUsize;
//!
//! use bms_control_flow::FlowDoc;
//! use bms_tokenizer::{BmsHeader, BmsHeaderControlFlow, BmsHeaderMetadata, BmsToken};
//!
//! let line = NonZeroUsize::MIN;
//! let tokens = vec![
//!     (line, BmsToken::Header(BmsHeader::ControlFlow(BmsHeaderControlFlow::Random(2)))),
//!     (line, BmsToken::Header(BmsHeader::ControlFlow(BmsHeaderControlFlow::If(1)))),
//!     (line, BmsToken::Header(BmsHeader::Metadata(BmsHeaderMetadata::Title("x".to_owned())))),
//!     (line, BmsToken::Header(BmsHeader::ControlFlow(BmsHeaderControlFlow::EndIf))),
//!     (line, BmsToken::Header(BmsHeader::ControlFlow(BmsHeaderControlFlow::EndRandom))),
//! ];
//!
//! let tree = FlowDoc::from_tokens(tokens).unwrap();
//! let flat = tree.to_tokens();
//! assert_eq!(flat.len(), 5);
//! ```

mod error;
mod from_tokens;
mod map_payload;
mod rng;
mod select;
mod to_tokens;
mod types;

pub use error::{ControlFlowError, ControlFlowWarning};
pub use rng::BranchRng;
pub use types::*;
