//! RNG abstraction for branch selection.

/// A random number generator that produces values in `[1, max]`.
///
/// Implementors can inject deterministic or truly random sources.
pub trait BranchRng {
    /// Generate a random value in `[1, max]` (inclusive).
    ///
    /// # Panics
    ///
    /// May panic if `max` is zero.
    fn gen_range(&mut self, max: u64) -> u64;
}

/// A simple linear-congruential generator for deterministic testing.
///
/// Uses the constants from PCG / Numerical Recipes for reasonable
/// distribution. Not cryptographically secure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicRng {
    /// Internal LCG state.
    state: u64,
}

impl DeterministicRng {
    /// Create a new RNG with the given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl BranchRng for DeterministicRng {
    fn gen_range(&mut self, max: u64) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.state >> 33) % max) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_rng_produces_known_sequence() {
        let mut rng = DeterministicRng::new(42);
        let a = rng.gen_range(10);
        let b = rng.gen_range(10);
        // Verify same seed produces same sequence
        let mut rng2 = DeterministicRng::new(42);
        assert_eq!(a, rng2.gen_range(10));
        assert_eq!(b, rng2.gen_range(10));
    }

    #[test]
    fn deterministic_rng_stays_in_range() {
        let mut rng = DeterministicRng::new(123);
        for _ in 0..100 {
            let v = rng.gen_range(5);
            assert!((1..=5).contains(&v), "value {v} out of range [1, 5]");
        }
    }
}
