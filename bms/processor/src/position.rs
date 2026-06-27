//! BMS Position → absolute tick conversion.
//!
//! BMS uses fractional positions within measures. Each measure has a
//! configurable length (ratio of a standard 4/4 measure). This module builds
//! a cumulative tick table to convert [`Position`] to absolute ticks.

use bms_parser::Position;

/// Table mapping measure numbers to cumulative tick positions.
///
/// Built from `#XXX` measure-length changes. Measures not explicitly listed
/// default to `1.0` (standard 4/4 = `resolution * 4` ticks).
pub struct MeasureTable {
    /// Cumulative tick at the start of each measure (index = measure number).
    starts: Vec<u64>,
}

impl MeasureTable {
    /// Build a measure table from BMS measure-length definitions.
    ///
    /// `resolution` is ticks per quarter note (e.g. 240).
    /// `measure_lengths` is the `bms.messages.measure_lengths` vector.
    ///
    /// Each measure's length in ticks is `resolution * 4 * length_ratio`,
    /// where `length_ratio` is the BMS `#xxx02` value (`1.0` = 4/4).
    pub(crate) fn new(
        max_measure: u16,
        measure_lengths: &[bms_parser::MeasureLength],
        resolution: u64,
    ) -> Self {
        let ticks_per_measure = resolution * 4;

        // Build a lookup: measure → length_ratio (default 1.0).
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

    /// Convert a BMS [`Position`] to an absolute tick.
    ///
    /// The position within a measure is `numer / denom` of that measure's
    /// tick length.
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
        // Measure 0 = 1.0 (4/4), measure 1 = 0.5 (2/4), measure 2 = 1.0 (4/4).
        let lengths = vec![MeasureLength {
            measure: 1,
            length_ratio: 0.5,
        }];
        let table = MeasureTable::new(3, &lengths, RES);

        // Measure 0 = 960 ticks, measure 1 = 480 ticks, measure 2 = 960 ticks.
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

        // Measure 0 is 480 ticks. Position 1/2 = tick 240.
        assert_eq!(table.position_to_tick(Position::new(0, 1, 2)), 240);
    }

    #[test]
    fn three_four_time() {
        // 3/4 time: length_ratio = 0.75 → 720 ticks per measure.
        let lengths = vec![MeasureLength {
            measure: 0,
            length_ratio: 0.75,
        }];
        let table = MeasureTable::new(2, &lengths, RES);

        // Measure 0 = 720 ticks total. Position 3/4 means 540 ticks in.
        assert_eq!(table.position_to_tick(Position::new(0, 3, 4)), 540);
        assert_eq!(table.position_to_tick(Position::new(0, 4, 4)), 720);
        assert_eq!(table.position_to_tick(Position::new(1, 0, 1)), 720);
    }
}
