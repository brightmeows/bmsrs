//! BMS 的控制流树构建、分支选择与 roundtrip 转换。
//!
//! 本 crate 接收来自 [`bms_tokenizer`] 的扁平 token 流，构建一棵保留所有
//! 控制流分支的结构化树（[`FlowDoc`]）。随后可借助随机数生成器为每个块
//! 选择单个分支，或将树转换回扁平 token 序列。
//!
//! # Usage
//!
//! ```
//! use bms_control_flow::FlowDoc;
//! use bms_tokenizer::BmsTokenizer;
//!
//! let tokens: Vec<_> = BmsTokenizer::new()
//!     .tokenize::<Vec<_>, &str>("#RANDOM 2\n#IF 1\n#00101:11\n#ENDIF\n#ENDRANDOM")
//!     .into_iter()
//!     .filter_map(|(line, res)| res.ok().map(|t| (line, t)))
//!     .collect();
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
