//! BMS 通道正文的 2 字符单元切分。
//!
//! BMS 通道正文是一串 2 字符值。解析采用宽松策略：连续两个合法
//! Base62 字符组成一个单元，非法字符被跳过，末尾单字符被丢弃。
//! parser、writer 与 analysis 共用本模块，避免各自的切块实现在
//! 畸形正文上产生不同的位置模型。

use std::ops::Range;

use crate::index::is_base62;

/// 正文中的一个 2 字符单元。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell<'a> {
    /// 单元在完整正文网格中的序号（包含被 [`BodyCells::non_zero`]
    /// 跳过的 `"00"`）。
    pub index: usize,
    /// 单元在正文中的字节区间。
    pub range: Range<usize>,
    /// 单元的 2 字符文本。
    pub text: &'a str,
}

/// 按宽松规则切分 BMS 通道正文的迭代器。
///
/// 由 [`BodyCells::including_zero`] 或 [`BodyCells::non_zero`] 构造。构造时扫描
/// 一次以确定完整单元数，随后迭代不分配。
#[derive(Clone, Debug)]
pub struct BodyCells<'a> {
    /// 待切分的正文。
    body: &'a str,
    /// 是否保留 `"00"` 单元。
    include_zero: bool,
    /// 完整正文的单元数。
    total: usize,
    /// 下一个待扫描的字节位置。
    pos: usize,
    /// 下一个单元的完整网格序号。
    index: usize,
}

impl<'a> BodyCells<'a> {
    /// 切分全部单元，包含 `"00"`。
    #[must_use]
    pub fn including_zero(body: &'a str) -> Self {
        Self::new(body, true)
    }

    /// 切分非 `"00"` 单元；序号仍按完整网格计算。
    #[must_use]
    pub fn non_zero(body: &'a str) -> Self {
        Self::new(body, false)
    }

    /// 使用显式的 `"00"` 策略构造。
    fn new(body: &'a str, include_zero: bool) -> Self {
        Self {
            body,
            include_zero,
            total: count_cells(body),
            pos: 0,
            index: 0,
        }
    }

    /// 返回完整正文的单元数（包含 `"00"`，不受 `non_zero` 影响）。
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }

    /// 返回底层正文。
    #[must_use]
    pub const fn body(&self) -> &'a str {
        self.body
    }
}

impl<'a> Iterator for BodyCells<'a> {
    type Item = Cell<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.body.as_bytes();
        while let (Some(&first), Some(&second)) = (bytes.get(self.pos), bytes.get(self.pos + 1)) {
            if is_base62(first) && is_base62(second) {
                let range = self.pos..self.pos + 2;
                self.pos += 2;
                let index = self.index;
                self.index += 1;
                let text = self.body.get(range.clone()).unwrap_or_default();
                if !self.include_zero && text == "00" {
                    continue;
                }
                return Some(Cell { index, range, text });
            }
            self.pos += 1;
        }
        None
    }
}

/// 统计正文中的完整 2 字符单元数。
fn count_cells(body: &str) -> usize {
    let bytes = body.as_bytes();
    let mut count = 0;
    let mut pos = 0;
    while let (Some(&first), Some(&second)) = (bytes.get(pos), bytes.get(pos + 1)) {
        if is_base62(first) && is_base62(second) {
            count += 1;
            pos += 2;
        } else {
            pos += 1;
        }
    }
    count
}
