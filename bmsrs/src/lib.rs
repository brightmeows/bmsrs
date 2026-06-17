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

    /// Control-flow tree construction, branch selection, and roundtrip conversion.
    pub mod control_flow {
        pub use bms_control_flow::*;
    }

    /// BMS parser — structured document model from flat token streams.
    pub mod parser {
        pub use bms_parser::*;
    }

    /// BMS processor — converts `Bms` into a format-agnostic `Chart`.
    pub mod processor {
        pub use bms_processor::*;
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

    /// BMSON processor — converts bmson data into a format-agnostic `Chart`.
    pub mod processor {
        pub use bmson_processor::*;
    }
}

/// Format-agnostic chart data model.
pub mod chart {
    pub use bmsrs_chart::*;
}

/// Pure simulation / playback layer.
pub mod player {
    pub use bmsrs_player::*;
}
