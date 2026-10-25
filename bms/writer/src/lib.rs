//! `Bms` 模型 → BMS 文本的全量重写生成器。
//!
//! [`BmsWriter`] 将结构化的 [`Bms`] 模型重新生成为
//! BMS 文本。生成是**规范化**的：头部按域分组输出，通道行按
//! （小节，通道）排序合并——注释、原始行格式与空行不保留。
//! 保真行级编辑请配合 `bms-document` 的行编辑能力使用，本
//! crate 服务于结构性大改后的兜底重写。
//!
//! # 管道
//!
//! ```text
//! bms_parser::Bms → BmsWriter::write → String（BMS 文本）
//! ```

pub mod channel;
mod headers;
mod messages;

use bms_parser::Bms;

/// BMS 文本生成器。
///
/// 零大小类型，全部功能通过关联函数 [`BmsWriter::write`] 暴露。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BmsWriter;

impl BmsWriter {
    /// 将 [`Bms`] 模型生成为 BMS 文本。
    ///
    /// 输出布局：
    /// 1. 头部命令，按 元数据 → 游玩 → 计时 → 显示 → 音频 → 视觉 →
    ///    未识别头（原序）分组
    /// 2. 通道行，按（小节，通道）升序；同一（小节，通道）的
    ///    多行按位置合并为一行（非 `00` 覆盖，`00` 保留位置），
    ///    与 [`bms_parser`] 的解析合并语义一致
    /// 3. 以换行符结尾
    ///
    /// 输出为 Unicode 文本；字符编码转换（Shift-JIS 等）由调用方
    /// 负责。
    #[must_use]
    pub fn write(bms: &Bms) -> String {
        let mut lines = headers::write_headers(bms);
        lines.extend(messages::write_messages(&bms.messages));
        let mut text = lines.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        text
    }
}
