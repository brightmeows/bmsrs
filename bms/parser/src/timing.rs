//! 计时定义与全局标量。
//!
//! 对应分词器的 [`BmsHeaderTiming`]。基于位置（position-based）的计时
//! 事件（BPM 变更、停止、滚动……）存放在
//! [`Messages`](crate::messages::Messages) 中，与通道事件并列，以共享
//! 相同的位置模型。

use std::collections::BTreeMap;

use bms_tokenizer::{BmsBase, BmsHeaderTiming, BpmIndex, ScrollIndex, SpeedIndex, StopIndex};

/// 计时定义与全局标量。
///
/// 标量字段（`bpm`、`base_bpm`）使用最后胜出语义；索引定义
///（`bpm_defs`、`stop_defs`……）使用 `BTreeMap`。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timing {
    /// 全局初始 BPM（`#BPM`）。
    pub bpm: Option<f64>,
    /// 用于自动 HI-SPEED 的参考 BPM（`#BASEBPM`）。
    pub base_bpm: Option<f64>,
    /// 扩展 BPM 定义（`#BPMxx`、`#EXBPMxx`）。
    pub bpm_defs: BTreeMap<BpmIndex, f64>,
    /// 停止序列定义（`#STOPxx`）。
    pub stop_defs: BTreeMap<StopIndex, f64>,
    /// 滚动速度倍率定义（`#SCROLLxx`）。
    pub scroll_defs: BTreeMap<ScrollIndex, f64>,
    /// 视觉音符间距定义（`#SPEEDxx`）。
    pub speed_defs: BTreeMap<SpeedIndex, f64>,
}

impl Timing {
    /// 将一个计时头部命令应用到此结构体。
    ///
    /// 索引键（`BpmIndex`、`StopIndex` 等）使用 `base` 归一化，以便在
    /// 标准 BMS 中进行不区分大小写的比较。
    pub fn apply(&mut self, header: &BmsHeaderTiming, base: BmsBase) {
        match header {
            BmsHeaderTiming::Bpm(v) => self.bpm = Some(*v),
            BmsHeaderTiming::BpmDef { id, value } | BmsHeaderTiming::ExBpm { id, value } => {
                let nid = BpmIndex::from(id.normalize(base));
                self.bpm_defs.insert(nid, *value);
            }
            BmsHeaderTiming::BaseBpm(v) => self.base_bpm = Some(*v),
            BmsHeaderTiming::StopDef { id, value } => {
                let nid = StopIndex::from(id.normalize(base));
                self.stop_defs.insert(nid, *value);
            }
            BmsHeaderTiming::ScrollDef { id, value } => {
                let nid = ScrollIndex::from(id.normalize(base));
                self.scroll_defs.insert(nid, *value);
            }
            BmsHeaderTiming::SpeedDef { id, value } => {
                let nid = SpeedIndex::from(id.normalize(base));
                self.speed_defs.insert(nid, *value);
            }
            BmsHeaderTiming::Stp { .. } => {
                // `#STP` 是基于位置的停止；它存储在
                // `Messages::stp_events` 中而非此处，因为它使用与
                // 通道事件相同的位置模型。
                // 调用方负责将其路由到那里。
            }
        }
    }
}
