//! RNG abstraction for branch selection.

use rand::RngExt;

/// A random number generator that produces values in `[1, max]`.
///
/// This trait encapsulates the BMS-spec semantics of `#RANDOM N` (uniform
/// selection from `1..=N`). Any [`rand::RngExt`] automatically satisfies this
/// via the blanket impl below, so callers can pass in `StdRng`, `ThreadRng`,
/// or any other `rand` generator directly.
pub trait BranchRng {
    /// Generate a random value in `[1, max]` (inclusive).
    ///
    /// # Panics
    ///
    /// May panic if `max` is zero (empty range).
    fn gen_range(&mut self, max: u64) -> u64;
}

/// Any [`rand::RngExt`] is a [`BranchRng`]: delegates to
/// [`rand::RngExt::random_range`] over the inclusive range `1..=max`.
impl<R: RngExt + ?Sized> BranchRng for R {
    fn gen_range(&mut self, max: u64) -> u64 {
        self.random_range(1..=max)
    }
}
