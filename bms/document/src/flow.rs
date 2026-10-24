//! 行导向的控制流视图。

use std::num::NonZeroUsize;

use bms_control_flow::FlowPayload;

/// 控制流树中的一段连续行。
///
/// 区间两端都是 1 起、含端点的行号。空白、注释与错误行也会落在某个
/// 区间内，使载荷区间覆盖原文的连续行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRef {
    /// 起始行号（1 起，含）。
    pub start: NonZeroUsize,
    /// 结束行号（1 起，含）。
    pub end: NonZeroUsize,
}

impl LineRef {
    /// 构造一段行区间。
    #[must_use]
    pub const fn new(start: NonZeroUsize, end: NonZeroUsize) -> Self {
        Self { start, end }
    }

    /// 返回区间包含的行数。
    #[must_use]
    pub const fn line_count(self) -> usize {
        self.end
            .get()
            .saturating_sub(self.start.get())
            .saturating_add(1)
    }
}

impl FlowPayload for LineRef {
    /// 行条目本身不携带数据，行号已经由构建器给出。
    type Item = ();

    fn from_run(run: Vec<(NonZeroUsize, ())>) -> Self {
        let start = run.first().map_or(NonZeroUsize::MIN, |(line, ())| *line);
        let end = run.last().map_or(start, |(line, ())| *line);
        Self { start, end }
    }
}
