//! Java `java.util.Random` replication for reproducible BMS branch layout.
//!
//! `beatoraja` and `LR2oraja` drive `#RANDOM` / `#SWITCH` resolution through
//! `java.util.Random`; reproducing their exact branch layout requires the same
//! number sequence. This mirrors the documented 48-bit linear congruential
//! generator, including the `nextInt(bound)` rejection loop.

use crate::ControlFlowRng;

/// Scrambling multiplier of the 48-bit state.
const MULT: u64 = 0x5_DEEC_E66D;
/// Addend of the 48-bit state.
const ADD: u64 = 0xB;
/// Low 48-bit state mask.
const MASK: u64 = (1 << 48) - 1;

/// A replication of `java.util.Random` for BMS branch selection.
///
/// It implements [`ControlFlowRng`], so it plugs into
/// [`SourceFile::select_lines`](crate::SourceFile::select_lines) directly:
///
/// ```
/// use bmsrs_bms_syntax::{ControlFlowCompat, JavaRandom, SourceFile};
///
/// let file =
///     SourceFile::parse("#RANDOM 2\n#IF 1\n#TITLE from-one\n#ENDIF\n").source_file();
/// let mut rng = JavaRandom::new(123_456_789);
/// let (_lines, selection) = file.select_lines(&mut rng, ControlFlowCompat::Ideal);
/// assert_eq!(selection.decisions().len(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JavaRandom {
    /// Scrambled 48-bit internal state.
    seed: u64,
}

impl JavaRandom {
    /// Create a generator from the given seed, scrambling it the way
    /// `java.util.Random` does.
    #[must_use]
    pub const fn new(seed: i64) -> Self {
        let bits = seed.cast_unsigned();
        Self {
            seed: (bits ^ MULT) & MASK,
        }
    }

    /// Java `next(bits)`: the top `bits` of the scrambled state.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "java.util.Random.next wraps the top bits to i32 by specification"
    )]
    const fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULT).wrapping_add(ADD)) & MASK;
        ((self.seed >> (48 - bits)) & ((1 << bits) - 1)) as i32
    }

    /// Java `nextInt()`: any `i32` value.
    #[must_use]
    pub const fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Java `nextInt(bound)`: a value in `[0, bound)`.
    ///
    /// # Panics
    ///
    /// Panics when `bound` is not positive.
    #[must_use]
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let m = bound - 1;
        if (bound & m) == 0 {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the product is within [0, bound) by construction"
            )]
            {
                ((i64::from(bound) * i64::from(self.next(31))) >> 31) as i32
            }
        } else {
            loop {
                let bits = self.next(31);
                let val = bits % bound;
                if bits - val + m >= 0 {
                    return val;
                }
            }
        }
    }
}

impl ControlFlowRng for JavaRandom {
    /// Bounds up to [`i32::MAX`] mirror `java.util.Random.nextInt` exactly;
    /// larger bounds (never produced by real charts) fall back to rejection
    /// sampling over 53 drawn bits.
    ///
    /// # Panics
    ///
    /// Panics when `max` is zero.
    fn draw(&mut self, max: u64) -> u64 {
        assert!(max >= 1, "max must be positive");
        if let Ok(bound) = i32::try_from(max) {
            #[expect(
                clippy::cast_sign_loss,
                reason = "nextInt(bound) returns a non-negative value for positive bounds"
            )]
            let offset = self.next_int_bound(bound) as u64;
            offset + 1
        } else {
            loop {
                #[expect(
                    clippy::cast_sign_loss,
                    reason = "java next(bits) bits are reinterpreted as unsigned"
                )]
                let bits =
                    (u64::from(self.next(31) as u32) << 22) | u64::from(self.next(22) as u32);
                let val = bits % max;
                if bits - val <= u64::MAX - (max - 1) {
                    return val + 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_int_matches_java_reference_values() {
        let next_for = |seed: i64| {
            let mut generator = JavaRandom::new(seed);
            generator.next_int()
        };
        assert_eq!(next_for(0), -1_155_484_576);
        assert_eq!(next_for(1), -1_155_869_325);
        assert_eq!(next_for(42), -1_170_105_035);
        assert_eq!(next_for(123_456_789), -1_442_945_365);
        assert_eq!(next_for(-42), 1_170_874_531);
    }

    #[test]
    fn next_int_bound_matches_java_reference_values() {
        let bound_values_for = |seed: i64, bound: i32, count: usize| {
            let mut generator = JavaRandom::new(seed);
            (0..count)
                .map(|_| generator.next_int_bound(bound))
                .collect::<Vec<i32>>()
        };
        assert_eq!(bound_values_for(0, 10, 5), [0, 8, 9, 7, 5]);
        assert_eq!(bound_values_for(123_456_789, 10, 5), [5, 0, 3, 4, 0]);
        assert_eq!(
            bound_values_for(123_456_789, 1024, 6),
            [679, 781, 467, 260, 399, 221]
        );
        assert_eq!(bound_values_for(7, 1, 3), [0, 0, 0]);
    }
}
