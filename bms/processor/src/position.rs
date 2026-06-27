//! BMS Position → 绝对脉冲转换。
//!
//! BMS 在小节内使用分数位置。每个小节有一个可配置长度（相对标准 4/4 拍
//! 小节的比率）。本模块构建累计脉冲表，将 [`Position`] 转换为绝对脉冲。

use bms_parser::Position;

/// 将小节号映射到累计脉冲位置的表。
///
/// 由 `#XXX` 小节长度变更构建。未显式列出的小节默认为 `1.0`（标准 4/4 拍
/// = `resolution * 4` 脉冲）。
pub struct MeasureTable {
    /// 每个小节起点的累计脉冲（索引 = 小节号）。
    starts: Vec<u64>,
}

impl MeasureTable {
    /// 从 BMS 小节长度定义构建小节表。
    ///
    /// `resolution` 为每四分音符的脉冲数（如 240）。
    /// `measure_lengths` 为 `bms.messages.measure_lengths` 向量。
    ///
    /// 每个小节的脉冲长度为 `resolution * 4 * length_ratio`，其中
    /// `length_ratio` 为 BMS `#xxx02` 值（`1.0` = 4/4 拍）。
    pub(crate) fn new(
        max_measure: u16,
        measure_lengths: &[bms_parser::MeasureLength],
        resolution: u64,
    ) -> Self {
        let ticks_per_measure = resolution * 4;

        // 构建查找表：小节 → length_ratio（默认 1.0）。
        let mut ratios: std::collections::BTreeMap<u16, f64> = std::collections::BTreeMap::new();
        for ml in measure_lengths {
            ratios.insert(ml.measure, ml.length_ratio);
        }

        let mut starts = Vec::with_capacity(usize::from(max_measure) + 2);
        starts.push(0);

        let mut prev = 0u64;
        for m in 0..=max_measure {
            let ratio = ratios.get(&m).copied().unwrap_or(1.0);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "measure length in ticks fits in u64; ratio is non-negative by construction"
            )]
            let len = (ticks_per_measure as f64 * ratio) as u64;
            starts.push(prev + len);
            prev += len;
        }

        Self { starts }
    }

    /// 将 BMS [`Position`] 转换为绝对脉冲。
    ///
    /// 小节内位置占该小节脉冲长度的 `numer / denom`。
    pub(crate) fn position_to_tick(&self, pos: Position) -> u64 {
        let m = usize::from(pos.measure);
        let Some(&measure_start) = self.starts.get(m) else {
            return self.starts.last().copied().unwrap_or(0);
        };
        let measure_end = self.starts.get(m + 1).copied().unwrap_or(measure_start);
        let measure_len = measure_end - measure_start;

        if pos.denom == 0 {
            return measure_start;
        }

        measure_start + u64::from(pos.numer) * measure_len / u64::from(pos.denom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_parser::MeasureLength;

    const RES: u64 = 240;

    #[test]
    fn constant_meter_position_to_tick() {
        let table = MeasureTable::new(4, &[], RES);

        assert_eq!(table.position_to_tick(Position::new(0, 0, 8)), 0);
        assert_eq!(table.position_to_tick(Position::new(0, 4, 8)), 480);
        assert_eq!(table.position_to_tick(Position::new(0, 8, 8)), 960);
        assert_eq!(table.position_to_tick(Position::new(1, 0, 8)), 960);
    }

    #[test]
    fn variable_meter_preserves_measure_lengths() {
        // 小节 0 = 1.0（4/4 拍），小节 1 = 0.5（2/4 拍），小节 2 = 1.0（4/4 拍）。
        let lengths = vec![MeasureLength {
            measure: 1,
            length_ratio: 0.5,
        }];
        let table = MeasureTable::new(3, &lengths, RES);

        // 小节 0 = 960 脉冲，小节 1 = 480 脉冲，小节 2 = 960 脉冲。
        assert_eq!(table.position_to_tick(Position::new(0, 0, 1)), 0);
        assert_eq!(table.position_to_tick(Position::new(1, 0, 1)), 960);
        assert_eq!(table.position_to_tick(Position::new(2, 0, 1)), 1440);
        assert_eq!(table.position_to_tick(Position::new(3, 0, 1)), 2400);
    }

    #[test]
    fn fractional_position_within_short_measure() {
        let lengths = vec![MeasureLength {
            measure: 0,
            length_ratio: 0.5,
        }];
        let table = MeasureTable::new(1, &lengths, RES);

        // 小节 0 为 480 脉冲。位置 1/2 = 脉冲 240。
        assert_eq!(table.position_to_tick(Position::new(0, 1, 2)), 240);
    }

    #[test]
    fn three_four_time() {
        // 3/4 拍：length_ratio = 0.75 → 每小节 720 脉冲。
        let lengths = vec![MeasureLength {
            measure: 0,
            length_ratio: 0.75,
        }];
        let table = MeasureTable::new(2, &lengths, RES);

        // 小节 0 总计 720 脉冲。位置 3/4 表示进入 540 脉冲。
        assert_eq!(table.position_to_tick(Position::new(0, 3, 4)), 540);
        assert_eq!(table.position_to_tick(Position::new(0, 4, 4)), 720);
        assert_eq!(table.position_to_tick(Position::new(1, 0, 1)), 720);
    }
}
