//! Per-note BMS extension data.

use bmsrs_chart::{LnTypeHint, NoteExt};

/// Per-note BMS extension: the long-note type assigned by a typed
/// `#CNOBJ` / `#HCNOBJ` / `#HLNOBJ` terminator marker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BmsNoteExt {
    /// Long-note type from the matched terminator marker.
    pub ln_type_hint: Option<LnTypeHint>,
}

impl NoteExt for BmsNoteExt {
    fn ln_type_hint(&self) -> Option<LnTypeHint> {
        self.ln_type_hint
    }
}
