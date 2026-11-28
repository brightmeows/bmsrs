//! Shared position construction for processor integration tests.

use std::num::NonZeroU32;

use bmsrs_bms_model::Position;

/// Build a position from a `u32` denominator known to be non-zero.
pub fn pos(measure: u16, numer: u32, denom: u32) -> Position {
    let denom_nonzero = NonZeroU32::new(denom)
        .unwrap_or_else(|| panic!("test position denominator must be non-zero"));
    Position::new(measure, numer, denom_nonzero)
}
