//! BMS（Be-Music Script）格式解析器。
//!
//! 本 crate 提供 BMS 解析管道的第二阶段：将 flat token 流（无控制流
//! 命令）转换为结构化的 [`Bms`] 对象，包含带类型的元数据、资源定义、
//! 计时与通道消息。

mod audio;
mod cst;
mod display;
mod gameplay;
mod messages;
mod metadata;
mod timing;
mod visual;
mod warnings;

// 重新导出子模块的全部公开类型。
pub use audio::{Audio, OwnedExWavParams};
pub use bmsrs_bms_values::*;
pub use cst::{
    CstParse, CstWarning, CstWarningKind, ErrorStrategy, LineToken, tokenize_lines,
    tokenize_lines_with,
};
pub use display::Display;
pub use gameplay::Gameplay;
pub use messages::{
    BgaEvent, BgmEvent, BmsBgaLayer, BpmChange, BpmValue, KeyType, LongNoteEvent, MeasureLength,
    Messages, MineEvent, NonEventData, NoteEvent, Position, ScrollEvent, SpeedEvent, StopEvent,
    StpEvent,
};
pub use metadata::Metadata;
pub use timing::Timing;
pub use visual::{OwnedExBmpParams, OwnedSwBgaParams, Visual};
pub use warnings::ChannelValueWarning;

// Bms —— 根文档模型

/// 由 flat token 流构建的 BMS 文件结构化表示。
///
/// 每个子结构体恰好对应一个分词器头部分组，使 `process_header` 方法
/// 能直接路由到子模块的 `apply` 方法。唯一例外是 `#STP`（Timing →
/// Messages），它跨越了子结构体边界，因为它使用与通道事件相同的位置
/// 模型。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bms {
    /// 乐曲 / 谱面识别元数据（← `BmsHeaderMetadata`）。
    pub metadata: metadata::Metadata,
    /// 游玩行为设置（← `BmsHeaderGameplay`）。
    pub gameplay: gameplay::Gameplay,
    /// 计时定义（← `BmsHeaderTiming`）。
    pub timing: timing::Timing,
    /// 显示资源 / 难度标记（← `BmsHeaderDisplay`）。
    pub display: display::Display,
    /// 音频资源定义（← `BmsHeaderResDefAudio`）。
    pub audio: audio::Audio,
    /// 视觉 / BGA 资源定义（← `BmsHeaderResDefVisual`）。
    pub visual: visual::Visual,
    /// 通道消息数据（原始数据 + 已解析事件）。
    pub messages: messages::Messages,
    /// 未识别 / 引擎特有的头部命令。
    pub fallback_headers: Vec<(String, String)>,
}

impl Bms {
    /// 由 flat token 流（无控制流命令）构建 `Bms`。
    ///
    /// 两阶段处理：
    /// 1. 预扫描定位 `#BASE` 以确定进制基数。
    /// 2. 随后所有 token 以正确的基数处理，将索引键（`WavIndex`、
    ///    `BmpIndex` 等）归一化，以便在标准（Base36）BMS 文件中进行
    ///    不区分大小写的比较。
    pub fn from_flat_tokens(tokens: impl IntoIterator<Item = BmsToken>) -> Self {
        let mut bms = Self::default();

        // 预扫描 #BASE。先收集 token 迭代器，因为需要迭代两次
        //（一次用于 BASE，一次用于处理）。
        let all_tokens: Vec<_> = tokens.into_iter().collect();
        let bms_base = detect_base(&all_tokens);

        for token in &all_tokens {
            match token {
                BmsToken::Header(header) => bms.process_header(header, bms_base),
                BmsToken::Message(msg) => bms.process_message(msg),
            }
        }

        bms.messages.finalize(bms_base);
        bms
    }

    // 头部分发 —— 纯路由到子模块的 apply() 方法

    /// 将头部命令路由到对应子模块的 `apply` 方法。
    ///
    /// `base` 是由 `#BASE` 确定的进制基数（默认为 [`BmsBase::Base36`]）。
    fn process_header(&mut self, header: &BmsHeader, base: BmsBase) {
        match header {
            BmsHeader::Metadata(m) => self.metadata.apply(m),
            BmsHeader::Gameplay(g) => self.gameplay.apply(g, base),
            BmsHeader::Timing(t) => {
                self.timing.apply(t, base);
                // #STP 跨越子结构体边界
                if let BmsHeaderTiming::Stp { params } = t {
                    self.messages.stp_events.push(messages::StpEvent {
                        position: messages::Position::new(
                            params.measure,
                            u32::from(params.position),
                            1000,
                        ),
                        duration_ms: params.duration_ms,
                    });
                }
            }
            BmsHeader::ResDefAudio(a) => self.audio.apply(a, base),
            BmsHeader::ResDefVisual(v) => self.visual.apply(v, base),
            BmsHeader::Display(d) => self.display.apply(d),
            BmsHeader::ControlFlow(_) => { /* skipped — not stored in Bms */ }
            BmsHeader::Fallback(f) => self
                .fallback_headers
                .push((f.command.clone(), f.value.clone())),
        }
    }

    // 消息

    /// 将通道消息插入到消息容器中。
    fn process_message(&mut self, m: &BmsMessage) {
        self.messages.concat_raw(m);
    }
}

/// 预扫描 token 以查找 `#BASE`，确定进制基数。
///
/// 当未找到 `#BASE` 头部命令时，默认为 [`BmsBase::Base36`]。
fn detect_base(tokens: &[BmsToken]) -> BmsBase {
    tokens
        .iter()
        .find_map(|token| {
            if let BmsToken::Header(BmsHeader::Gameplay(BmsHeaderGameplay::Base(b))) = token {
                Some(*b)
            } else {
                None
            }
        })
        .unwrap_or(BmsBase::Base36)
}
