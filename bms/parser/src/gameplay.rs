//! 游玩行为字段。
//!
//! 对应分词器的 [`BmsHeaderGameplay`]。

use std::collections::BTreeMap;

use bms_tokenizer::{
    BmsBase, BmsHeaderGameplay, ChangeOptionIndex, ExRankIndex, LnMode, LnObjIndex, LnType,
    PlayerMode, Rank,
};

/// 游玩行为设置。
///
/// 标量字段使用最后胜出语义；索引字段使用 `BTreeMap`（按 ID 最后
/// 胜出）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gameplay {
    /// 游戏模式（`#PLAYER`）。
    pub player: Option<PlayerMode>,
    /// 判定难度（`#RANK`）。
    pub rank: Option<Rank>,
    /// 以百分比表示的精细判定难度（`#DEFEXRANK`）。
    pub def_ex_rank: Option<f64>,
    /// 血量槽最大增量（`#TOTAL`）。
    pub total: Option<f64>,
    /// 主音量百分比（`#VOLWAV`）。
    pub vol_wav: Option<f64>,
    /// 长音记法（`#LNTYPE`）。
    pub ln_type: Option<LnType>,
    /// 用作长音终止标记的 WAV 索引（`#LNOBJ`）。
    pub ln_obj: Option<LnObjIndex>,
    /// 强制 LN / CN / HCN 模式（`#LNMODE`）。
    pub ln_mode: Option<LnMode>,
    /// 索引命令的进制基数（`#BASE`）。
    pub base: Option<BmsBase>,
    /// 按位置的判定宽度覆盖（`#EXRANKxx`）。
    pub ex_rank_defs: BTreeMap<ExRankIndex, f64>,
    /// 动态选项变更定义（`#CHANGEOPTIONxx`）。
    pub change_option_defs: BTreeMap<ChangeOptionIndex, String>,
    /// 八度 / 折叠点标志（`#OCT` / `#FP` / `#OCT/FP`）。
    pub oct_fp: Option<bool>,
    /// 引擎特有的选项字符串（`#OPTION`）。
    pub option: Option<String>,
}

impl Gameplay {
    /// 将一个游玩头部命令应用到此结构体。
    ///
    /// 索引键（[`ExRankIndex`]、[`ChangeOptionIndex`]）使用 `base` 归一化，
    /// 以便在标准 BMS 中进行不区分大小写的比较——与
    /// [`Timing`](crate::timing::Timing) / [`Visual`](crate::visual::Visual)
    /// 的归一化契约保持一致。
    pub fn apply<C: AsRef<str>>(&mut self, header: &BmsHeaderGameplay<C>, base: BmsBase) {
        match header {
            BmsHeaderGameplay::Player(m) => self.player = Some(*m),
            BmsHeaderGameplay::Rank(r) => self.rank = Some(*r),
            BmsHeaderGameplay::DefExRank(v) => self.def_ex_rank = Some(*v),
            BmsHeaderGameplay::ExRank { id, value } => {
                self.ex_rank_defs
                    .insert(ExRankIndex::from(id.normalize(base)), *value);
            }
            BmsHeaderGameplay::Total(v) => self.total = Some(*v),
            BmsHeaderGameplay::VolWav(v) => self.vol_wav = Some(*v),
            BmsHeaderGameplay::LnType(t) => self.ln_type = Some(*t),
            BmsHeaderGameplay::LnObj(id) => self.ln_obj = Some(*id),
            BmsHeaderGameplay::LnMode(m) => self.ln_mode = Some(*m),
            BmsHeaderGameplay::Base(m) => self.base = Some(*m),
            BmsHeaderGameplay::OctFp => self.oct_fp = Some(true),
            BmsHeaderGameplay::Option(s) => self.option = Some(s.as_ref().to_owned()),
            BmsHeaderGameplay::ChangeOption { id, value } => {
                self.change_option_defs.insert(
                    ChangeOptionIndex::from(id.normalize(base)),
                    value.as_ref().to_owned(),
                );
            }
        }
    }
}
