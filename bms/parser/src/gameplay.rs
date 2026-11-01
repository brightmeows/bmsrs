//! Gameplay behavior fields.
//!
//! Corresponds to the tokenizer's [`BmsHeaderGameplay`].

use std::collections::BTreeMap;

use bmsrs_bms_values::{
    BmsBase, BmsHeaderGameplay, ChangeOptionIndex, ExRankIndex, LnMode, LnObjIndex, LnType,
    PlayerMode, Rank,
};

/// Gameplay behavior settings.
///
/// Scalar fields use last-wins semantics; indexed fields use `BTreeMap`
/// (last-wins per ID).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gameplay {
    /// Game mode (`#PLAYER`).
    pub player: Option<PlayerMode>,
    /// Judgment difficulty (`#RANK`).
    pub rank: Option<Rank>,
    /// Fine-grained judgment difficulty as a percentage (`#DEFEXRANK`).
    pub def_ex_rank: Option<f64>,
    /// Maximum life gauge increase (`#TOTAL`).
    pub total: Option<f64>,
    /// Master volume percentage (`#VOLWAV`).
    pub vol_wav: Option<f64>,
    /// Long-note notation (`#LNTYPE`).
    pub ln_type: Option<LnType>,
    /// WAV index used as a long-note terminator marker (`#LNOBJ`).
    pub ln_obj: Option<LnObjIndex>,
    /// Forced LN / CN / HCN mode (`#LNMODE`).
    pub ln_mode: Option<LnMode>,
    /// Radix base for indexed commands (`#BASE`).
    pub base: Option<BmsBase>,
    /// Per-position judgment width overrides (`#EXRANKxx`).
    pub ex_rank_defs: BTreeMap<ExRankIndex, f64>,
    /// Dynamic option change definitions (`#CHANGEOPTIONxx`).
    pub change_option_defs: BTreeMap<ChangeOptionIndex, String>,
    /// Octave / fold-point flag (`#OCT` / `#FP` / `#OCT/FP`).
    pub oct_fp: Option<bool>,
    /// Engine-specific option string (`#OPTION`).
    pub option: Option<String>,
}

impl Gameplay {
    /// Apply one gameplay header command to this structure.
    ///
    /// Index keys ([`ExRankIndex`], [`ChangeOptionIndex`]) are normalized with
    /// `base` for case-insensitive comparison in standard BMS, matching the
    /// normalization contract of [`Timing`](crate::timing::Timing) and
    /// [`Visual`](crate::visual::Visual).
    pub fn apply(&mut self, header: &BmsHeaderGameplay, base: BmsBase) {
        // EXRANK and CHANGEOPTION definitions are not in the Base62 command
        // list and keep Base36 case-insensitivity even in Base62 files.
        let ext_base = crate::family_base(base, false);
        match header {
            BmsHeaderGameplay::Player(m) => self.player = Some(*m),
            BmsHeaderGameplay::Rank(r) => self.rank = Some(*r),
            BmsHeaderGameplay::DefExRank(v) => self.def_ex_rank = Some(*v),
            BmsHeaderGameplay::ExRank { id, value } => {
                self.ex_rank_defs
                    .insert(ExRankIndex::from(id.normalize(ext_base)), *value);
            }
            BmsHeaderGameplay::Total(v) => self.total = Some(*v),
            BmsHeaderGameplay::VolWav(v) => self.vol_wav = Some(*v),
            BmsHeaderGameplay::LnType(t) => self.ln_type = Some(*t),
            BmsHeaderGameplay::LnObj(id) => self.ln_obj = Some(*id),
            BmsHeaderGameplay::LnMode(m) => self.ln_mode = Some(*m),
            BmsHeaderGameplay::Base(m) => self.base = Some(*m),
            BmsHeaderGameplay::OctFp => self.oct_fp = Some(true),
            BmsHeaderGameplay::Option(s) => self.option = Some(s.clone()),
            BmsHeaderGameplay::ChangeOption { id, value } => {
                self.change_option_defs.insert(
                    ChangeOptionIndex::from(id.normalize(ext_base)),
                    value.clone(),
                );
            }
        }
    }
}
