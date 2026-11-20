//! Standard random sources for control-flow branch selection.
//!
//! Two players define the de-facto layouts:
//!
//! - **beatoraja** draws each `#RANDOM` / `#SWITCH` value from
//!   `java.util.Random` seeded with the requested seed; the facade crate's
//!   `JavaRandom` reproduces its bit stream exactly.
//! - **LR2** draws from `DxLib`'s MT19937, whose seeding differs from the
//!   standard `init_genrand`: `srandMT` fills the state through a 69069 LCG
//!   taking the high 16 bits of each step. [`Lr2Mt19937`] reproduces the
//!   stream, the `GetRand` scaling, and the lane shuffle that LR2 applies to
//!   RANDOM options.
//!
//! The facade crate's `JavaRandom` implements [`ControlFlowRng`] in place;
//! both then share the draw interface accepted by
//! [`SourceFile::select_lines_with_rng`](crate::SourceFile::select_lines_with_rng).
//! Because the caller owns the generator, the remaining state stays available
//! after branch selection; LR2 needs exactly that: `#RANDOM` values are drawn
//! first, then the RANDOM lane option continues from the same stream.

/// A source of control-flow values.
pub trait ControlFlowRng {
    /// Draw one value in `[1, max]`. `max` is at least 1.
    fn draw(&mut self, max: u64) -> u64;
}

/// Number of `u32` words in the MT19937 state.
const N: usize = 624;
/// Middle word offset of the twist.
const M: usize = 397;
/// Twist matrix coefficient.
const MATRIX_A: u32 = 0x9908_B0DF;
/// Top bit mask.
const UPPER_MASK: u32 = 0x8000_0000;
/// Remaining 31 bits mask.
const LOWER_MASK: u32 = 0x7FFF_FFFF;

/// The largest seed LR2 records in its rankings.
pub const LR2_SEED_MAX: u32 = 0x7FFE;

/// `DxLib`'s MT19937 as LR2 uses it.
#[derive(Debug, Clone)]
pub struct Lr2Mt19937 {
    /// Generator state.
    state: Box<[u32; N]>,
    /// Next unread index; `N` means the twist runs first.
    index: usize,
}

impl Lr2Mt19937 {
    /// Build the state the way `DxLib`'s `srandMT(seed)` does.
    ///
    /// `DxLib` seeds through a 69069 LCG, storing the high 16 bits of two
    /// successive steps in each word. One twist then runs immediately.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        let mut state = Box::new([0u32; N]);
        let mut step = seed;
        for word in &mut state {
            *word = step & 0xFFFF_0000;
            step = step.wrapping_mul(69_069).wrapping_add(1);
            *word |= (step & 0xFFFF_0000) >> 16;
            step = step.wrapping_mul(69_069).wrapping_add(1);
        }
        let mut generator = Self { state, index: N };
        generator.twist();
        generator
    }

    /// Run one twist over the whole state.
    #[expect(
        clippy::indexing_slicing,
        reason = "MT19937 indexing is modulo a compile-time array length, so every index is in bounds"
    )]
    fn twist(&mut self) {
        let state = &mut self.state;
        for k in 0..N {
            let y = (state[k] & UPPER_MASK) | (state[(k + 1) % N] & LOWER_MASK);
            let magic = if y & 1 == 0 { 0 } else { MATRIX_A };
            state[k] = state[(k + M) % N] ^ (y >> 1) ^ magic;
        }
        self.index = 0;
    }

    /// Standard tempering output.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "index is checked against N before use, so it is in bounds"
    )]
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C_5680;
        y ^= (y << 15) & 0xEFC6_0000;
        y ^= y >> 18;
        y
    }

    /// `DxLib`'s `GetRand(max)`: a value in `[0, max]` scaled by the top 32 bits
    /// of a product rather than by a remainder.
    ///
    /// Exactly one word is consumed, even when `max` is negative (the case of
    /// `GetRand(-1)` from `#RANDOM 0`), which then yields 0.
    #[must_use]
    pub fn get_rand(&mut self, max: i64) -> i64 {
        let value = u64::from(self.next_u32());
        let range = u64::try_from(max.saturating_add(1)).unwrap_or(0);
        i64::try_from((u128::from(value) * u128::from(range)) >> 32).unwrap_or(0)
    }

    /// Build LR2's RANDOM lane table for `keys` keys.
    ///
    /// `table[original]` is the destination key, both one-based; `table[0]` is
    /// always 0. LR2 walks `c = 1..keys` and swaps `c` with
    /// `c + GetRand(keys - c)`. Scratch is never shuffled: LR2 rankings do not
    /// record that choice.
    #[must_use]
    pub fn shuffle_key_table(&mut self, keys: usize) -> Vec<usize> {
        let mut table: Vec<usize> = (0..=keys).collect();
        for c in 1..keys {
            let span = i64::try_from(keys - c).unwrap_or(i64::MAX);
            let offset = usize::try_from(self.get_rand(span)).unwrap_or(0);
            let a = c + offset;
            table.swap(c, a);
        }
        table
    }
}

impl ControlFlowRng for Lr2Mt19937 {
    fn draw(&mut self, max: u64) -> u64 {
        let bound = i64::try_from(max).unwrap_or(i64::MAX);
        u64::try_from(self.get_rand(bound - 1) + 1).unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lr2_matches_the_dxlib_stream() {
        let mut generator = Lr2Mt19937::new(2179);
        assert_eq!(
            [
                generator.next_u32(),
                generator.next_u32(),
                generator.next_u32()
            ],
            [88_460_912, 2_752_084_135, 2_394_320_606]
        );
    }

    #[test]
    fn lr2_get_rand_scales_by_the_high_product_bits() {
        let mut generator = Lr2Mt19937::new(2179);
        assert_eq!(generator.get_rand(1), 0);
        assert_eq!(generator.get_rand(3), 2);
        assert_eq!(generator.get_rand(255), 142);
    }

    #[test]
    fn lr2_negative_bound_consumes_once_and_returns_zero() {
        let mut probe = Lr2Mt19937::new(391);
        let mut generator = Lr2Mt19937::new(391);
        assert_eq!(generator.get_rand(-1), 0);
        // One word was consumed, so the two states line up after one step.
        assert_ne!(probe.next_u32(), 0);
        assert_eq!(generator.next_u32(), probe.next_u32());
    }

    #[test]
    fn lr2_shuffle_matches_the_recorded_lane_table() {
        let mut generator = Lr2Mt19937::new(2179);
        assert_eq!(generator.shuffle_key_table(7), [0, 1, 5, 2, 6, 3, 7, 4]);
    }

    #[test]
    fn lr2_draw_is_one_based() {
        let mut generator = Lr2Mt19937::new(391);
        assert_eq!([generator.draw(1), generator.draw(3)], [1, 1]);
    }
}
