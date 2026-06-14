//! Gameplay behaviour fields.
//!
//! Corresponds to [`BmsHeaderGameplay`] from the tokenizer.

use std::collections::BTreeMap;

use bms_tokenizer::{
    BmsBaseMode, BmsChannelId, BmsHeaderGameplay, ChangeOptionTag, ExRankTag, LnMode, LnObjTag,
    LnType, PlayerMode, Rank,
};

/// Gameplay behaviour settings.
///
/// Scalar fields use last-wins semantics; indexed fields use
/// `BTreeMap` (last-wins per ID).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gameplay {
    /// Game mode (`#PLAYER`).
    pub player: Option<PlayerMode>,
    /// Judgment difficulty (`#RANK`).
    pub rank: Option<Rank>,
    /// Fine-grained judgment difficulty as percentage (`#DEFEXRANK`).
    pub def_ex_rank: Option<f64>,
    /// Maximum groove gauge increase (`#TOTAL`).
    pub total: Option<f64>,
    /// Master volume percentage (`#VOLWAV`).
    pub vol_wav: Option<f64>,
    /// Long-note notation (`#LNTYPE`).
    pub ln_type: Option<LnType>,
    /// WAV index used as LN termination marker (`#LNOBJ`).
    pub ln_obj: Option<BmsChannelId<LnObjTag>>,
    /// Forced LN / CN / HCN mode (`#LNMODE`).
    pub ln_mode: Option<LnMode>,
    /// Numbering base for indexed commands (`#BASE`).
    pub base: Option<BmsBaseMode>,
    /// Per-position judgment width overrides (`#EXRANKxx`).
    pub ex_rank_defs: BTreeMap<BmsChannelId<ExRankTag>, f64>,
    /// Dynamic option-change definitions (`#CHANGEOPTIONxx`).
    pub change_option_defs: BTreeMap<BmsChannelId<ChangeOptionTag>, String>,
    /// Octave / Folding-point flag (`#OCT` / `#FP` / `#OCT/FP`).
    pub oct_fp: Option<bool>,
    /// Engine-specific option string (`#OPTION`).
    pub option: Option<String>,
}

impl Gameplay {
    /// Apply a gameplay header to this struct.
    pub fn apply(&mut self, header: &BmsHeaderGameplay<'_>) {
        match header {
            BmsHeaderGameplay::Player(m) => self.player = Some(*m),
            BmsHeaderGameplay::Rank(r) => self.rank = Some(*r),
            BmsHeaderGameplay::DefExRank(v) => self.def_ex_rank = Some(*v),
            BmsHeaderGameplay::ExRank { id, value } => {
                self.ex_rank_defs.insert(*id, *value);
            }
            BmsHeaderGameplay::Total(v) => self.total = Some(*v),
            BmsHeaderGameplay::VolWav(v) => self.vol_wav = Some(*v),
            BmsHeaderGameplay::LnType(t) => self.ln_type = Some(*t),
            BmsHeaderGameplay::LnObj(id) => self.ln_obj = Some(*id),
            BmsHeaderGameplay::LnMode(m) => self.ln_mode = Some(*m),
            BmsHeaderGameplay::Base(m) => self.base = Some(*m),
            BmsHeaderGameplay::OctFp => self.oct_fp = Some(true),
            BmsHeaderGameplay::Option(s) => self.option = Some((*s).to_owned()),
            BmsHeaderGameplay::ChangeOption { id, value } => {
                self.change_option_defs.insert(*id, (*value).to_owned());
            }
        }
    }
}
