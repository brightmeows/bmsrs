//! Re-export facade for the bmsrs workspace.
//!
//! This crate aggregates and re-exports public APIs from sub-crates
//! under a unified module hierarchy mirroring the workspace directory
//! structure.

/// BMS format parsing and processing.
pub mod bms {
    /// BMS tokenizer — low-level line-by-line parsing.
    pub mod tokenizer {
        pub use bms_tokenizer::*;
    }
}

/// BMSON format type definitions and converters.
pub mod bmson {
    /// BMSON type system (v0/v1/v2).
    pub mod def {
        pub use bmson_def::v0;
        pub use bmson_def::v1;
        pub use bmson_def::*;
    }
}
