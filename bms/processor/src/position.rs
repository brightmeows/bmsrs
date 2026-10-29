//! BMS position to absolute tick conversion.
//!
//! BMS uses fractional positions within a measure. Each measure has a
//! configurable length (a ratio relative to a standard 4/4 measure). This
//! module builds a cumulative tick table and converts [`Position`] to absolute
//! ticks.

use bmsrs_bms_parser::Position;

/// Table mapping measure numbers to cumulative tick positions.
///
/// Built from `#xxx02` measure length changes. Measures not listed explicitly
/// default to `1.0` (a standard 4/4 measure, or `resolution * 4` ticks).
pub struct MeasureTable {
    /// Cumulative tick at each measure start (index = measure number).
    starts: Vec<u64>,
}

impl MeasureTable {
    /// Build a measure table from BMS measure length definitions.
    ///
    /// `resolution` is the number of ticks per quarter note (such as 240).
    /// `measure_lengths` is the `bms.messages.measure_lengths` vector.
    ///
    /// Each measure's tick length is `resolution * 4 * length_ratio`, where
    /// `length_ratio` is the BMS `#xxx02` value (`1.0` means 4/4).
    pub(crate) fn new(
        max_measure: u16,
        measure_lengths: &[bmsrs_bms_parser::MeasureLength],
        resolution: u64,
    ) -> Self {
        let ticks_per_measure = resolution * 4;

        // Build the lookup table: measure to length_ratio (default 1.0).
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

    /// Return the cumulative ticks at every measure start (the barline
    /// positions).
    ///
    /// The returned slice has length `max_measure + 2` and is indexed by
    /// measure number. Item 0 is `0`; the last item is the end of the final
    /// measure (usable as the last barline). This supports variable time
    /// signatures: each bar tick is an actual barline position, not a fixed
    /// 4/4 spacing.
    #[must_use]
    pub(crate) fn bar_ticks(&self) -> &[u64] {
        &self.starts
    }

    /// Convert a BMS [`Position`] to an absolute tick.
    ///
    /// The position within the measure is `numer / denom` of that measure's tick
    /// length.
    ///
    /// Uses u128 intermediate arithmetic and rounds to nearest, avoiding u64
    /// overflow and reducing truncation error. The old implementation used
    /// integer division `numer * measure_len / denom`, which collapsed many
    /// positions to the same tick in short measures or with large denominators
    /// (for example, 1/8 in a 7-tick measure gave 0).
    ///
    /// `denom > 0` is guaranteed by [`Position::new`] at construction (the field
    /// is private and cannot be bypassed externally), so no defensive
    /// `denom == 0` check is needed here.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "offset fits in u64 (bounded by measure_len)"
    )]
    pub(crate) fn position_to_tick(&self, pos: Position) -> u64 {
        let m = usize::from(pos.measure());
        let Some(&measure_start) = self.starts.get(m) else {
            return self.starts.last().copied().unwrap_or(0);
        };
        let measure_end = self.starts.get(m + 1).copied().unwrap_or(measure_start);
        let measure_len = measure_end - measure_start;

        let n = u128::from(pos.numer()) * u128::from(measure_len);
        let d = u128::from(pos.denom());
        // Round to nearest: add d/2, then divide.
        let offset = ((n + d / 2) / d) as u64;
        measure_start + offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmsrs_bms_parser::MeasureLength;

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

        // Measure 0 is 480 ticks. Position 1/2 is tick 240.
        assert_eq!(table.position_to_tick(Position::new(0, 1, 2)), 240);
    }

    #[test]
    fn three_four_time() {
        // 3/4 time: length_ratio = 0.75 gives 720 ticks per measure.
        let lengths = vec![MeasureLength {
            measure: 0,
            length_ratio: 0.75,
        }];
        let table = MeasureTable::new(2, &lengths, RES);

        // Measure 0 totals 720 ticks. Position 3/4 lands at tick 540.
        assert_eq!(table.position_to_tick(Position::new(0, 3, 4)), 540);
        assert_eq!(table.position_to_tick(Position::new(0, 4, 4)), 720);
        assert_eq!(table.position_to_tick(Position::new(1, 0, 1)), 720);
    }

    #[test]
    fn short_measure_rounds_instead_of_truncates() {
        // Very short measure: ratio = 0.0078125, so 960 * 0.0078125 = 7.5,
        // truncated to 7.
        let lengths = vec![MeasureLength {
            measure: 0,
            length_ratio: 0.007_812_5,
        }];
        let table = MeasureTable::new(1, &lengths, RES);

        // measure_len = 7.
        // Position 1/2: exact 3.5 rounds to 4 (old truncation gave 3).
        assert_eq!(table.position_to_tick(Position::new(0, 1, 2)), 4);
        // Position 1/8: exact 0.875 rounds to 1 (old truncation gave 0,
        // colliding with the measure start).
        assert_eq!(table.position_to_tick(Position::new(0, 1, 8)), 1);
        // Position 1/192: exact 0.036 rounds to 0.
        assert_eq!(table.position_to_tick(Position::new(0, 1, 192)), 0);
    }

    #[test]
    fn exact_positions_unchanged_by_rounding() {
        // Exact divisions must not be affected by rounding.
        let table = MeasureTable::new(4, &[], RES);
        assert_eq!(table.position_to_tick(Position::new(0, 1, 3)), 320);
        assert_eq!(table.position_to_tick(Position::new(0, 1, 6)), 160);
        assert_eq!(table.position_to_tick(Position::new(0, 5, 6)), 800);
    }
}
