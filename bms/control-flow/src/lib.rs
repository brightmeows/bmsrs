//! Control-flow tree construction, branch selection, and roundtrip conversion for BMS.
//!
//! This crate takes the flat token stream from [`bms_tokenizer`] and builds a
//! structured tree ([`FlowTree`]) that preserves all control-flow branches.
//! It can then select a single branch per block using an RNG, or convert the
//! tree back to a flat token sequence.
//!
//! # Usage
//!
//! ```
//! use bms_control_flow::FlowTree;
//! use bms_tokenizer::BmsTokenizer;
//!
//! let tokens: Vec<_> = BmsTokenizer::new()
//!     .tokenize::<Vec<_>, &str>("#RANDOM 2\n#IF 1\n#00101:11\n#ENDIF\n#ENDRANDOM")
//!     .into_iter()
//!     .filter_map(|(line, res)| res.ok().map(|t| (line, t)))
//!     .collect();
//!
//! let tree = FlowTree::from_tokens(tokens).unwrap();
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

pub use error::ControlFlowError;
pub use rng::{BranchRng, DeterministicRng};
pub use types::*;
